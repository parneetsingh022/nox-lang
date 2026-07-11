use crate::{
    diagnostic::Span,
    tokenizer::{Token, TokenKind},
};

/// Returns true if given character is a whitespace.
pub fn is_whitespace(ch: u8) -> bool {
    ch.is_ascii_whitespace()
}

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
    source: &'a str,
    chars: &'a [u8],
    cursor: Cursor,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_eof() {
            return None;
        }

        self.lex_next_token()
    }
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source,
            chars: source.as_bytes(),
            cursor: Cursor::default(),
        }
    }

    fn is_eof(&self) -> bool {
        self.cursor.offset >= self.chars.len()
    }

    fn peek(&self) -> Option<u8> {
        self.chars.get(self.cursor.offset).copied()
    }

    fn advance(&mut self) {
        if let Some(ch) = self.peek() {
            self.cursor.consume(ch);
        }
    }

    fn span_from(&self, start: Cursor) -> Span {
        Span::new(start.offset, self.cursor.offset, start.line, start.column)
    }

    fn read_while(&mut self, predicate: impl Fn(u8) -> bool) -> &'a str {
        let start = self.cursor.clone();
        while let Some(ch) = self.peek()
            && predicate(ch)
        {
            self.advance();
        }

        let span = self.span_from(start);

        return &self.source[span.start..span.end];
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek()
            && is_whitespace(ch)
        {
            self.advance();
        }
    }

    fn lex_next_token(&mut self) -> Option<Token<'a>> {
        self.skip_whitespace();

        match self.peek() {
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'_') => Some(self.lex_identifier()),
            Some(b'0'..=b'9') => Some(self.lex_integer()),
            Some(_) => {
                todo!("handle unexpected characters");
            }
            None => None,
        }
    }

    fn lex_identifier(&mut self) -> Token<'a> {
        let start = self.cursor.clone();
        let ident = self.read_while(|ch| ch.is_ascii_alphanumeric() || ch == b'_');
        let span = self.span_from(start);

        // Attempt to classify the identifier as a language keyword.
        // If it is not a keyword, fall back to treating it as a standard identifier.
        let token_kind =
            TokenKind::map_keyword(&ident).unwrap_or_else(|| TokenKind::Identifier(ident));

        Token::new(token_kind, span)
    }

    fn lex_integer(&mut self) -> Token<'a> {
        let start = self.cursor.clone();
        let value = self.read_while(|ch| ch.is_ascii_digit());

        if self.peek() == Some(b'.') {
            return self.lex_float(start);
        }

        let span = self.span_from(start);
        Token::new(TokenKind::IntLiteral(value), span)
    }

    fn lex_float(&mut self, start: Cursor) -> Token<'a> {
        self.advance(); // consume '.'
        self.read_while(|ch| ch.is_ascii_digit());

        let span = self.span_from(start.clone());
        let value = &self.source[span.start..span.end];

        Token::new(TokenKind::FloatLiteral(value), span)
    }
}
