//! Syntax parser for translating token streams into abstract syntax trees (ASTs).

pub mod ast;
pub mod expression;

use crate::lexer::{SymbolRegistry, Token, TokenKind};

/// Parses a stream of lexical tokens into an abstract syntax tree (AST).
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

    /// Returns the current token without advancing the position.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    /// Consumes the current token and advances the parser position.
    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos)?;
        self.pos += 1;
        Some(token)
    }

    /// Returns `true` if the current token matches the given kind.
    fn check(&self, kind: TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == kind)
    }

    /// Consumes the current token if it matches the given kind, or panics with a message.
    fn expect(&mut self, kind: TokenKind, message: &str) -> &Token {
        if self.check(kind) {
            self.advance().unwrap()
        } else {
            panic!("{message}");
        }
    }
}
