use std::sync::Arc;

use miette::{NamedSource, SourceSpan};

use crate::{
    diagnostic::{
        IncompleteFloatError, InvalidNumericSuffixError, LexerError, SourceFile, Span,
        UnexpectedCharError, UnterminatedCommentError,
    },
    tokenizer::{SymbolRegistry, Token, TokenKind},
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
    named_source: SourceFile,

    // Diagnostics that can be reported together after tokenization.
    //
    // Some lexer errors do not prevent us from continuing to scan the rest of
    // the file. For those cases, we record the diagnostic here and keep going,
    // so the user can see multiple errors at once.
    //
    // Fatal errors are different: if the lexer cannot reliably continue, such as
    // after an unterminated block comment or string, the error is returned
    // immediately as `Err` from the iterator.
    errors: Vec<LexerError>,
    pub symbol_registry: SymbolRegistry,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexerError>;

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
    pub fn new(source: &'a str, filename: impl Into<String>) -> Self {
        Self {
            source,
            cursor: Cursor::default(),
            named_source: Arc::new(NamedSource::new(filename.into(), source.to_string())),
            errors: Vec::new(),
            symbol_registry: SymbolRegistry::new(),
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

    /// Checks if the remaining source string starts with the provided pattern.
    ///
    /// This performs a non-consuming check, allowing the lexer to look ahead
    /// for multi-character tokens without advancing the internal cursor.
    ///
    /// # Arguments
    ///
    /// * `s` - The string pattern to match against the current position.
    ///
    /// # Returns
    ///
    /// * `true` if the source at the current cursor matches the pattern `s`.
    /// * `false` otherwise, or if the remaining source is shorter than `s`.
    fn starts_with(&self, s: &str) -> bool {
        self.source[self.cursor.offset..].starts_with(s)
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

    fn advance_n(&mut self, n: usize) {
        // Get the slice of the remaining source
        let remaining_source = &self.source[self.cursor.offset..];

        // Take up to n characters from the iterator
        for ch in remaining_source.chars().take(n) {
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

    fn skip_single_line_comments(&mut self) {
        if !self.starts_with("//") {
            return;
        }

        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }

            self.advance();
        }
    }

    /// Skips over multi-line comments `/* ... */`.
    ///
    /// If the comment is not terminated (EOF reached), it returns an `UnterminatedCommentError`.
    fn skip_multi_line_comment(&mut self) -> Result<(), LexerError> {
        if !self.starts_with("/*") {
            return Ok(());
        }

        let start = self.cursor;
        self.advance_n(2); // Consume "/*"

        while !self.starts_with("*/") {
            if self.is_eof() {
                // Limit the error span to just the opening "/*"
                // by setting the length to two
                let error_span = SourceSpan::new(start.offset.into(), 2);
                // Unterminated block comments are fatal because the lexer cannot reliably
                // determine where normal tokenization should resume.
                return Err(UnterminatedCommentError {
                    at: error_span,
                    src: self.named_source.clone(),
                }
                .into());
            }
            self.advance();
        }

        self.advance_n(2); // Consume "*/"

        Ok(())
    }

    fn push_diagnostic(&mut self, err: impl Into<LexerError>) {
        // Store diagnostics for errors where the lexer can still continue scanning.
        // These are printed together after tokenization finishes.
        self.errors.push(err.into());
    }

    /// Creates an `Unexpected` token for an unrecognized character.
    ///
    /// The character is consumed, and a diagnostic is stored so lexing can continue
    /// and report more errors from the same file.
    fn unexpected_char_token(&mut self, start: Cursor, ch: char) -> Token {
        self.advance();

        let span = self.span_from(start);

        self.push_diagnostic(UnexpectedCharError {
            char: ch,
            at: span.into(),
            src: self.named_source.clone(),
        });

        Token::new(TokenKind::Unexpected, span)
    }

    /// Creates an `Unexpected` token for a number with an invalid suffix.
    ///
    /// The full suffix is consumed so input like `123abc` is reported as one invalid
    /// token instead of an integer token followed by an identifier token.
    fn invalid_numeric_suffix_token(&mut self, start: Cursor) -> Token {
        // Consume the full suffix so `123abc` becomes one error token,
        // not an integer token followed by an identifier token.
        self.read_while(is_ident_continue);

        let span = self.span_from(start);

        self.push_diagnostic(InvalidNumericSuffixError {
            at: span.into(),
            src: self.named_source.clone(),
        });

        Token::new(TokenKind::Unexpected, span)
    }

    /// Creates an `Unexpected` token for an incomplete floating-point literal.
    ///
    /// This handles numbers ending with a decimal point, such as `123.`. The
    /// diagnostic includes a suggested `.0` completion.
    fn incomplete_float_token(&mut self, start: Cursor) -> Token {
        let span = self.span_from(start);
        let source_span = span.into();

        let err = IncompleteFloatError {
            at: source_span,
            src: self.named_source.clone(),

            suggestion: source_span,
            // `span.end - 1` removes the trailing `.` from the suggestion value.
            val: self.source[span.start..span.end - 1].to_string(),
        };

        self.push_diagnostic(err);
        Token::new(TokenKind::Unexpected, span)
    }

    fn lex_next_token(&mut self) -> Option<Result<Token, LexerError>> {
        loop {
            let start_offset = self.cursor.offset;

            self.skip_whitespace();
            self.skip_single_line_comments();
            if let Err(err) = self.skip_multi_line_comment() {
                return Some(Err(err));
            }

            // If the offset didn't move, current char
            // doesn't represent any whitespace or comment
            if start_offset == self.cursor.offset {
                break;
            }
        }

        let ch = self.peek()?;
        let token = match ch {
            _ if is_ident_start(ch) => self.lex_identifier(),
            _ if ch.is_ascii_digit() => self.lex_number(),

            // Double char tokens
            _ if self.starts_with("&&") => self.lex_double_char_tokens(TokenKind::And),
            _ if self.starts_with("||") => self.lex_double_char_tokens(TokenKind::Or),

            // Potential two character symbols
            '+' => self.lex_plus_or_plus_plus(),
            '-' => self.lex_minus_or_minus_minus(),
            '=' => self.lex_eq_or_eqeq(),
            '!' => self.lex_bang_or_bangeq(),
            '<' => self.lex_lt_or_lteq(),
            '>' => self.lex_gt_or_gteq(),

            // Single char symbols
            '*' => self.lex_single_char_tokens(TokenKind::Star),
            '/' => self.lex_single_char_tokens(TokenKind::Slash),
            '%' => self.lex_single_char_tokens(TokenKind::Percent),
            '^' => self.lex_single_char_tokens(TokenKind::Caret),
            ';' => self.lex_single_char_tokens(TokenKind::Semi),
            ',' => self.lex_single_char_tokens(TokenKind::Comma),
            '.' => self.lex_single_char_tokens(TokenKind::Dot),
            '(' => self.lex_single_char_tokens(TokenKind::OpenParen),
            ')' => self.lex_single_char_tokens(TokenKind::CloseParen),
            '{' => self.lex_single_char_tokens(TokenKind::OpenBrace),
            '}' => self.lex_single_char_tokens(TokenKind::CloseBrace),
            '[' => self.lex_single_char_tokens(TokenKind::OpenBracket),
            ']' => self.lex_single_char_tokens(TokenKind::CloseBracket),
            invalid_char => self.unexpected_char_token(self.cursor, invalid_char),
        };

        Some(Ok(token))
    }

    fn lex_identifier(&mut self) -> Token {
        let start = self.cursor;
        let ident = self.read_while(is_ident_continue);
        let span = self.span_from(start);

        // Attempt to classify the identifier as a language keyword.
        // If it is not a keyword, fall back to treating it as a standard identifier.
        let token_kind = TokenKind::map_keyword(ident)
            .unwrap_or(TokenKind::identifier(&mut self.symbol_registry, ident));

        Token::new(token_kind, span)
    }

    /// Lexes an integer or floating-point number.
    ///
    /// If the number is followed by a `.`, lexing continues as a float.
    fn lex_number(&mut self) -> Token {
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
            return self.invalid_numeric_suffix_token(start);
        }
        let span = self.span_from(start);
        Token::new(
            TokenKind::int_literal(&mut self.symbol_registry, value),
            span,
        )
    }

    /// Continues lexing a floating-point number after the integer part.
    ///
    /// This assumes the current byte is `.` and consumes it before reading the
    /// fractional digits.
    fn lex_float(&mut self, start: Cursor) -> Token {
        // We enter this function only after seeing `.`, so consume it first.
        self.advance();

        let rest = self.read_while(|ch| ch.is_ascii_digit());

        if rest.is_empty() {
            // A decimal point must be followed by at least one digit.
            // For example, `123.` is treated as an incomplete float.
            return self.incomplete_float_token(start);
        }

        if let Some(ch) = self.peek()
            && is_ident_start(ch)
        {
            // A number cannot be immediately followed by an identifier-like suffix.
            // For example, `123abc` or `123.45abc` should be reported as one
            // invalid numeric token instead of separate number and identifier tokens.
            return self.invalid_numeric_suffix_token(start);
        }

        let span = self.span_from(start);
        let value = &self.source[span.start..span.end];

        Token::new(
            TokenKind::float_literal(&mut self.symbol_registry, value),
            span,
        )
    }

    /// Lexes a single-character token.
    ///
    /// Captures the cursor position, consumes the current character by advancing,
    /// and then constructs a new token using the span from the captured start
    /// position to the new cursor position.
    fn lex_single_char_tokens(&mut self, kind: TokenKind) -> Token {
        let start = self.cursor;
        self.advance();
        Token::new(kind, self.span_from(start))
    }

    /// Lexes a two-character token.
    ///
    /// Captures the cursor position, consumes the next two character by advancing,
    /// and then constructs a new token using the span from the captured start
    /// position to the new cursor position.
    fn lex_double_char_tokens(&mut self, kind: TokenKind) -> Token {
        let start = self.cursor;
        self.advance_n(2);
        Token::new(kind, self.span_from(start))
    }

    /// Lexes a `+` or `++` token.
    ///
    /// Consumes the leading `+` and looks ahead to determine if it is followed
    /// by another `+`. If so, it consumes the second character and returns
    /// a `PlusPlus` token; otherwise, it returns a `Plus` token.
    fn lex_plus_or_plus_plus(&mut self) -> Token {
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
    fn lex_minus_or_minus_minus(&mut self) -> Token {
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
    fn lex_eq_or_eqeq(&mut self) -> Token {
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
    fn lex_bang_or_bangeq(&mut self) -> Token {
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
    fn lex_lt_or_lteq(&mut self) -> Token {
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
    fn lex_gt_or_gteq(&mut self) -> Token {
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
    use rstest::rstest;

    use crate::tokenizer::Keyword;

    macro_rules! assert_token {
        // Case 1: Tokens with data (Identifier, IntLiteral, etc.)
        ($lexer:expr, $kind:path, $expected_str:expr) => {
            let token = next_token($lexer);
            match token.kind {
                $kind(sym) => assert_eq!($lexer.symbol_registry.resolve(sym), $expected_str),
                other => panic!("Expected variant with data, got {:?}", other),
            }
        };
        // Case 2: Tokens without data (Keyword, Plus, etc.)
        ($lexer:expr, $kind:expr) => {
            let token = next_token($lexer);
            assert_eq!(token.kind, $kind);
        };
    }

    fn next_token<'a>(lexer: &mut Lexer<'a>) -> Token {
        lexer
            .next()
            .expect("expected token, found EOF")
            .expect("expected token, found lexer error")
    }

    fn assert_eof<'a>(lexer: &mut Lexer<'a>) {
        assert!(lexer.next().is_none());
    }

    fn assert_identifier(code: &str) {
        let mut lexer = Lexer::new(code, "main.nox");
        let token = next_token(&mut lexer);

        match token.kind {
            TokenKind::Identifier(sym) => {
                let value = lexer.symbol_registry.resolve(sym);
                assert_eq!(code, value);
            }
            other => panic!("Expected Identifier, got {:?}", other),
        }
        // Verify the lexer reached EOF
        assert_eof(&mut lexer);
    }

    fn assert_keyword(code: &str, expected: Keyword) {
        let mut lexer = Lexer::new(code, "main.nox");
        let token = next_token(&mut lexer);
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

    fn assert_kinds(code: &str, expected: Vec<TokenKind>) {
        let mut lexer = Lexer::new(code, "main.nox");

        let generated: Vec<TokenKind> = (0..expected.len())
            .map(|_| next_token(&mut lexer).kind)
            .collect();

        assert_eq!(generated, expected);
        assert_eof(&mut lexer);
    }

    fn assert_token_spans(code: &str, expected: Vec<(TokenKind, Span)>) {
        let mut lexer = Lexer::new(code, "main.nox");

        let generated: Vec<Token> = (0..expected.len())
            .map(|_| next_token(&mut lexer))
            .collect();

        assert_eof(&mut lexer);

        for (index, (generated, (expected_kind, expected_span))) in
            generated.iter().zip(expected.iter()).enumerate()
        {
            assert_eq!(
                &generated.kind, expected_kind,
                "token kind mismatch at index {index}"
            );

            assert_eq!(
                &generated.span, expected_span,
                "span mismatch for token {:?} at index {index}",
                generated.kind
            );
        }
    }

    mod token_kinds {
        use super::*;

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
            let expected = code
                .split_whitespace()
                .map(|num| TokenKind::IntLiteral(lexer.symbol_registry.store(num)))
                .collect();

            assert_kinds(code, expected);
        }

        #[test]
        fn lexer_recognizes_positive_floats() {
            let code = "234.49 4549.5239 32.39 0.0";

            let mut lexer = Lexer::new(code, "main.nox");
            let expected = code
                .split_whitespace()
                .map(|num| TokenKind::FloatLiteral(lexer.symbol_registry.store(num)))
                .collect();

            assert_kinds(code, expected);
        }

        #[test]
        fn lexer_recognizes_mixed_tokens() {
            let code = "let x 123 45.67 const";
            let mut lexer = Lexer::new(code, "main.nox");

            assert_token!(&mut lexer, TokenKind::Keyword(Keyword::Let));
            assert_token!(&mut lexer, TokenKind::Identifier, "x");
            assert_token!(&mut lexer, TokenKind::IntLiteral, "123");
            assert_token!(&mut lexer, TokenKind::FloatLiteral, "45.67");

            assert_token!(&mut lexer, TokenKind::Keyword(Keyword::Const));
            assert_eof(&mut lexer);
        }

        #[rstest]
        #[case("+", TokenKind::Plus)]
        #[case("++", TokenKind::PlusPlus)]
        #[case("-", TokenKind::Minus)]
        #[case("--", TokenKind::MinusMinus)]
        #[case("*", TokenKind::Star)]
        #[case("/", TokenKind::Slash)]
        #[case("%", TokenKind::Percent)]
        #[case("^", TokenKind::Caret)]
        fn test_individual_math_operator(#[case] input: &str, #[case] expected: TokenKind) {
            let mut lexer = Lexer::new(input, "test.nox");
            let token = next_token(&mut lexer);
            assert_eq!(token.kind, expected);
            assert_eof(&mut lexer);
        }

        #[rstest]
        #[case("=", TokenKind::Eq)]
        #[case("==", TokenKind::EqEq)]
        #[case("!", TokenKind::Bang)]
        #[case("!=", TokenKind::BangEq)]
        #[case("<", TokenKind::Lt)]
        #[case("<=", TokenKind::LtEq)]
        #[case(">", TokenKind::Gt)]
        #[case(">=", TokenKind::GtEq)]
        fn test_comparison_operators(#[case] code: &str, #[case] expected: TokenKind) {
            let mut lexer = Lexer::new(code, "test.nox");

            assert_eq!(next_token(&mut lexer).kind, expected);
            assert_eof(&mut lexer);
        }

        #[test]
        fn single_line_comments_are_excluded_from_tokens() {
            let code = r#"// This is starting comment
let x = 10; // this is comment
// let z = 20;
// last line must be excluded
print(x);
"#;

            let mut lexer = Lexer::new(code, "main.nox");

            // Assert the exact sequence of tokens, ignoring comments
            assert_token!(&mut lexer, TokenKind::Keyword(Keyword::Let));
            assert_token!(&mut lexer, TokenKind::Identifier, "x");
            assert_token!(&mut lexer, TokenKind::Eq); // Assuming this is a unit variant
            assert_token!(&mut lexer, TokenKind::IntLiteral, "10");
            assert_token!(&mut lexer, TokenKind::Semi);

            assert_token!(&mut lexer, TokenKind::Identifier, "print");
            assert_token!(&mut lexer, TokenKind::OpenParen);
            assert_token!(&mut lexer, TokenKind::Identifier, "x");
            assert_token!(&mut lexer, TokenKind::CloseParen);
            assert_token!(&mut lexer, TokenKind::Semi);

            assert_eof(&mut lexer);
        }
    }

    mod token_span {
        use super::*;

        fn s(start: usize, end: usize, line: usize, col: usize) -> Span {
            Span {
                start,
                end,
                line,
                column: col,
            }
        }

        #[test]
        fn lexer_tracks_position_correctly() {
            let code = "let\n  x";
            let mut lexer = Lexer::new(code, "main.nox");

            let t1 = next_token(&mut lexer);
            assert_eq!(t1.span, s(0, 3, 1, 1));

            let t2 = next_token(&mut lexer);
            assert_eq!(t2.span, s(6, 7, 2, 3)); // Accounts for 2 spaces of indentation
        }

        #[test]
        fn lexer_tracks_multiline_positions() {
            let code = "a\n\nb";
            let mut lexer = Lexer::new(code, "main.nox");

            let t1 = next_token(&mut lexer);
            assert_eq!(t1.span, s(0, 1, 1, 1));

            let t2 = next_token(&mut lexer);
            assert_eq!(t2.span, s(3, 4, 3, 1));
        }

        #[test]
        fn lexer_handles_whitespace_only_source() {
            let mut lexer = Lexer::new("   \n\t\r\n  ", "main.nox");
            assert_eof(&mut lexer);
        }

        #[test]
        fn lexer_tracks_span_offsets_after_whitespace() {
            let code = "  \n  hello";
            let mut lexer = Lexer::new(code, "main.nox");

            let t = next_token(&mut lexer);

            match t.kind {
                TokenKind::Identifier(symbol) => {
                    assert_eq!(lexer.symbol_registry.resolve(symbol), "hello")
                }
                kind => panic!("expected Identifier found, {:?}", kind),
            }

            assert_eq!(t.span, s(5, 10, 2, 3));
        }

        #[test]
        fn lexer_handles_tabs_before_token() {
            let code = "\t\tabc";
            let mut lexer = Lexer::new(code, "main.nox");

            let t = next_token(&mut lexer);
            match t.kind {
                TokenKind::Identifier(symbol) => {
                    assert_eq!(lexer.symbol_registry.resolve(symbol), "abc")
                }
                kind => panic!("expected Identifier found, {:?}", kind),
            }
            // Starts at 2, ends at 5, line 1, column 3
            assert_eq!(t.span, s(2, 5, 1, 3));
        }

        #[test]
        fn lexer_handles_crlf_newlines() {
            let code = "a\r\nb";
            let mut lexer = Lexer::new(code, "main.nox");

            let t1 = next_token(&mut lexer);
            assert_eq!(t1.span, s(0, 1, 1, 1));

            let t2 = next_token(&mut lexer);
            assert_eq!(t2.span, s(3, 4, 2, 1));
        }

        #[rstest]
        #[case("let", s(0, 3, 1, 1))] // No whitespace
        #[case("    let", s(4, 7, 1, 5))] // Spaces
        #[case("\t\tlet", s(2, 5, 1, 3))] // Tabs
        #[case("\n  let", s(3, 6, 2, 3))] // Newline + Spaces
        fn spans_track_columns_correctly(#[case] code: &str, #[case] expected_span: Span) {
            assert_token_spans(
                code,
                vec![(TokenKind::Keyword(Keyword::Let), expected_span)],
            );
        }

        #[rstest]
        #[case("++", TokenKind::PlusPlus, 0, 2)]
        #[case("--", TokenKind::MinusMinus, 0, 2)]
        #[case("==", TokenKind::EqEq, 0, 2)]
        #[case("!=", TokenKind::BangEq, 0, 2)]
        #[case("&&", TokenKind::And, 0, 2)]
        #[case("||", TokenKind::Or, 0, 2)]
        fn test_single_operator_span(
            #[case] code: &str,
            #[case] kind: TokenKind,
            #[case] start: usize,
            #[case] end: usize,
        ) {
            let mut lexer = Lexer::new(code, "test.nox");
            let token = next_token(&mut lexer);

            assert_eq!(token.kind, kind);
            assert_eq!(token.span.start, start);
            assert_eq!(token.span.end, end);
        }

        #[rstest]
        #[case(";", TokenKind::Semi, s(0, 1, 1, 1))]
        #[case(",", TokenKind::Comma, s(0, 1, 1, 1))]
        #[case(".", TokenKind::Dot, s(0, 1, 1, 1))]
        #[case("(", TokenKind::OpenParen, s(0, 1, 1, 1))]
        #[case(")", TokenKind::CloseParen, s(0, 1, 1, 1))]
        #[case("{", TokenKind::OpenBrace, s(0, 1, 1, 1))]
        #[case("}", TokenKind::CloseBrace, s(0, 1, 1, 1))]
        #[case("[", TokenKind::OpenBracket, s(0, 1, 1, 1))]
        #[case("]", TokenKind::CloseBracket, s(0, 1, 1, 1))]
        fn test_punctuation_and_delimiters(
            #[case] input: &str,
            #[case] kind: TokenKind,
            #[case] expected_span: Span,
        ) {
            let mut lexer = Lexer::new(input, "test.nox");
            let token = next_token(&mut lexer);

            assert_eq!(token.kind, kind);
            assert_eq!(token.span, expected_span);
            assert!(lexer.next().is_none(), "Expected EOF after delimiter");
        }

        #[test]
        fn test_complex_mixed_expression_spans() {
            let code = "let x = (1 + [2 * 3]);";
            let mut lexer = Lexer::new(code, "main.nox");

            assert_eq!(next_token(&mut lexer).span, s(0, 3, 1, 1)); // let
            assert_eq!(next_token(&mut lexer).span, s(4, 5, 1, 5)); // x
            assert_eq!(next_token(&mut lexer).span, s(6, 7, 1, 7)); // =
            assert_eq!(next_token(&mut lexer).span, s(8, 9, 1, 9)); // (
            assert_eq!(next_token(&mut lexer).span, s(9, 10, 1, 10)); // 1
            assert_eq!(next_token(&mut lexer).span, s(11, 12, 1, 12)); // +
            assert_eq!(next_token(&mut lexer).span, s(13, 14, 1, 14)); // [
            assert_eq!(next_token(&mut lexer).span, s(14, 15, 1, 15)); // 2
            assert_eq!(next_token(&mut lexer).span, s(16, 17, 1, 17)); // *
            assert_eq!(next_token(&mut lexer).span, s(18, 19, 1, 19)); // 3
            assert_eq!(next_token(&mut lexer).span, s(19, 20, 1, 20)); // ]
            assert_eq!(next_token(&mut lexer).span, s(20, 21, 1, 21)); // )
            assert_eq!(next_token(&mut lexer).span, s(21, 22, 1, 22)); // ;

            assert_eof(&mut lexer);
        }

        #[test]
        fn spans_are_correct_after_single_line_comment() {
            let code = "// comment\nlet x = 10;";
            let mut lexer = Lexer::new(code, "main.nox");

            assert_eq!(next_token(&mut lexer).span, s(11, 14, 2, 1)); // let
            assert_eq!(next_token(&mut lexer).span, s(15, 16, 2, 5)); // x
            assert_eq!(next_token(&mut lexer).span, s(17, 18, 2, 7)); // =
            assert_eq!(next_token(&mut lexer).span, s(19, 21, 2, 9)); // 10
            assert_eq!(next_token(&mut lexer).span, s(21, 22, 2, 11)); // ;

            assert_eof(&mut lexer);
        }
    }

    mod test_errors {
        use super::*;

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
    }
}
