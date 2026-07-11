use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

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

        Self::new(offset.into(), length.into())
    }
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
    pub src: NamedSource<String>,
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
    pub src: miette::NamedSource<String>,
}
