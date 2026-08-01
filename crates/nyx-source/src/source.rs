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
        &self.contents()[span.start..span.end]
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
