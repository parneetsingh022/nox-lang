//! Syntax parser for translating token streams into abstract syntax trees (ASTs).

pub mod ast;
pub mod expression;
pub mod statement;

use crate::{
    diagnostic::{ParserError, SourceFile, Span},
    lexer::{SymbolRegistry, Token, TokenKind},
    parser::ast::{SpannedIdentifier, Stmt},
};

/// Parses a stream of lexical tokens into an abstract syntax tree (AST).
pub struct Parser<'a> {
    source_file: SourceFile,
    tokens: &'a [Token],
    symbol_registry: &'a SymbolRegistry,
    pos: usize,
}

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Stmt, ParserError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_eof() {
            return None;
        }

        Some(self.parse_stmt())
    }
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
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.pos).copied()
    }

    /// Returns the most recently consumed token.
    fn previous(&self) -> Option<Token> {
        self.pos
            .checked_sub(1)
            .and_then(|pos| self.tokens.get(pos))
            .copied()
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn eof_span(&self) -> Span {
        self.previous()
            .map_or(Span::new(0, 0, 1, 1), |token| token.span)
    }

    /// Consumes the current token and advances the parser position.
    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).copied()?;
        self.pos += 1;
        Some(token)
    }

    /// Returns `true` if the current token matches the given kind.
    fn check(&self, kind: TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == kind)
    }

    fn expect(&mut self, expected: TokenKind) -> Result<Token, ParserError> {
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
                Err(ParserError::ExpectedToken {
                    expected,
                    found,
                    at: span.into(),
                    src: self.source_file.clone(),
                })
            }

            None => Err(self.unexpected_eof_error()),
        }
    }

    fn expect_identifier(&mut self) -> Result<SpannedIdentifier, ParserError> {
        let token = self.peek().ok_or_else(|| self.unexpected_eof_error())?;

        match token.kind {
            TokenKind::Identifier(symbol) => {
                let span = token.span;
                self.advance().unwrap();
                Ok(SpannedIdentifier::new(symbol, span))
            }
            _ => Err(ParserError::ExpectedIdentifier {
                found: token.kind,
                at: token.span.into(),
                src: self.source_file.clone(),
            }),
        }
    }

    fn expect_semicolon(&mut self) -> Result<Token, ParserError> {
        let peeked = self.peek();

        if let Some(token) = peeked
            && token.kind == TokenKind::Semi
        {
            self.advance(); // consume semicolon
            return Ok(token);
        }

        // Use the previous token's span so the error diagnostic points to the end
        // of the current line (where the semicolon is missing) rather than pointing
        // at the first token on the next line.
        let err_token = self.previous().or(peeked);
        let span = err_token.map(|token| token.span).unwrap_or(self.eof_span());

        Err(ParserError::ExpectedSemicolon {
            at: span.into(),
            src: self.source_file.clone(),
        })
    }

    /// Expects a closing delimiter (like `)` or `}`).
    /// If the token is missing, throws an UnclosedDelimiterError pointing to the `opened_at` span.
    fn expect_closing(
        &mut self,
        expected: TokenKind,
        opened_at: Span,
    ) -> Result<Token, ParserError> {
        match self.peek() {
            Some(token) if token.kind == expected => Ok(self.advance().unwrap()),
            _ => Err(ParserError::UnclosedDelimiter {
                expected,
                opened_at: opened_at.into(),
                src: self.source_file.clone(),
            }),
        }
    }

    fn unexpected_eof_error(&self) -> ParserError {
        ParserError::UnexpectedEof {
            at: self.eof_span().into(),
            src: self.source_file.clone(),
        }
    }

    fn expected_statement_error(&self, token: Token) -> ParserError {
        ParserError::ExpectedStatement {
            found: token.kind,
            at: token.span.into(),
            src: self.source_file.clone(),
        }
    }
}
