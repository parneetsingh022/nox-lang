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

    fn span_from(&self, start: Cursor) -> Span {
        Span::new(start.offset, self.cursor.offset, start.line, start.column)
    }

    fn parse_next_token(&mut self) -> Token {
        match self.peek() {
            Some(b' ' | b'\n') => {
                self.advance();
                self.parse_next_token()
            }
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => self.lex_identifier(),
            Some(b'0'..=b'9') => self.lex_integer(),
            Some(unexpected) => panic!("Unexpected char: '{}'", unexpected as char),
            None => self.lex_eof(),
        }
    }

    fn lex_eof(&self) -> Token {
        let span = Span::new(
            self.cursor.offset,
            self.cursor.offset,
            self.cursor.line,
            self.cursor.column,
        );
        return Token::new(TokenKind::Eof, span);
    }

    fn lex_identifier(&mut self) -> Token {
        let mut ident = String::new();
        let start = self.cursor.clone();

        while let Some(ch) = self.peek()
            && (ch.is_ascii_alphanumeric() || ch == b'_')
        {
            ident.push(ch as char);
            self.advance();
        }

        let span = self.span_from(start);

        // Attempt to classify the identifier as a language keyword.
        // If it is not a keyword, fall back to treating it as a standard identifier.
        let token_kind =
            TokenKind::map_keyword(&ident).unwrap_or_else(|| TokenKind::Identifier(ident));

        Token::new(token_kind, span)
    }

    fn lex_integer(&mut self) -> Token {
        let mut value = String::new();
        let start = self.cursor.clone();

        while let Some(ch) = self.peek()
            && (ch.is_ascii_digit())
        {
            value.push(ch as char);
            self.advance();
        }

        if let Some(ch) = self.peek()
            && ch == b'.'
        {
            return self.lex_float(value.as_str(), start);
        }

        let span = self.span_from(start);
        Token::new(TokenKind::IntLiteral(value), span)
    }

    fn lex_float(&mut self, pre: &str, start: Cursor) -> Token {
        let mut value = String::from(pre);
        value.push('.');
        self.advance();

        while let Some(ch) = self.peek()
            && (ch.is_ascii_digit())
        {
            value.push(ch as char);
            self.advance();
        }

        let span = self.span_from(start);
        Token::new(TokenKind::FloatLiteral(value), span)
    }
}
