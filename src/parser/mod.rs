pub mod ast;
pub mod expression;

use crate::lexer::{SymbolRegistry, Token, TokenKind};

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
}
