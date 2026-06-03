//! App-neutral text span primitives for the engine extraction seam.
//!
//! Carries semantic span classifications (syntax highlighting categories and
//! generic text decorations) without any IDE-specific concepts.

/// Semantic classification for a text span.
///
/// These are intentionally generic text categories that any text-heavy
/// application can produce (source code viewer, plain text reader, log viewer,
/// notes app). No app-specific concepts (IDE, terminal, AI) are present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanKind {
    // -- syntax highlighting categories (universal) --
    Comment,
    String,
    Keyword,
    Function,
    Type,
    Number,
    Constant,
    Variable,
    Operator,
    Attribute,

    // -- generic decoration categories --
    Plain,
    Emphasis,
    Dim,
    Link,
    Highlight,
}

/// A contiguous annotated span within a single text line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSpan {
    /// 0-based start column within the line.
    pub start_column: u32,

    /// 0-based end column (exclusive) within the line.
    pub end_column: u32,

    /// Semantic category for this span.
    pub kind: SpanKind,
}

impl TextSpan {
    pub fn new(start_column: u32, end_column: u32, kind: SpanKind) -> Self {
        Self { start_column, end_column, kind }
    }
}

/// Collection of spans for a single text line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxSpan {
    /// 0-based line index within the viewport.
    pub line_index: u32,

    /// Ordered, non-overlapping spans on this line.
    pub spans: Vec<TextSpan>,
}

impl SyntaxSpan {
    pub fn new(line_index: u32, spans: Vec<TextSpan>) -> Self {
        Self { line_index, spans }
    }
}
