use crate::diagnostic::Span;
use crate::parser::Parser;

use crate::{
    lexer::{Symbol, TokenKind},
    parser::ast::{BinaryOp, Expr, ExprKind},
};

impl<'a> Parser<'a> {
    fn parse_integer_literal(&self, symbol: Symbol, span: Span) -> Expr {
        let value = self
            .symbol_registry
            .resolve(symbol)
            .parse::<i64>()
            .expect("Lexer produced an invalid integer literal");

        Expr::new(ExprKind::IntLiteral(value), span)
    }

    fn parse_float_literal(&self, symbol: Symbol, span: Span) -> Expr {
        let value = self
            .symbol_registry
            .resolve(symbol)
            .parse::<f64>()
            .expect("Lexer produced an invalid float literal");

        Expr::new(ExprKind::FloatLiteral(value), span)
    }

    /// Parses an Expr starting at the current token.
    ///
    /// This is the main entry point for Expr parsing. It starts the Pratt
    /// parser with the lowest binding power so that the complete Expr can
    /// be parsed.
    pub fn parse_expr(&mut self) -> Expr {
        self.parse_bp(0)
    }

    /// Parses an Expr using Pratt parsing.
    ///
    /// `min_bp` is the minimum binding power an operator must have to become part
    /// of the current Expr. Operators with lower binding power are left for
    /// the caller to parse.
    fn parse_bp(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix_expression();

        loop {
            // Function calls bind more tightly than binary operators.
            if self.check(TokenKind::OpenParen) {
                lhs = self.parse_call_expression(lhs);
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

            lhs = self.parse_binary_expression(lhs, op, right_bp);
        }

        lhs
    }

    /// Parses an Expr that begins at the current token.
    ///
    /// This is the null denotation (nud) step in Pratt parsing. It handles tokens
    /// that can start an Expr, such as integer literals, identifiers, prefix
    /// operators, and parenthesized expressions.
    ///
    /// For example, when parsing 1 + 2, this method first parses 1 before the
    /// parser continues with the + operator.
    fn parse_prefix_expression(&mut self) -> Expr {
        let (kind, span) = self
            .advance()
            .map(|token| (token.kind, token.span))
            .expect("Expected token find eof!");

        match kind {
            TokenKind::IntLiteral(symbol) => self.parse_integer_literal(symbol, span),
            TokenKind::FloatLiteral(symbol) => self.parse_float_literal(symbol, span),
            TokenKind::Identifier(symbol) => Expr::new(ExprKind::Identifier(symbol), span),
            TokenKind::OpenParen => self.parse_grouped_expression(span),
            _ => panic!("Unexpected token!"),
        }
    }

    /// Parses the right-hand side of a binary Expr and combines it with the
    /// previously parsed left-hand side.
    ///
    /// This corresponds to the left denotation (`led`) step in Pratt parsing.
    /// The right-hand Expr is parsed using `right_bp`, which controls
    /// precedence and associativity.
    ///
    /// For example, after parsing `1` and consuming `+` in `1 + 2`, this method
    /// parses `2` and produces the Expr `1 + 2`.
    fn parse_binary_expression(&mut self, lhs: Expr, op: BinaryOp, right_bp: u8) -> Expr {
        let rhs = self.parse_bp(right_bp);
        let span = Span::from_bounds(lhs.span(), rhs.span());

        Expr::new(
            ExprKind::Binary {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
            },
            span,
        )
    }

    /// Parses an Expr enclosed in parentheses.
    ///
    /// The opening ( is consumed by the caller before this method is invoked.
    /// This method parses the Expr inside the parentheses and then requires
    /// a matching closing ).
    fn parse_grouped_expression(&mut self, open_pren_span: Span) -> Expr {
        let mut expr = self.parse_expr();

        let close_paren = self.expect(TokenKind::CloseParen, "Expected `)` after grouped Expr");

        // Grouping parentheses are omitted from the AST, but the expression span
        // still covers them so diagnostics can reference the full source range.
        let span = Span::from_bounds(open_pren_span, close_paren.span);
        expr.set_span(span);

        expr
    }

    /// Parses a function call whose callee has already been parsed.
    ///
    /// For example, after parsing `foo` in `foo(1, 2)`, this method parses the
    /// argument list and produces a call Expr with `foo` as its callee.
    fn parse_call_expression(&mut self, callee: Expr) -> Expr {
        self.expect(TokenKind::OpenParen, "Expected `(` after function Expr");

        let mut arguments = Vec::new();

        if !self.check(TokenKind::CloseParen) {
            loop {
                arguments.push(self.parse_expr());

                if !self.check(TokenKind::Comma) {
                    break;
                }

                self.advance(); // Consume `,`.
            }
        }

        let close_paren = self.expect(
            TokenKind::CloseParen,
            "Expected `)` after function arguments",
        );

        let span = Span::from_bounds(callee.span(), close_paren.span);

        Expr::new(
            ExprKind::Call {
                callee: Box::new(callee),
                arguments,
            },
            span,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use rstest::rstest;

    fn parse_expression(source: &str) -> Expr {
        let mut lexer = Lexer::new(source, "main.nox");

        let tokens = lexer.by_ref().map(|tok| tok.unwrap()).collect::<Vec<_>>();

        let mut parser = Parser::new(&tokens, &lexer.symbol_registry);

        parser.parse_expr()
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
}
