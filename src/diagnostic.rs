use std::sync::Arc;

use miette::{Diagnostic, MietteError, NamedSource, SourceCode, SourceSpan, SpanContents};
use thiserror::Error;

use crate::lexer::TokenKind;

/// Shared source text used by the lexer, parser, and diagnostics.
///
/// The underlying [`NamedSource`] stores the source contents and filename used
/// by `miette` when rendering labeled spans and code snippets.
///
/// Wrapping it in an [`Arc`] allows the source file to be shared cheaply across
/// the lexer, parser, and multiple diagnostics without duplicating the source
/// text.
#[derive(Debug, Clone)]
pub struct SourceFile(Arc<NamedSource<String>>);

impl SourceFile {
    /// Creates a shared source file with the given name and contents.
    ///
    /// The source text is stored in a [`NamedSource`] and wrapped in an [`Arc`]
    /// so it can be cloned and shared cheaply.
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let name_str: String = name.into();
        let content_str: String = content.into();
        SourceFile(Arc::new(NamedSource::new(name_str, content_str)))
    }

    /// Returns the complete source text.
    ///
    /// The returned string slice is borrowed from this source file.
    pub fn contents(&self) -> &str {
        self.0.inner().as_str()
    }

    /// Returns the source text covered by `span`.
    ///
    /// # Panics
    ///
    /// Panics if the span is out of bounds, reversed, or does not lie on valid
    /// UTF-8 character boundaries.
    pub fn slice(&self, span: Span) -> &str {
        &self.0.inner().as_str()[span.start..span.end]
    }
}

impl SourceCode for SourceFile {
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn SpanContents<'a> + 'a>, MietteError> {
        self.0
            .read_span(span, context_lines_before, context_lines_after)
    }
}

/// Represents position of a token in the source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Byte index where this span starts.
    pub start: usize,

    /// Byte index immediately after this span ends.
    pub end: usize,

    /// 1-based line number where this span starts.
    pub line: usize,

    /// 1-based column number where this span starts.
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }

    /// Creates a span that covers the source range from the beginning of `first`
    /// through the end of `last`.
    ///
    /// The resulting span inherits its starting line and column from `first`.
    pub fn from_bounds(first: Span, last: Span) -> Self {
        debug_assert!(first.start <= last.end);

        Self {
            start: first.start,
            end: last.end,
            line: first.line,
            column: first.column,
        }
    }
}

impl From<Span> for SourceSpan {
    fn from(span: Span) -> Self {
        // Calculate the length from your end and start offsets
        let offset = span.start;
        let length = span.end - span.start;

        Self::new(offset.into(), length)
    }
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum LexerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnexpectedChar(#[from] UnexpectedCharError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidNumericSuffix(#[from] InvalidNumericSuffixError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    IncompleteFloat(#[from] IncompleteFloatError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    UnterminatedComment(#[from] UnterminatedCommentError),
}

#[derive(Error, Debug, Diagnostic)]
#[error("Unexpected character '{char}'")]
#[diagnostic(
    code(nox::unexpected_char),
    help(
        "The symbol '{char}' is not recognized by nox. Ensure your code only uses valid identifiers, operators, and syntax."
    )
)]
pub struct UnexpectedCharError {
    pub char: char,

    #[label("invalid character")]
    pub at: SourceSpan,

    // Use NamedSource to hold the context efficiently
    #[source_code]
    pub src: SourceFile,
}

#[derive(Error, Debug, Diagnostic)]
#[error("Incomplete floating-point literal")]
#[diagnostic(
    code(nox::lexer::incomplete_float),
    help("Floating-point literals require a fractional component.")
)]
pub struct IncompleteFloatError {
    #[label("this is missing a fractional part")]
    pub at: SourceSpan,

    #[label("suggested fix: {val}.0")]
    pub suggestion: SourceSpan, // Point to the same location

    pub val: String,
    #[source_code]
    pub src: SourceFile,
}

#[derive(Error, Debug, Diagnostic)]
#[error("Invalid numeric literal")]
#[diagnostic(
    code(nox::lexer::invalid_numeric_suffix),
    help("Add whitespace or an operator between the number and the identifier.")
)]
pub struct InvalidNumericSuffixError {
    #[label("number cannot be directly followed by identifier characters")]
    pub at: SourceSpan,

    #[source_code]
    pub src: SourceFile,
}

#[derive(Error, Debug, Diagnostic)]
#[error("Unterminated multi-line comment")]
#[diagnostic(
    code(nox::lexer::unterminated_comment),
    help("Multi-line comments started with '/*' must be closed with '*/'.")
)]
pub struct UnterminatedCommentError {
    #[label("this comment was never closed")]
    pub at: SourceSpan,

    #[source_code]
    pub src: SourceFile,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ParserError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnexpectedToken(#[from] UnexpectedTokenError),

    #[error(transparent)]
    #[diagnostic(transparent)]
    UnexpectedEof(#[from] UnexpectedEofError),
}

#[derive(Error, Debug, Diagnostic)]
#[error("Unexpected token '{found}'")]
#[diagnostic(
    code(nox::parser::unexpected_token),
    help("An expression was expected here, but '{found}' was encountered instead.")
)]
pub struct UnexpectedTokenError {
    pub found: TokenKind,

    #[label("unexpected token")]
    pub at: SourceSpan,

    #[source_code]
    pub src: SourceFile,
}

#[derive(Error, Debug, Diagnostic)]
#[error("unexpected end of file")]
#[diagnostic(
    code(nox::parser::unexpected_eof),
    help("check for unclosed delimiters, incomplete expressions, or trailing operators")
)]
pub struct UnexpectedEofError {
    #[label("unexpected end of file here")]
    pub at: SourceSpan,

    #[source_code]
    pub src: SourceFile,
}
