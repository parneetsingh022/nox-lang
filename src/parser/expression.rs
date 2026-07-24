use crate::diagnostic::{ExpectedExpressionError, ParserError, Span, UnexpectedEofError};
use crate::parser::Parser;

use crate::parser::ast::UnaryOp;
use crate::{
    lexer::{Symbol, TokenKind},
    parser::ast::{BinaryOp, Expr, ExprKind},
};

/// Determines whether a given [`TokenKind`] represents a valid unary (prefix) operator.
///
/// The parser uses this check to identify tokens that can legitimately start
/// an expression during the null denotation (`nud`) step.
fn is_unary_operator(token_kind: TokenKind) -> bool {
    matches!(token_kind, TokenKind::Minus | TokenKind::Bang)
}

impl<'a> Parser<'a> {
    fn is_expr_start(&self) -> bool {
        let Some(current) = self.peek().map(|tok| tok.kind) else {
            return false;
        };

        if is_unary_operator(current) {
            return true;
        }

        matches!(
            current,
            TokenKind::Identifier(_)
                | TokenKind::IntLiteral(_)
                | TokenKind::FloatLiteral(_)
                | TokenKind::OpenParen
        )
    }

    fn parse_integer_literal(&self, symbol: Symbol, span: Span) -> Expr {
        // It is okay to panic here, because this is not a user error. If the lexer
        // works as intended, it will not lex any invalid IntLiteral.
        let value = self
            .symbol_registry
            .resolve(symbol)
            .parse::<i64>()
            .expect("Lexer produced an invalid integer literal");

        Expr::new(ExprKind::IntLiteral(value), span)
    }

    fn parse_float_literal(&self, symbol: Symbol, span: Span) -> Expr {
        // It is okay to panic here, because this is not a user error. If the lexer
        // works as intended, it will not lex any invalid FloatLiteral.
        let value = self
            .symbol_registry
            .resolve(symbol)
            .parse::<f64>()
            .expect("Lexer produced an invalid float literal");

        Expr::new(ExprKind::FloatLiteral(value), span)
    }

    /// Parses an [`Expr`] starting at the current token.
    ///
    /// This is the main entry point for expression parsing. It starts the Pratt
    /// parser with the lowest binding power so that the complete [`Expr`] can
    /// be parsed.
    pub fn parse_expr(&mut self) -> Result<Expr, ParserError> {
        self.parse_bp(0)
    }

    /// Parses an [`Expr`] using Pratt parsing.
    ///
    /// `min_bp` is the minimum binding power an operator must have to become part
    /// of the current expression. Operators with lower binding power are left for
    /// the caller to parse.
    fn parse_bp(&mut self, min_bp: u8) -> Result<Expr, ParserError> {
        let mut lhs = self.parse_prefix_expression()?;

        loop {
            // Function calls bind more tightly than binary operators.
            if self.check(TokenKind::OpenParen) {
                lhs = self.parse_call_expression(lhs)?;
                continue;
            }

            let Some(op) = self.peek().and_then(BinaryOp::from_token) else {
                break;
            };

            let (left_bp, right_bp) = op.binding_power();

            if left_bp < min_bp {
                break;
            }

            self.advance();

            lhs = self.parse_binary_expression(lhs, op, right_bp)?;
        }

        Ok(lhs)
    }

    /// Parses an [`Expr`] that begins at the current token.
    ///
    /// This is the null denotation (nud) step in Pratt parsing. It handles tokens
    /// that can start an expression, such as integer literals, identifiers, prefix
    /// operators, and parenthesized expressions.
    ///
    /// For example, when parsing 1 + 2, this method first parses 1 before the
    /// parser continues with the + operator.
    fn parse_prefix_expression(&mut self) -> Result<Expr, ParserError> {
        let (kind, span) = self
            .advance()
            .map(|token| (token.kind, token.span))
            .ok_or_else(|| UnexpectedEofError {
                at: self.eof_span().into(),
                src: self.source_file.clone(),
            })?;

        let expr = match kind {
            TokenKind::IntLiteral(symbol) => self.parse_integer_literal(symbol, span),
            TokenKind::FloatLiteral(symbol) => self.parse_float_literal(symbol, span),
            TokenKind::Identifier(symbol) => Expr::new(ExprKind::Identifier(symbol), span),
            TokenKind::OpenParen => self.parse_grouped_expression(span)?,
            unary if is_unary_operator(unary) => self.parse_unary_expr()?,
            unexpected => {
                return Err(ExpectedExpressionError {
                    at: span.into(),
                    src: self.source_file.clone(),
                    found: unexpected,
                }
                .into());
            }
        };

        Ok(expr)
    }

