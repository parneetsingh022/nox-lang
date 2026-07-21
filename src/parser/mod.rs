pub mod ast;
pub mod expression;
pub mod statements;

use std::panic;

use crate::{
    lexer::{Symbol, SymbolRegistry, Token, TokenKind},
    parser::ast::Program,
};

pub struct Parser<'a> {
    tokens: &'a [Token],
    symbol_registry: &'a SymbolRegistry,
    pos: usize,
    program: Program,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], symbol_registry: &'a SymbolRegistry) -> Self {
        Self {
            tokens,
            pos: 0,
            symbol_registry,
            program: Program::default(),
        }
    }

    pub fn parse(&mut self) {
        while !self.is_eof() {
            let statement = self.parse_statement();
            self.program.push(statement);
        }
    }

    pub fn take_program(&mut self) -> Program {
        std::mem::take(&mut self.program)
    }

    fn is_eof(&self) -> bool {
        self.tokens.len() <= self.pos
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

    fn expect_keyword(&mut self, message: &str) -> &Token {
        let token = self.advance();
        match token.map(|token| token.kind) {
            Some(TokenKind::Keyword(_)) => token.unwrap(),
            Some(kind) => panic!("{message}, found {kind:?}"),
            None => panic!("{message}, found EOF"),
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Symbol {
        let Some(token) = self.advance() else {
            panic!("{message}, got EOF");
        };

        match token.kind {
            TokenKind::Identifier(symbol) => symbol,
            _ => panic!("{message}, got: {:?}", token.kind),
        }
    }
}
