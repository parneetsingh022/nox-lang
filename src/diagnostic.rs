use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

/// Shared source text used by diagnostics.
///
/// Each lexer diagnostic needs access to the original source so `miette` can
/// print labeled spans. `Arc` keeps cloning cheap when multiple diagnostics
/// refer to the same file.
pub type SourceFile = Arc<NamedSource<String>>;

/// Represents position of a token in the source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