    /// Parses the right-hand side of a binary [`Expr`]  and combines it with the
    /// previously parsed left-hand side.
    ///
    /// This corresponds to the left denotation (`led`) step in Pratt parsing.
    /// The right-hand expression is parsed using `right_bp`, which controls
    /// precedence and associativity.
    ///
    /// For example, after parsing `1` and consuming `+` in `1 + 2`, this method
    /// parses `2` and produces the Expr `1 + 2`.
    fn parse_binary_expression(
        &mut self,
        lhs: Expr,
        op: BinaryOp,
        right_bp: u8,
    ) -> Result<Expr, ParserError> {
        let rhs = self.parse_bp(right_bp)?;
        let span = Span::from_bounds(lhs.span(), rhs.span());

        Ok(Expr::new(
            ExprKind::Binary {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            },
            span,
        ))
    }

    /// Parses an [`Expr`]  enclosed in parentheses.
    ///
    /// The opening ( is consumed by the caller before this method is invoked.
    /// This method parses the expression inside the parentheses and then requires
    /// a matching closing ).
    fn parse_grouped_expression(&mut self, open_paren_span: Span) -> Result<Expr, ParserError> {
        let mut expr = self.parse_expr()?;

        let close_paren_span = self
            .expect_closing(TokenKind::CloseParen, open_paren_span)?
            .span;

        // Grouping parentheses are omitted from the AST, but the expression span
        // still covers them so diagnostics can reference the full source range.
        let span = Span::from_bounds(open_paren_span, close_paren_span);
        expr.set_span(span);

        Ok(expr)
    }

    /// Parses a function call whose callee has already been parsed.
    ///
    /// For example, after parsing `foo` in `foo(1, 2)`, this method parses the
    /// argument list and produces a call [`Expr`]  with `foo` as its callee.
    fn parse_call_expression(&mut self, callee: Expr) -> Result<Expr, ParserError> {
        let open_paren = self.expect(TokenKind::OpenParen)?;
        let open_paren_span = open_paren.span;

        let mut arguments = Vec::new();

        if !self.check(TokenKind::CloseParen) {
            loop {
                arguments.push(self.parse_expr()?);

                if self.check(TokenKind::Comma) {
                    self.advance();
                    continue;
                }

                if self.check(TokenKind::CloseParen) {
                    break;
                }

                // We are missing either a `,` or a `)`.
                // Check if the current token could be the start of a new expression.
                if self.is_expr_start() {
                    // Example: `hello(29, 39 11)`
                    // The `11` is an expression start. Force a missing comma error here.
                    self.expect(TokenKind::Comma)?;
                } else {
                    // Example: `call(a, b \n` or `call(a, b }`
                    // The next token (Newline, EOF, etc.) does NOT start an expression.
                    // Break the loop so the final `expect` below throws the missing `)` error.
                    break;
                }
            }
        }

        let close_paren_span = self
            .expect_closing(TokenKind::CloseParen, open_paren_span)?
            .span;

        let span = Span::from_bounds(callee.span(), close_paren_span);

        Ok(Expr::new(
            ExprKind::Call {
                callee: Box::new(callee),
                arguments,
            },
            span,
        ))
    }

