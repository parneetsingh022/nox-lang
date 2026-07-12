use miette::NamedSource;

use crate::{
    diagnostic::{
        IncompleteFloatError, InvalidNumericSuffixError, LexerError, Span, UnexpectedCharError,
    },
    tokenizer::{Token, TokenKind},
};

/// Returns whether the byte is ASCII whitespace.
///
/// This includes spaces, tabs, newlines, carriage returns, and other ASCII
/// whitespace bytes recognized by [`u8::is_ascii_whitespace`].
pub fn is_whitespace(ch: char) -> bool {
    ch.is_ascii_whitespace()
}

/// Returns whether the byte can start an identifier.
///
/// Identifiers may start with an ASCII letter (`a-z`, `A-Z`) or an underscore
/// (`_`).
fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

/// Returns whether the byte can continue an identifier.
///
/// After the first character, identifiers may contain ASCII letters, digits
/// (`0-9`), or underscores (`_`).
fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// Tracks current position for lexer in source file
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
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
    fn consume(&mut self, ch: char) {
        let byte_len = ch.len_utf8();
        match ch {
            '\n' => self.newline(),
            _ => {
                self.offset += byte_len;
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
    cursor: Cursor,
    named_source: NamedSource<String>,
    // Store any errors encountered while tokenizing
    errors: Vec<LexerError>,
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
    /// Creates a lexer for the given source text.
    ///
    /// `filename` is used only for diagnostics, so errors can point back to the
    /// source file they came from.
    pub fn new(source: &'a str, filename: &'a str) -> Self {
        Self {
            source,
            cursor: Cursor::default(),
            named_source: NamedSource::new(filename, source.to_string()),
            errors: Vec::new(),
        }
    }

    /// takes all lexer errors collected so far, leaving the lexer with an empty
    /// error list.
    pub fn take_errors(&mut self) -> Vec<LexerError> {
        std::mem::take(&mut self.errors)
    }

    fn is_eof(&self) -> bool {
        self.cursor.offset >= self.source.len()
    }

    /// Returns the character at the current cursor position without consuming it.
    ///
    /// Returns `None` if the cursor is at the end of the source.
    fn peek(&self) -> Option<char> {
        self.source[self.cursor.offset..].chars().next()
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) {
        if let Some(ch) = self.peek() {
            self.cursor.consume(ch);
        }
    }

    /// Creates a span from `start` to the lexer's current cursor position.
    ///
    /// The start position is usually captured before consuming a token, while the
    /// current cursor position marks the end of that token.
    fn span_from(&self, start: Cursor) -> Span {
        Span::new(start.offset, self.cursor.offset, start.line, start.column)
    }

    /// Consumes bytes while `predicate` returns true and returns the consumed text.
    ///
    /// The returned string slice points into the original source.
    fn read_while(&mut self, predicate: impl Fn(char) -> bool) -> &'a str {
        let start = self.cursor;
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

    fn emit_error(&mut self, err: impl Into<LexerError>) {
        self.errors.push(err.into());
    }

    /// Emits an unexpected-character error and returns an error token for it.
    ///
    /// The invalid byte is consumed so lexing can continue after the error.
    fn emit_unexpected_char(&mut self, start: Cursor, ch: char) -> Token<'a> {
        self.advance();

        let span = self.span_from(start);

        self.emit_error(UnexpectedCharError {
            char: ch,
            at: span.into(),
            src: self.named_source.clone(),
        });

        Token::new(TokenKind::Unexpected, span)
    }

    /// Emits an invalid numeric suffix error and returns an error token.
    ///
    /// This consumes the full suffix so input like `123abc` is reported as one
    /// invalid token instead of separate `123` and `abc` tokens.
    fn emit_invalid_numeric_suffix(&mut self, start: Cursor) -> Token<'a> {
        // Consume the full suffix so `123abc` becomes one error token,
        // not an integer token followed by an identifier token.
        self.read_while(is_ident_continue);

        let span = self.span_from(start);

        self.emit_error(InvalidNumericSuffixError {
            at: span.into(),
            src: self.named_source.clone(),
        });

        Token::new(TokenKind::Unexpected, span)
    }

    /// Emits an incomplete-float error and returns an error token.
    ///
    /// This is used for numbers ending with a decimal point, such as `123.`.
    fn emit_incomplete_float(&mut self, start: Cursor) -> Token<'a> {
        let span = self.span_from(start);
        let source_span = span.into();

        let err = IncompleteFloatError {
            at: source_span,
            src: self.named_source.clone(),

            suggestion: source_span,
            // `span.end - 1` removes the trailing `.` from the suggestion value.
            val: self.source[span.start..span.end - 1].to_string(),
        };

        self.emit_error(err);
        Token::new(TokenKind::Unexpected, span)
    }

    fn lex_next_token(&mut self) -> Option<Token<'a>> {
        self.skip_whitespace();

        let ch = self.peek()?;
        match ch {
            _ if is_ident_start(ch) => Some(self.lex_identifier()),
            _ if ch.is_ascii_digit() => Some(self.lex_number()),

            // Potential two character symbols
            '+' => Some(self.lex_plus_or_plus_plus()),
            '-' => Some(self.lex_minus_or_minus_minus()),
            '=' => Some(self.lex_eq_or_eqeq()),
            '!' => Some(self.lex_bang_or_bangeq()),
            '<' => Some(self.lex_lt_or_lteq()),
            '>' => Some(self.lex_gt_or_gteq()),

            // Single char symbols
            '*' => Some(self.lex_single_char_tokens(TokenKind::Star)),
            '/' => Some(self.lex_single_char_tokens(TokenKind::Slash)),
            '%' => Some(self.lex_single_char_tokens(TokenKind::Percent)),
            '^' => Some(self.lex_single_char_tokens(TokenKind::Caret)),
            ';' => Some(self.lex_single_char_tokens(TokenKind::Semi)),
            ',' => Some(self.lex_single_char_tokens(TokenKind::Comma)),
            '.' => Some(self.lex_single_char_tokens(TokenKind::Dot)),
            '(' => Some(self.lex_single_char_tokens(TokenKind::OpenParen)),
            ')' => Some(self.lex_single_char_tokens(TokenKind::CloseParen)),
            '{' => Some(self.lex_single_char_tokens(TokenKind::OpenBrace)),
            '}' => Some(self.lex_single_char_tokens(TokenKind::CloseBrace)),
            '[' => Some(self.lex_single_char_tokens(TokenKind::OpenBracket)),
            ']' => Some(self.lex_single_char_tokens(TokenKind::CloseBracket)),
            invalid_char => Some(self.emit_unexpected_char(self.cursor, invalid_char)),
        }
    }

    fn lex_identifier(&mut self) -> Token<'a> {
        let start = self.cursor;
        let ident = self.read_while(is_ident_continue);
        let span = self.span_from(start);

        // Attempt to classify the identifier as a language keyword.
        // If it is not a keyword, fall back to treating it as a standard identifier.
        let token_kind = TokenKind::map_keyword(ident).unwrap_or(TokenKind::Identifier(ident));

        Token::new(token_kind, span)
    }

    /// Lexes an integer or floating-point number.
    ///
    /// If the number is followed by a `.`, lexing continues as a float.
    fn lex_number(&mut self) -> Token<'a> {
        let start = self.cursor;
        let value = self.read_while(|ch| ch.is_ascii_digit());

        // A `.` after digits means this number may be a float.
        if self.peek() == Some('.') {
            return self.lex_float(start);
        }

        // Identifiers cannot be attached directly to number literals.
        if let Some(ch) = self.peek()
            && is_ident_start(ch)
        {
            return self.emit_invalid_numeric_suffix(start);
        }
        let span = self.span_from(start);
        Token::new(TokenKind::IntLiteral(value), span)
    }

    /// Continues lexing a floating-point number after the integer part.
    ///
    /// This assumes the current byte is `.` and consumes it before reading the
    /// fractional digits.
    fn lex_float(&mut self, start: Cursor) -> Token<'a> {
        // We enter this function only after seeing `.`, so consume it first.
        self.advance();

        let rest = self.read_while(|ch| ch.is_ascii_digit());

        if rest.is_empty() {
            // A decimal point must be followed by at least one digit.
            // For example, `123.` is treated as an incomplete float.
            return self.emit_incomplete_float(start);
        }

        if let Some(ch) = self.peek()
            && is_ident_start(ch)
        {
            // A number cannot be immediately followed by an identifier-like suffix.
            // For example, `123abc` or `123.45abc` should be reported as one
            // invalid numeric token instead of separate number and identifier tokens.
            return self.emit_invalid_numeric_suffix(start);
        }

        let span = self.span_from(start);
        let value = &self.source[span.start..span.end];

        Token::new(TokenKind::FloatLiteral(value), span)
    }

    /// Lexes a single-character token.
    ///
    /// Captures the cursor position, consumes the current character by advancing,
    /// and then constructs a new token using the span from the captured start
    /// position to the new cursor position.
    fn lex_single_char_tokens(&mut self, kind: TokenKind<'a>) -> Token<'a> {
        let start = self.cursor;
        self.advance();
        Token::new(kind, self.span_from(start))
    }

    /// Lexes a `+` or `++` token.
    ///
    /// Consumes the leading `+` and looks ahead to determine if it is followed
    /// by another `+`. If so, it consumes the second character and returns
    /// a `PlusPlus` token; otherwise, it returns a `Plus` token.
    fn lex_plus_or_plus_plus(&mut self) -> Token<'a> {
        // We expect current token to be `+`, so we skip it
        let start = self.cursor;
        self.advance();

        let token_kind = if self.consume_if('+') {
            TokenKind::PlusPlus
        } else {
            TokenKind::Plus
        };

        Token::new(token_kind, self.span_from(start))
    }

    /// Lexes a `-` or `--` token.
    ///
    /// Consumes the leading `-` and looks ahead to determine if it is followed
    /// by another `-`. If so, it consumes the second character and returns
    /// a `MinusMinus` token; otherwise, it returns a `Minus` token.
    fn lex_minus_or_minus_minus(&mut self) -> Token<'a> {
        // We expect current token to be `-`, so we skip it
        let start = self.cursor;
        self.advance();

        let token_kind = if self.consume_if('-') {
            TokenKind::MinusMinus
        } else {
            TokenKind::Minus
        };

        Token::new(token_kind, self.span_from(start))
    }

    /// Lexes an `=` or `==` token.
    ///
    /// Consumes the leading `=` and looks ahead to determine if it is followed
    /// by another `=`. If so, it returns an `EqEq` token; otherwise,
    /// it returns an `Eq` token.
    fn lex_eq_or_eqeq(&mut self) -> Token<'a> {
        let start = self.cursor;
        self.advance();

        let kind = if self.consume_if('=') {
            TokenKind::EqEq
        } else {
            TokenKind::Eq
        };

        Token::new(kind, self.span_from(start))
    }

    /// Lexes a `!` or `!=` token.
    ///
    /// Consumes the `!` and checks if it is followed by `=`. If so, it returns
    /// a `BangEq` token; otherwise, it returns a `Bang` token.
    fn lex_bang_or_bangeq(&mut self) -> Token<'a> {
        let start = self.cursor;
        self.advance();

        let kind = if self.consume_if('=') {
            TokenKind::BangEq
        } else {
            TokenKind::Bang
        };

        Token::new(kind, self.span_from(start))
    }

    /// Lexes a `<` or `<=` token.
    ///
    /// Consumes the `<` and checks if it is followed by `=`. If so, it returns
    /// an `LtEq` token; otherwise, it returns an `Lt` token.
    fn lex_lt_or_lteq(&mut self) -> Token<'a> {
        let start = self.cursor;
        self.advance();

        let kind = if self.consume_if('=') {
            TokenKind::LtEq
        } else {
            TokenKind::Lt
        };

        Token::new(kind, self.span_from(start))
    }

    /// Lexes a `>` or `>=` token.
    ///
    /// Consumes the `>` and checks if it is followed by `=`. If so, it returns
    /// a `GtEq` token; otherwise, it returns a `Gt` token.
    fn lex_gt_or_gteq(&mut self) -> Token<'a> {
        let start = self.cursor;
        self.advance();

        let kind = if self.consume_if('=') {
            TokenKind::GtEq
        } else {
            TokenKind::Gt
        };

        Token::new(kind, self.span_from(start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::Keyword;

    fn assert_eof<'a>(lexer: &mut Lexer<'a>) {
        assert_eq!(lexer.next(), None);
    }

    fn assert_identifier(code: &str) {
        let mut lexer = Lexer::new(code, "main.nox");

        // Get the next token and verify it exists
        let token = lexer.next().unwrap_or_else(|| {
            panic!("Expected identifier token for input: '{}', found EOF", code)
        });

        // Verify the kind matches the input
        assert_eq!(
            token.kind,
            TokenKind::Identifier(code),
            "Lexer returned wrong token kind for input: '{}'",
            code
        );

        // Verify the lexer reached EOF
        assert_eof(&mut lexer);
    }

    fn assert_keyword(code: &str, expected: Keyword) {
        let mut lexer = Lexer::new(code, "main.nox");
        let token = lexer.next().expect("Expected a token");
        assert_eq!(token.kind, TokenKind::Keyword(expected));
        assert!(lexer.next().is_none());
    }

    fn assert_lexer_errors(code: &str, expected_variants: &[fn(&LexerError) -> bool]) {
        let mut lexer = Lexer::new(code, "main.nox");
        let _: Vec<_> = lexer.by_ref().collect();
        let errors = lexer.take_errors();
        dbg!(&errors);
        assert_eq!(
            errors.len(),
            expected_variants.len(),
            "Expected {} errors, but found {}",
            expected_variants.len(),
            errors.len()
        );

        for (i, (err, matcher)) in errors.iter().zip(expected_variants).enumerate() {
            assert!(
                matcher(err),
                "Error at index {} did not match expected pattern: {:?}",
                i,
                err
            );
        }
    }

    #[test]
    fn lexer_handles_empty_source() {
        let mut lexer = Lexer::new("   \n\t\r\n  ", "main.nox");
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_recognizes_identifiers() {
        assert_identifier("ident");
        assert_identifier("a");
        assert_identifier("Z");
        assert_identifier("underscore_ident");
        assert_identifier("_start_with_underscore");
        assert_identifier("ident123");
        assert_identifier("a_b_c_1_2_3");
        assert_identifier("__with_two_underscores");
    }

    #[test]
    fn lexer_recognizes_keywords() {
        assert_keyword("let", Keyword::Let);
        assert_keyword("const", Keyword::Const);
    }

    #[test]
    fn lexer_recognizes_positive_integers() {
        let code = "234 596 32 0";
        let mut lexer = Lexer::new(code, "main.nox");

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
        let mut lexer = Lexer::new(code, "main.nox");

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
        let mut lexer = Lexer::new(code, "main.nox");

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
        let mut lexer = Lexer::new(code, "main.nox");

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
        let mut lexer = Lexer::new(code, "main.nox");

        let t = lexer.next().unwrap();
        // 'hello' starts at 0 and ends at 5
        assert_eq!(t.span.start, 0);
        assert_eq!(t.span.end, 5);
    }

    #[test]
    fn lexer_handles_whitespace_only_source() {
        let mut lexer = Lexer::new("   \n\t\r\n  ", "main.nox");
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_recognizes_mixed_tokens() {
        let code = "let x 123 45.67 const";
        let mut lexer = Lexer::new(code, "main.nox");

        assert_eq!(lexer.next().unwrap().kind, TokenKind::Keyword(Keyword::Let));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::Identifier("x"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::IntLiteral("123"));
        assert_eq!(lexer.next().unwrap().kind, TokenKind::FloatLiteral("45.67"));
        assert_eq!(
            lexer.next().unwrap().kind,
            TokenKind::Keyword(Keyword::Const)
        );
        assert_eof(&mut lexer);
    }

    #[test]
    fn lexer_recognizes_identifier_variants() {
        let code = "_abc abc123 letx const_value";
        let mut lexer = Lexer::new(code, "main.nox");

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
        let mut lexer = Lexer::new(code, "main.nox");

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
        let mut lexer = Lexer::new(code, "main.nox");

        let t = lexer.next().unwrap();
        assert_eq!(t.kind, TokenKind::Identifier("abc"));
        assert_eq!(t.span.column, 3);
    }

    #[test]
    fn lexer_handles_crlf_newlines() {
        let code = "a\r\nb";
        let mut lexer = Lexer::new(code, "main.nox");

        let t1 = lexer.next().unwrap();
        assert_eq!(t1.span.line, 1);
        assert_eq!(t1.span.column, 1);

        let t2 = lexer.next().unwrap();
        assert_eq!(t2.span.line, 2);
        assert_eq!(t2.span.column, 1);
    }

    #[test]
    fn incomplete_float_literal_emit_error() {
        // Single error case
        assert_lexer_errors(
            "395.",
            &[|err| matches!(err, LexerError::IncompleteFloat(_))],
        );

        // Multiple errors in one string
        assert_lexer_errors(
            "395. 123.",
            &[
                |err| matches!(err, LexerError::IncompleteFloat(_)),
                |err| matches!(err, LexerError::IncompleteFloat(_)),
            ],
        );

        // Incomplete float followed by an integer
        assert_lexer_errors(
            "423. 34",
            &[|err| matches!(err, LexerError::IncompleteFloat(_))],
        );
    }

    #[test]
    fn invalid_number_suffix_raises_error() {
        assert_lexer_errors(
            "395abc 492.4adb",
            &[
                |err| matches!(err, LexerError::InvalidNumericSuffix(_)),
                |err| matches!(err, LexerError::InvalidNumericSuffix(_)),
            ],
        )
    }

    #[test]
    fn lexer_recognizes_unexpected_chars() {
        let cases = vec!["@", "#", "$", "§", "name@unexp", "@name"];

        for code in cases {
            assert_lexer_errors(code, &[|err| matches!(err, LexerError::UnexpectedChar(_))])
        }
    }

    #[test]
    fn test_math_operators() {
        let code = "+ ++ - -- * / % ^";
        let mut lexer = Lexer::new(code, "test.nox");

        let expected = vec![
            TokenKind::Plus,
            TokenKind::PlusPlus,
            TokenKind::Minus,
            TokenKind::MinusMinus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::Caret,
        ];

        for kind in expected {
            let token = lexer.next().expect("Expected token, found EOF");
            assert_eq!(token.kind, kind);
        }
    }

    #[test]
    fn test_comparison_operators() {
        let code = "= == ! != < <= > >=";
        let mut lexer = Lexer::new(code, "test.nox");

        let expected = vec![
            TokenKind::Eq,
            TokenKind::EqEq,
            TokenKind::Bang,
            TokenKind::BangEq,
            TokenKind::Lt,
            TokenKind::LtEq,
            TokenKind::Gt,
            TokenKind::GtEq,
        ];

        for kind in expected {
            let token = lexer.next().expect("Expected token, found EOF");
            assert_eq!(token.kind, kind);
        }
    }

    #[cfg(test)]
    mod span_tests {
        use super::*;

        #[test]
        fn test_operator_spans() {
            // String:       "++ -- == !="
            // Indices:       01234567890
            let code = "++ -- == !=";
            let mut lexer = Lexer::new(code, "test.nox");

            // We define the expected start/end offsets (inclusive start, exclusive end)
            let expectations = vec![
                (TokenKind::PlusPlus, 0, 2),
                (TokenKind::MinusMinus, 3, 5),
                (TokenKind::EqEq, 6, 8),
                (TokenKind::BangEq, 9, 11),
            ];

            for (kind, start, end) in expectations {
                let token = lexer.next().expect("Expected token, found EOF");

                // Verify Kind
                assert_eq!(token.kind, kind);

                // Verify Span
                assert_eq!(
                    token.span.start, start,
                    "Start offset mismatch for {:?}",
                    kind
                );
                assert_eq!(token.span.end, end, "End offset mismatch for {:?}", kind);
            }
        }
    }

    #[test]
    fn test_punctuation_and_delimiters() {
        // String: "; , . ( ) { } [ ]"
        // Indices: 0 2 4 6 8 10 12 14 16
        let code = "; , . ( ) { } [ ]";
        let mut lexer = Lexer::new(code, "test.nox");

        let expectations = vec![
            (TokenKind::Semi, 0, 1),
            (TokenKind::Comma, 2, 3),
            (TokenKind::Dot, 4, 5),
            (TokenKind::OpenParen, 6, 7),
            (TokenKind::CloseParen, 8, 9),
            (TokenKind::OpenBrace, 10, 11),
            (TokenKind::CloseBrace, 12, 13),
            (TokenKind::OpenBracket, 14, 15),
            (TokenKind::CloseBracket, 16, 17),
        ];

        for (expected_kind, start, end) in expectations {
            let token = lexer.next().expect("Expected token, found EOF");

            assert_eq!(
                token.kind, expected_kind,
                "Kind mismatch for {:?}",
                expected_kind
            );
            assert_eq!(
                token.span.start, start,
                "Start offset mismatch for {:?}",
                expected_kind
            );
            assert_eq!(
                token.span.end, end,
                "End offset mismatch for {:?}",
                expected_kind
            );
        }
    }

    #[test]
    fn test_complex_mixed_expression() {
        // String: "let x = (1 + [2 * 3]);"
        // Token sequence:
        // Keyword(Let), Identifier("x"), Assign, OpenParen, Int("1"), Plus,
        // OpenBracket, Int("2"), Star, Int("3"), CloseBracket, CloseParen, Semi
        let code = "let x = (1 + [2 * 3]);";
        let mut lexer = Lexer::new(code, "test.nox");

        let expected = vec![
            (TokenKind::Keyword(Keyword::Let), 0, 3),
            (TokenKind::Identifier("x"), 4, 5),
            (TokenKind::Eq, 6, 7),
            (TokenKind::OpenParen, 8, 9),
            (TokenKind::IntLiteral("1"), 9, 10),
            (TokenKind::Plus, 11, 12),
            (TokenKind::OpenBracket, 13, 14),
            (TokenKind::IntLiteral("2"), 14, 15),
            (TokenKind::Star, 16, 17),
            (TokenKind::IntLiteral("3"), 18, 19),
            (TokenKind::CloseBracket, 19, 20),
            (TokenKind::CloseParen, 20, 21),
            (TokenKind::Semi, 21, 22),
        ];

        for (i, (expected_kind, start, end)) in expected.into_iter().enumerate() {
            let token = lexer
                .next()
                .unwrap_or_else(|| panic!("Token at index {} missing", i));

            assert_eq!(token.kind, expected_kind, "Kind mismatch at index {}", i);
            assert_eq!(
                token.span.start, start,
                "Start span mismatch at index {}",
                i
            );
            assert_eq!(token.span.end, end, "End span mismatch at index {}", i);
        }

        assert!(lexer.next().is_none(), "Expected EOF");
    }
}
