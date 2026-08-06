use miette::SourceSpan;

/// A half-open byte range within a source file.
///
/// The span represents `start..end`, where `end` is the first byte
/// immediately after the covered source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Byte index where this span starts.
    pub start: u32,

    /// Byte index immediately after this span ends.
    pub end: u32,
}

impl Span {
    /// Creates a new span covering `start..end`.
    pub const fn new(start: u32, end: u32) -> Self {
        assert!(start <= end, "span start must not exceed span end");
        Self { start, end }
    }

    /// Creates a span covering from the start of `first`
    /// through the end of `last`.
    ///
    /// The spans are expected to appear in source order.
    pub fn from_bounds(first: Self, last: Self) -> Self {
        assert!(
            first.start <= last.start,
            "spans must appear in source order"
        );

        Self {
            start: first.start,
            end: last.end,
        }
    }

    /// Returns the length of this span in bytes.
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    /// Returns whether this span covers no bytes.
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Converts the span into a range usable for indexing source text.
    pub const fn range(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl From<Span> for SourceSpan {
    fn from(span: Span) -> Self {
        // Calculate the length from your end and start offsets
        let offset = span.start as usize;
        let length = span.len() as usize;

        Self::new(offset.into(), length)
    }
}