    /// Parses a unary prefix expression.
    ///
    /// This method is called as part of the null denotation (nud) step in
    /// Pratt parsing. It expects the caller to have already consumed the
    /// prefix operator token. It dynamically fetches the operator's binding
    /// power and parses the right-hand operand accordingly.
    fn parse_unary_expr(&mut self) -> Result<Expr, ParserError> {
        // `parse_unary` is exclusively called by `parse_prefix_expression`
        // immediately after advancing past a validated prefix operator.
        // Therefore, `self.previous()` is guaranteed to exist, and
        // `UnaryOp::from_token` is guaranteed to succeed. A panic here
        // indicates a parser bug.
        let op = self
            .previous()
            .expect("parse_unary called without advancing the parser first");

        let start = op.span;
        let unary_op = UnaryOp::from_token(op).unwrap_or_else(|| {
            panic!(
                "parse_unary expected prefix operator, but found {:?}",
                op.kind
            )
        });

        let right_bp = unary_op.binding_power();
        let expr = self.parse_bp(right_bp)?;

        let span = Span::from_bounds(start, expr.span());

        Ok(Expr::new(
            ExprKind::Unary {
                op: unary_op,
                expr: Box::new(expr),
            },
            span,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::make_lexer;
    use rstest::rstest;

    fn parse_expression(source: &str) -> Expr {
        let (mut lexer, source_file) = make_lexer(source);

        let tokens = lexer.by_ref().map(|tok| tok.unwrap()).collect::<Vec<_>>();

        let mut parser = Parser::new(&tokens, &lexer.symbol_registry, source_file);

        parser.parse_expr().unwrap()
    }

    fn int(value: i64) -> Expr {
        Expr::new(ExprKind::IntLiteral(value), Span::default())
    }

    fn float(value: f64) -> Expr {
        Expr::new(ExprKind::FloatLiteral(value), Span::default())
    }

    fn binary(left: Expr, op: BinaryOp, right: Expr) -> Expr {
        Expr::new(
            ExprKind::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            },
            Span::default(),
        )
    }

    fn unary(op: UnaryOp, expr: Expr) -> Expr {
        Expr::new(
            ExprKind::Unary {
                op,
                expr: Box::new(expr),
            },
            Span::default(),
        )
    }

    #[rstest]
    #[case("42", int(42))]
    #[case("0", int(0))]
    #[allow(clippy::approx_constant)]
    #[case("3.14", float(3.14))]
    #[case("0.5", float(0.5))]
    fn parses_literals(#[case] source: &str, #[case] expected: Expr) {
        assert_eq!(parse_expression(source), expected);
    }

    #[rstest]
    #[case("1 + 2", BinaryOp::Plus)]
    #[case("1 - 2", BinaryOp::Minus)]
    #[case("1 * 2", BinaryOp::Multiply)]
    #[case("1 / 2", BinaryOp::Divide)]
    fn parses_binary_operators(#[case] source: &str, #[case] op: BinaryOp) {
        let expected = binary(int(1), op, int(2));

        assert_eq!(parse_expression(source), expected);
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        let expr = parse_expression("1 + 2 * 3");

        let expected = binary(
            int(1),
            BinaryOp::Plus,
            binary(int(2), BinaryOp::Multiply, int(3)),
        );

        assert_eq!(expr, expected);
    }

    #[test]
    fn division_binds_tighter_than_subtraction() {
        let expr = parse_expression("10 - 8 / 2");

        let expected = binary(
            int(10),
            BinaryOp::Minus,
            binary(int(8), BinaryOp::Divide, int(2)),
        );

        assert_eq!(expr, expected);
    }

    #[test]
    fn addition_is_left_associative() {
        let expr = parse_expression("1 + 2 + 3");

        let expected = binary(
            binary(int(1), BinaryOp::Plus, int(2)),
            BinaryOp::Plus,
            int(3),
        );

        assert_eq!(expr, expected);
    }

    #[test]
    fn subtraction_is_left_associative() {
        let expr = parse_expression("10 - 5 - 2");

        let expected = binary(
            binary(int(10), BinaryOp::Minus, int(5)),
            BinaryOp::Minus,
            int(2),
        );

        assert_eq!(expr, expected);
    }

    #[test]
    fn grouped_expression_overrides_precedence() {
        let expr = parse_expression("(1 + 2) * 3");

        let expected = binary(
            binary(int(1), BinaryOp::Plus, int(2)),
            BinaryOp::Multiply,
            int(3),
        );

        assert_eq!(expr, expected);
    }

    #[test]
    fn nested_groups_are_parsed() {
        let expr = parse_expression("((1 + 2) * 3)");

        let expected = binary(
            binary(int(1), BinaryOp::Plus, int(2)),
            BinaryOp::Multiply,
            int(3),
        );

        assert_eq!(expr, expected);
    }

    #[rstest]
    #[case("-42", unary(UnaryOp::Minus, int(42)))]
    #[case("!42", unary(UnaryOp::Not, int(42)))]
    #[allow(clippy::approx_constant)]
    #[case("-3.14", unary(UnaryOp::Minus, float(3.14)))]
    fn parses_unary_operators(#[case] source: &str, #[case] expected: Expr) {
        assert_eq!(parse_expression(source), expected);
    }

    #[test]
    fn parses_unary_identifier() {
        let expr = parse_expression("-foo");

        let ExprKind::Unary { op, expr: inner } = expr.kind() else {
            panic!("Expected unary expression");
        };

        assert_eq!(*op, UnaryOp::Minus);
        assert!(matches!(inner.kind(), ExprKind::Identifier(_)));
    }

    #[test]
    fn parses_unary_call() {
        let expr = parse_expression("!foo()");

        let ExprKind::Unary { op, expr: inner } = expr.kind() else {
            panic!("Expected unary expression");
        };

        assert_eq!(*op, UnaryOp::Not);
        assert!(matches!(inner.kind(), ExprKind::Call { .. }));
    }

    #[test]
    fn unary_binds_tighter_than_addition() {
        // -1 + 2 should be parsed as (-1) + 2, not -(1 + 2)
        let expr = parse_expression("-1 + 2");

        let expected = binary(unary(UnaryOp::Minus, int(1)), BinaryOp::Plus, int(2));

        assert_eq!(expr, expected);
    }

    #[test]
    fn unary_binds_tighter_than_multiplication() {
        // !1 * 2 should be parsed as (!1) * 2
        let expr = parse_expression("!1 * 2");

        let expected = binary(unary(UnaryOp::Not, int(1)), BinaryOp::Multiply, int(2));

        assert_eq!(expr, expected);
    }

    #[test]
    fn multiple_unary_operators_are_parsed() {
        // !!1 should be parsed as !(!1)
        let expr = parse_expression("!!1");

        let expected = unary(UnaryOp::Not, unary(UnaryOp::Not, int(1)));

        assert_eq!(expr, expected);
    }
    // =========================================================================
    // Span Tests
    // =========================================================================
    #[test]
    fn parses_literal_spans() {
        let expr = parse_expression("42");
        // Assuming "42" starts at byte 0 and ends at byte 2, on line 1, column 1
        assert_eq!(expr.span(), Span::new(0, 2, 1, 1));
    }

    #[test]
    fn parses_binary_expression_spans() {
        // "1 + 2"
        // '1' is at 0..1
        // '+' is at 2..3
        // '2' is at 4..5
        let expr = parse_expression("1 + 2");

        // The binary expression should span from the start of '1' to the end of '2' (0..5)
        assert_eq!(expr.span(), Span::new(0, 5, 1, 1));

        // Verify inner node spans as well
        if let ExprKind::Binary { left, right, .. } = expr.kind() {
            assert_eq!(left.span(), Span::new(0, 1, 1, 1));
            assert_eq!(right.span(), Span::new(4, 5, 1, 5));
        } else {
            panic!("Expected binary expression");
        }
    }

    #[test]
    fn parses_grouped_expression_spans() {
        // "(1 + 2)"
        // '(' at 0..1, '1' at 1..2, '+' at 3..4, '2' at 5..6, ')' at 6..7
        let expr = parse_expression("(1 + 2)");

        // The grouped expression span should enclose the parentheses (0..7)
        assert_eq!(expr.span(), Span::new(0, 7, 1, 1));
    }

    #[test]
    fn parses_nested_precedence_spans() {
        // "1 + 2 * 3"
        let expr = parse_expression("1 + 2 * 3");

        // Outer binary expression (+): spans from '1' (0) to '3' (8) -> 0..9 roughly depending on exact spacing
        assert_eq!(expr.span().start, 0);

        // Inner binary expression (*): "2 * 3" spans from '2' to '3'
        if let ExprKind::Binary { right, .. } = expr.kind() {
            assert_eq!(right.span(), Span::new(4, 9, 1, 5));
        } else {
            panic!("Expected binary expression");
        }
    }

    #[test]
    fn parses_complex_expression_spans_with_spacing() {
        // Source: "  ( 10 + 20 ) * 30  "        // Index 0-1: two spaces "  "
        // Index 2: '('
        // Index 3: ' '
        // Index 4-5: '10'
        // Index 6: ' '
        // Index 7: '+'
        // Index 8: ' '
        // Index 9-10: '20'
        // Index 11: ' '
        // Index 12: ')'
        // Index 13: ' '
        // Index 14: '*'
        // Index 15: ' '
        // Index 16-17: '30'
        // Index 18-19: two trailing spaces "  "
        let source = "  ( 10 + 20 ) * 30  ";
        let expr = parse_expression(source);

        // The overall expression is a multiplication (*) spanning from the opening '(' to '30'
        // Opening parenthesis starts at index 2, '30' ends at index 18.
        assert_eq!(expr.span(), Span::new(2, 18, 1, 3));

        if let ExprKind::Binary { left, op, right } = expr.kind() {
            assert_eq!(*op, BinaryOp::Multiply);

            // Left side is the grouped expression "( 10 + 20 )", spanning from index 2 to 13
            assert_eq!(left.span(), Span::new(2, 13, 1, 3));

            // Right side is the integer literal "30", spanning from index 16 to 18
            assert_eq!(right.span(), Span::new(16, 18, 1, 17));

            // Check inner parts of the grouped expression
            if let ExprKind::Binary {
                left: inner_left,
                op: inner_op,
                right: inner_right,
            } = left.kind()
            {
                assert_eq!(*inner_op, BinaryOp::Plus);
                // "10" spans from index 4 to 6
                assert_eq!(inner_left.span(), Span::new(4, 6, 1, 5));
                // "20" spans from index 9 to 11
                assert_eq!(inner_right.span(), Span::new(9, 11, 1, 10));
            } else {
                panic!("Expected inner binary expression inside parentheses");
            }
        } else {
            panic!("Expected outer multiplication expression");
        }
    }

    #[test]
    fn parses_unary_expression_spans() {
        // "-42"
        // '-' is at 0..1 (col 1)
        // '42' is at 1..3 (col 2)
        let expr = parse_expression("-42");

        // The unary expression should span from the start of '-' to the end of '42' (0..3)
        assert_eq!(expr.span(), Span::new(0, 3, 1, 1));

        // Verify inner node span as well
        if let ExprKind::Unary {
            expr: inner_expr, ..
        } = expr.kind()
        {
            assert_eq!(inner_expr.span(), Span::new(1, 3, 1, 2));
        } else {
            panic!("Expected unary expression");
        }
    }

    #[test]
    fn parses_unary_expression_spans_with_spacing() {
        // "!  42"
        // '!' is at 0..1 (col 1)
        // two spaces at 1..3
        // '42' is at 3..5 (col 4)
        let expr = parse_expression("!  42");

        // The unary expression should span from the start of '!' to the end of '42' (0..5)
        assert_eq!(expr.span(), Span::new(0, 5, 1, 1));

        if let ExprKind::Unary {
            expr: inner_expr, ..
        } = expr.kind()
        {
            assert_eq!(inner_expr.span(), Span::new(3, 5, 1, 4));
        } else {
            panic!("Expected unary expression");
        }
    }
}
