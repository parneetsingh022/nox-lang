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
            source,
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

        &self.source[span.start..span.end]
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
        let token_kind = TokenKind::map_keyword(ident).unwrap_or(TokenKind::Identifier(ident));

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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_eof<'a>(lexer: &mut Lexer<'a>) {
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn lexer_handles_empty_source() {
        let mut lexer = Lexer::new("");
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_recognizes_identifier() {
        let code = "ident";
        let mut lexer = Lexer::new(code);

        assert_eq!(lexer.next().unwrap().kind, TokenKind::Identifier(code));
        // Eof must return None
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_recognizes_keywords() {
        let test_cases = vec![("let", TokenKind::Let), ("const", TokenKind::Const)];

        for (expr, kind) in test_cases {
            let mut lexer = Lexer::new(expr);
            assert_eq!(lexer.next().unwrap().kind, kind);
            // Eof must return None
            assert_eof(&mut lexer);
        }
    }

    #[test]
    fn lexer_recognizes_positive_integers() {
        let code = "234 596 32 0";
        let mut lexer = Lexer::new(code);

        assert_eq!(lexer.next().unwrap().kind, TokenKind::IntLiteral("234"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::IntLiteral("596"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::IntLiteral("32"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::IntLiteral("0"));
        // Eof must return None
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_recognizes_positive_floats() {
        let code = "234.49 4549.5239 32.39 0.0";
        let mut lexer = Lexer::new(code);

        assert_eq!(
            lexer.next().unwrap().kind,
            TokenKind::FloatLiteral("234.49")
        );
        assert_eq!(
            lexer.next().unwrap().kind,
            TokenKind::FloatLiteral("4549.5239")
        );
        assert_eq!(lexer.next().unwrap().kind, TokenKind::FloatLiteral("32.39"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::FloatLiteral("0.0"));
        // Eof must return None
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_tracks_position_correctly() {
        let code = "let\n  x";
        let mut lexer = Lexer::new(code);

        let t1 = lexer.next().unwrap();
        assert_eq!(t1.span.line, 1);
        assert_eq!(t1.span.column, 1);

        let t2 = lexer.next().unwrap();
        assert_eq!(t2.span.line, 2);
        assert_eq!(t2.span.column, 3); // Accounts for 2 spaces of indentation
    }

    #[test]
    fn lexer_tracks_multiline_positions() {
        let code = "a\n\nb";
        let mut lexer = Lexer::new(code);

        let t1 = lexer.next().unwrap();
        assert_eq!(t1.span.line, 1);
        assert_eq!(t1.span.column, 1);

        let t2 = lexer.next().unwrap(); // Should be 'b'
        assert_eq!(t2.span.line, 3);
        assert_eq!(t2.span.column, 1);
    }
    #[test]
    fn lexer_tracks_span_offsets() {
        let code = "hello";
        let mut lexer = Lexer::new(code);

        let t = lexer.next().unwrap();
        // 'hello' starts at 0 and ends at 5
        assert_eq!(t.span.start, 0);
        assert_eq!(t.span.end, 5);
    }

    #[test]
    fn lexer_handles_whitespace_only_source() {
        let mut lexer = Lexer::new("   \n\t\r\n  ");
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_recognizes_mixed_tokens() {
        let code = "let x 123 45.67 const";
        let mut lexer = Lexer::new(code);

        assert_eq!(lexer.next().unwrap().kind, TokenKind::Let);
        assert_eq!(lexer.next().unwrap().kind, TokenKind::Identifier("x"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::IntLiteral("123"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::FloatLiteral("45.67"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::Const);
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_recognizes_identifier_variants() {
        let code = "_abc abc123 letx const_value";
        let mut lexer = Lexer::new(code);

        assert_eq!(lexer.next().unwrap().kind, TokenKind::Identifier("_abc"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::Identifier("abc123"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::Identifier("letx"));
        assert_eq!(
            lexer.next().unwrap().kind,
            TokenKind::Identifier("const_value")
        );
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_tracks_span_offsets_after_whitespace() {
        let code = "  \n  hello";
        let mut lexer = Lexer::new(code);

        let t = lexer.next().unwrap();

        assert_eq!(t.kind, TokenKind::Identifier("hello"));
        assert_eq!(t.span.start, 5);
        assert_eq!(t.span.end, 10);
        assert_eq!(t.span.line, 2);
        assert_eq!(t.span.column, 3);
    }

    #[test]
    fn lexer_handles_tabs_before_token() {
        let code = "\t\tabc";
        let mut lexer = Lexer::new(code);

        let t = lexer.next().unwrap();
        assert_eq!(t.kind, TokenKind::Identifier("abc"));
        assert_eq!(t.span.column, 3);
    }

    #[test]
    fn lexer_handles_crlf_newlines() {
        let code = "a\r\nb";
        let mut lexer = Lexer::new(code);

        let t1 = lexer.next().unwrap();
        assert_eq!(t1.span.line, 1);
        assert_eq!(t1.span.column, 1);

        let t2 = lexer.next().unwrap();
        assert_eq!(t2.span.line, 2);
        assert_eq!(t2.span.column, 1);
    }
}
