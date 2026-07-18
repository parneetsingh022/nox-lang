use crate::parser::Parser;

use crate::{
    lexer::{Symbol, TokenKind},
    parser::ast::{BinaryOp, Expression},
};

impl<'a> Parser<'a> {
    fn parse_integer_literal(&self, symbol: Symbol) -> Expression {
        let value = self
            .symbol_registry
            .resolve(symbol)
            .parse::<i64>()
            .expect("Lexer produced an invalid integer literal");

        Expression::IntLiteral(value)
    }

    fn parse_float_literal(&self, symbol: Symbol) -> Expression {
        let value = self
            .symbol_registry
            .resolve(symbol)
            .parse::<f64>()
            .expect("Lexer produced an invalid float literal");

        Expression::FloatLiteral(value)
    }

    /// Parses an expression starting at the current token.
    ///
    /// This is the main entry point for expression parsing. It starts the Pratt
    /// parser with the lowest binding power so that the complete expression can
    /// be parsed.
    pub fn parse_expr(&mut self) -> Expression {
        self.parse_bp(0)
    }

    /// Parses an expression using Pratt parsing.
    ///
    /// `min_bp` is the minimum binding power an operator must have to become part
    /// of the current expression. Operators with lower binding power are left for
    /// the caller to parse.
    fn parse_bp(&mut self, min_bp: u8) -> Expression {
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

    /// Parses an expression that begins at the current token.
    ///
    /// This is the null denotation (nud) step in Pratt parsing. It handles tokens
    /// that can start an expression, such as integer literals, identifiers, prefix
    /// operators, and parenthesized expressions.
    ///
    /// For example, when parsing 1 + 2, this method first parses 1 before the
    /// parser continues with the + operator.
    fn parse_prefix_expression(&mut self) -> Expression {
        let kind = self
            .advance()
            .map(|token| token.kind)
            .expect("Expected token find eof!");

        match kind {
            TokenKind::IntLiteral(symbol) => self.parse_integer_literal(symbol),
            TokenKind::FloatLiteral(symbol) => self.parse_float_literal(symbol),
            TokenKind::Identifier(symbol) => Expression::Identifier(symbol),
            TokenKind::OpenParen => self.parse_grouped_expression(),
            _ => panic!("Unexpected token!"),
        }
    }

    /// Parses the right-hand side of a binary expression and combines it with the
    /// previously parsed left-hand side.
    ///
    /// This corresponds to the left denotation (`led`) step in Pratt parsing.
    /// The right-hand expression is parsed using `right_bp`, which controls
    /// precedence and associativity.
    ///
    /// For example, after parsing `1` and consuming `+` in `1 + 2`, this method
    /// parses `2` and produces the expression `1 + 2`.
    fn parse_binary_expression(
        &mut self,
        lhs: Expression,
        op: BinaryOp,
        right_bp: u8,
    ) -> Expression {
        let rhs = self.parse_bp(right_bp);

        Expression::Binary {
            left: Box::new(lhs),
            op,
            right: Box::new(rhs),
        }
    }

    /// Parses an expression enclosed in parentheses.
    ///
    /// The opening ( is consumed by the caller before this method is invoked.
    /// This method parses the expression inside the parentheses and then requires
    /// a matching closing ).
    fn parse_grouped_expression(&mut self) -> Expression {
        let expression = self.parse_expr();

        self.expect(
            TokenKind::CloseParen,
            "Expected `)` after grouped expression",
        );

        expression
    }

    /// Parses a function call whose callee has already been parsed.
    ///
    /// For example, after parsing `foo` in `foo(1, 2)`, this method parses the
    /// argument list and produces a call expression with `foo` as its callee.
    fn parse_call_expression(&mut self, callee: Expression) -> Expression {
        self.expect(
            TokenKind::OpenParen,
            "Expected `(` after function expression",
        );

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

        self.expect(
            TokenKind::CloseParen,
            "Expected `)` after function arguments",
        );

        Expression::Call {
            callee: Box::new(callee),
            arguments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use rstest::rstest;

    fn parse_expression(source: &str) -> Expression {
        let mut lexer = Lexer::new(source, "main.nox");

        let tokens = lexer.by_ref().map(|tok| tok.unwrap()).collect::<Vec<_>>();

        let mut parser = Parser::new(&tokens, &lexer.symbol_registry);

        parser.parse_expr()
    }

    fn int(value: i64) -> Expression {
        Expression::IntLiteral(value)
    }

    fn float(value: f64) -> Expression {
        Expression::FloatLiteral(value)
    }

    fn binary(left: Expression, op: BinaryOp, right: Expression) -> Expression {
        Expression::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    #[rstest]
    #[case("42", int(42))]
    #[case("0", int(0))]
    #[allow(clippy::approx_constant)]
    #[case("3.14", float(3.14))]
    #[case("0.5", float(0.5))]
    fn parses_literals(#[case] source: &str, #[case] expected: Expression) {
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
        let expression = parse_expression("1 + 2 * 3");

        let expected = binary(
            int(1),
            BinaryOp::Plus,
            binary(int(2), BinaryOp::Multiply, int(3)),
        );

        assert_eq!(expression, expected);
    }

    #[test]
    fn division_binds_tighter_than_subtraction() {
        let expression = parse_expression("10 - 8 / 2");

        let expected = binary(
            int(10),
            BinaryOp::Minus,
            binary(int(8), BinaryOp::Divide, int(2)),
        );

        assert_eq!(expression, expected);
    }

    #[test]
    fn addition_is_left_associative() {
        let expression = parse_expression("1 + 2 + 3");

        let expected = binary(
            binary(int(1), BinaryOp::Plus, int(2)),
            BinaryOp::Plus,
            int(3),
        );

        assert_eq!(expression, expected);
    }

    #[test]
    fn subtraction_is_left_associative() {
        let expression = parse_expression("10 - 5 - 2");

        let expected = binary(
            binary(int(10), BinaryOp::Minus, int(5)),
            BinaryOp::Minus,
            int(2),
        );

        assert_eq!(expression, expected);
    }

    #[test]
    fn grouped_expression_overrides_precedence() {
        let expression = parse_expression("(1 + 2) * 3");

        let expected = binary(
            binary(int(1), BinaryOp::Plus, int(2)),
            BinaryOp::Multiply,
            int(3),
        );

        assert_eq!(expression, expected);
    }

    #[test]
    fn nested_groups_are_parsed() {
        let expression = parse_expression("((1 + 2) * 3)");

        let expected = binary(
            binary(int(1), BinaryOp::Plus, int(2)),
            BinaryOp::Multiply,
            int(3),
        );

        assert_eq!(expression, expected);
    }
}
