//! Syntax parser for translating token streams into abstract syntax trees (ASTs).

pub mod ast;
pub mod expression;

use crate::{
    diagnostic::{ExpectedTokenError, ParserError, SourceFile, Span, UnexpectedEofError},
    lexer::{SymbolRegistry, Token, TokenKind},
};

/// Parses a stream of lexical tokens into an abstract syntax tree (AST).
pub struct Parser<'a> {
    source_file: SourceFile,
    tokens: &'a [Token],
    symbol_registry: &'a SymbolRegistry,
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(
        tokens: &'a [Token],
        symbol_registry: &'a SymbolRegistry,
        source_file: SourceFile,
    ) -> Self {
        Self {
            source_file,
            tokens,
            pos: 0,
            symbol_registry,
        }
    }

    /// Returns the current token without advancing the position.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    /// Returns the most recently consumed token.
    fn previous(&self) -> Option<&'a Token> {
        self.pos.checked_sub(1).and_then(|pos| self.tokens.get(pos))
    }

    fn eof_span(&self) -> Span {
        self.previous()
            .map_or(Span::new(0, 0, 1, 1), |token| token.span)
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

    pub fn expect(&mut self, expected: TokenKind) -> Result<&Token, ParserError> {
        match self.peek() {
            Some(token) if token.kind == expected => {
                // Safe to unwrap because peek() just guaranteed a token exists
                Ok(self.advance().unwrap())
            }

            // We found a token, but it's the wrong kind
            Some(token) => {
                let found = token.kind;
                // Fall back to the previous token's span if available,
                // otherwise use the current token's span.
                let span = self.previous().map(|prev| prev.span).unwrap_or(token.span);
                Err(ExpectedTokenError {
                    expected,
                    found,
                    at: span.into(),
                    src: self.source_file.clone(),
                }
                .into())
            }

            None => Err(UnexpectedEofError {
                at: self.eof_span().into(),
                src: self.source_file.clone(),
            }
            .into()),
        }
    }

    /// Expects a closing delimiter (like `)` or `}`).
    /// If the token is missing, throws an UnclosedDelimiterError pointing to the `opened_at` span.
    pub fn expect_closing(
        &mut self,
        expected: TokenKind,
        opened_at: Span,
    ) -> Result<&Token, ParserError> {
        match self.peek() {
            Some(token) if token.kind == expected => Ok(self.advance().unwrap()),
            _ => Err(crate::diagnostic::UnclosedDelimiterError {
                expected,
                opened_at: opened_at.into(),
                src: self.source_file.clone(),
            }
            .into()),
        }
    }
}
