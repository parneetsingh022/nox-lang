use miette::SourceSpan;

/// A half-open range within a source file.
///
/// Byte offsets use `start..end`, where `end` is the first byte after the span.
/// Line and column values are 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Byte index where this span starts.
    pub start: usize,

    /// Byte index immediately after this span ends.
    pub end: usize,

    /// Line where this span starts.
    pub start_line: usize,

    /// Column where this span starts.
    pub start_column: usize,

    /// Line where this span ends.
    pub end_line: usize,

    /// Column immediately after this span ends.
    pub end_column: usize,
}

impl Span {
    /// Creates a span with explicit start and end positions.
    pub const fn new(
        start: usize,
        end: usize,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            start,
            end,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// Creates a span contained entirely on one line.
    pub const fn single_line(
        start: usize,
        end: usize,
        line: usize,
        start_column: usize,
        end_column: usize,
    ) -> Self {
        Self::new(start, end, line, start_column, line, end_column)
    }

    /// Creates a span that covers the source range from the beginning of `first`
    /// through the end of `last`.
    ///
    /// The resulting span inherits its starting line and column from `first`.
    pub fn from_bounds(first: Self, last: Self) -> Self {
        debug_assert!(first.start <= last.end);

        Self {
            start: first.start,
            end: last.end,
            start_line: first.start_line,
            start_column: first.start_column,
            end_line: last.end_line,
            end_column: last.end_column,
        }
    }

    pub const fn is_multiline(self) -> bool {
        self.start_line != self.end_line
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
