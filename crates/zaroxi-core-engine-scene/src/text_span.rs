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

// ---------------------------------------------------------------------------
// SpanKind color palette — canonical semantic → visual mapping
// ---------------------------------------------------------------------------

impl SpanKind {
    /// Resolve a default dark-theme color for this span kind.
    /// Colors are [r, g, b, a] in [0, 1] linear space.
    pub fn color(&self) -> [f32; 4] {
        match self {
            Self::Comment => [0.45, 0.50, 0.55, 1.0],
            Self::String => [0.75, 0.65, 0.45, 1.0],
            Self::Keyword => [0.56, 0.58, 0.93, 1.0],
            Self::Function => [0.85, 0.78, 0.55, 1.0],
            Self::Type => [0.35, 0.82, 0.85, 1.0],
            Self::Number => [0.75, 0.78, 0.62, 1.0],
            Self::Constant => [0.75, 0.78, 0.62, 1.0],
            Self::Variable => [0.85, 0.80, 0.75, 1.0],
            Self::Operator => [0.62, 0.72, 0.82, 1.0],
            Self::Attribute => [0.85, 0.78, 0.55, 1.0],
            Self::Plain => [0.85, 0.87, 0.90, 1.0],
            Self::Emphasis => [0.90, 0.88, 0.85, 1.0],
            Self::Dim => [0.45, 0.48, 0.52, 1.0],
            Self::Link => [0.35, 0.55, 0.90, 1.0],
            Self::Highlight => [0.95, 0.90, 0.45, 1.0],
        }
    }
}
