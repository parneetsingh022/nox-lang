use miette::SourceSpan;

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

#[cfg(test)]
pub fn assert_span(actual: Span, expected: Span) {
    assert_eq!(
        actual,
        expected,
        "unexpected span: expected bytes {}..{} at {}:{}, found bytes {}..{} at {}:{}",
        expected.start,
        expected.end,
        expected.line,
        expected.column,
        actual.start,
        actual.end,
        actual.line,
        actual.column,
    );
}
