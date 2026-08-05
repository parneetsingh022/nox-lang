use std::sync::Arc;

use miette::{MietteError, NamedSource, SourceCode, SourceSpan, SpanContents};

use crate::Span;

/// Shared source text used by the lexer, parser, and diagnostics.
///
/// The underlying [`NamedSource`] stores the source contents and filename used
/// by `miette` when rendering labeled spans and code snippets.
///
/// Wrapping it in an [`Arc`] allows the source file to be shared cheaply across
/// the lexer, parser, and multiple diagnostics without duplicating the source
/// text.
#[derive(Debug, Clone)]
pub struct SourceFile(Arc<SourceFileInner>);

#[derive(Debug)]
struct SourceFileInner {
    source: NamedSource<String>,
    line_starts: Vec<u32>,
}

impl SourceFile {
    /// Creates a shared source file with the given name and contents.
    ///
    /// The source text is stored in a [`NamedSource`] and wrapped in an [`Arc`]
    /// so it can be cloned and shared cheaply.
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let name = name.into();
        let content = content.into();

        let mut line_starts = vec![0];

        u32::try_from(content.len()).expect("source file exceeds the maximum supported size");

        for (offset, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(
                    u32::try_from(offset + 1)
                        .expect("source file exceeds the maximum supported size"),
                );
            }
        }

        Self(Arc::new(SourceFileInner {
            source: NamedSource::new(name, content),
            line_starts,
        }))
    }

    /// Returns the complete source text.
    ///
    /// The returned string slice is borrowed from this source file.
    pub fn contents(&self) -> &str {
        self.0.source.inner().as_str()
    }

    /// Returns the source text covered by `span`.
    ///
    /// # Panics
    ///
    /// Panics if the span is out of bounds, reversed, or does not lie on valid
    /// UTF-8 character boundaries.
    pub fn slice(&self, span: Span) -> &str {
        &self.contents()[span.range()]
    }

    /// Returns the one-based line and character column for a byte offset.
    ///
    /// # Panics
    ///
    /// Panics if `offset` is out of bounds or is not a UTF-8 character boundary.
    pub fn location(&self, offset: u32) -> Location {
        let offset = offset as usize;

        assert!(
            offset <= self.contents().len(),
            "source offset is out of bounds"
        );

        assert!(
            self.contents().is_char_boundary(offset),
            "source offset is not a UTF-8 character boundary"
        );

        let line_index = match self.0.line_starts.binary_search(&(offset as u32)) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };

        let line_start = self.0.line_starts[line_index] as usize;

        let column = self.contents()[line_start..offset].chars().count() + 1;

        Location {
            line: u32::try_from(line_index + 1).expect("source contains too many lines"),
            column: u32::try_from(column).expect("source line is too long"),
        }
    }

    pub fn line_of(&self, offset: u32) -> u32 {
        self.location(offset).line
    }

    pub fn same_line(&self, first: u32, second: u32) -> bool {
        self.line_of(first) == self.line_of(second)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// One-based line number.
    pub line: u32,

    /// One-based character column.
    pub column: u32,
}

impl SourceCode for SourceFile {
    fn read_span<'a>(
        &'a self,
        span: &SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn SpanContents<'a> + 'a>, MietteError> {
        self.0
            .source
            .read_span(span, context_lines_before, context_lines_after)
    }
}
