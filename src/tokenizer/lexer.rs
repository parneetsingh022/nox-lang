use crate::{
    diagnostic::Span,
    tokenizer::{Token, TokenKind},
};

/// Tracks current position for lexer in source file
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Cursor {
    offset: usize,
    line: usize,
    column: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            offset: 0,
            line: 1,
            column: 1,
        }
    }
}

impl Cursor {
    fn consume(&mut self, ch: u8) {
        match ch {
            b'\n' => self.newline(),
            _ => {
                self.offset += 1;
                self.column += 1;
            }
        }
    }

    fn newline(&mut self) {
        self.offset += 1;
        self.line += 1;
        self.column = 1;
    }
}

pub struct Lexer<'a> {
    source: &'a [u8],
    cursor: Cursor,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.parse_next_token())
    }
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            cursor: Cursor::default(),
        }
    }

    fn at_eof(&self) -> bool {
        self.cursor.offset >= self.source.len()
    }

    fn peek(&mut self) -> Option<u8> {
        if self.at_eof() {
            return None;
        }

        let ch = self.source[self.cursor.offset];
        Some(ch)
    }

    fn advance(&mut self) {
        if let Some(ch) = self.peek() {
            self.cursor.consume(ch);
        }
    }

    fn parse_next_token(&mut self) -> Token {
        match self.peek() {
            Some(b' ' | b'\n') => {
                self.advance();
                self.parse_next_token()
            }
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => self.lex_identifier(),
            Some(unexpected) => panic!("Unexpected char: '{}'", unexpected as char),
            None => self.lex_eof(),
        }
    }

    fn lex_eof(&self) -> Token {
        let offset = self.cursor.offset;

        let line = self.cursor.line;
        let column = self.cursor.column;

        let span = Span::new(offset, offset, line, column);
        return Token::new(TokenKind::Eof, span);
    }

    fn lex_identifier(&mut self) -> Token {
        let mut ident = String::new();
        let start = self.cursor.offset;
        let start_line = self.cursor.line;
        let start_column = self.cursor.column;

        while let Some(ch) = self.peek()
            && (ch.is_ascii_alphanumeric() || ch == b'_')
        {
            ident.push(ch as char);
            self.advance();
        }

        let span = Span::new(start, self.cursor.offset, start_line, start_column);

        Token::new(TokenKind::Identifier(ident), span)
    }
}
