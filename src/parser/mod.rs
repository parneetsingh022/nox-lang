use crate::{
    parser::ast::{BinaryOp, Expression},
    tokenizer::{Symbol, SymbolRegistry, Token, TokenKind},
};

pub mod ast;

pub struct Parser<'a> {
    tokens: &'a [Token],
    symbol_registry: &'a SymbolRegistry,
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], symbol_registry: &'a SymbolRegistry) -> Self {
        Self {
            tokens,
            pos: 0,
            symbol_registry,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(token)
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == kind)
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> &Token {
        if self.check(kind) {
            self.advance().unwrap()
        } else {
            panic!("{message}");
        }
    }

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

    pub fn parse_expr(&mut self) -> Expression {
        self.parse_bp(0)
    }

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
