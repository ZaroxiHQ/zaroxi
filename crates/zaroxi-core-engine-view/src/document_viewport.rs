//! Generic document viewport contract for the engine extraction seam.
//!
//! These types define an app-neutral contract between interface adapters and
//! engine rendering. They carry only semantic, non-visual facts about what
//! should appear on screen, without any IDE-specific concepts.
//!
//! Design rules:
//! - No IDE names (tab, editor, explorer, terminal, AI, workspace, buffer).
//! - No layout geometry (coordinates, sizes).
//! - No colors, fonts, or rendering resources.
//! - Pure semantic description of text, selection, caret, and scroll state.

/// Describes which part of a document is currently visible through the viewport.
///
/// Models a sliding window over a linear document of lines. `top_line` is 1-based
/// (consistent with text-editor conventions), and `total_lines` is the full
/// document size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentViewport {
    /// 1-based index of the topmost visible line.
    pub top_line: u32,

    /// Total number of lines in the underlying document.
    pub total_lines: u32,

    /// Visible line content (cloned strings).
    pub visible_lines: Vec<String>,

    /// Opaque compact summary string (e.g. "Ln 5/120").
    pub summary: Option<String>,
}

impl DocumentViewport {
    pub fn new(
        top_line: u32,
        total_lines: u32,
        visible_lines: Vec<String>,
        summary: Option<String>,
    ) -> Self {
        Self { top_line, total_lines, visible_lines, summary }
    }

    pub fn absent() -> Self {
        Self { top_line: 0, total_lines: 0, visible_lines: Vec::new(), summary: None }
    }
}

/// Logical position of a text caret (cursor) in a document.
///
/// The line is 1-based and column is 0-based, matching conventional
/// text-editor indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaretModel {
    pub line: u32,
    pub column: u32,
}

impl CaretModel {
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

/// Selection range within a document, expressed as start/end line-column pairs.
///
/// Both line indices are 0-based and columns are 0-based. The range is
/// inclusive: the selection covers from the character at `start` up to
/// (but not including) the character at `end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionModel {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

impl SelectionModel {
    pub fn new(start_line: u32, start_column: u32, end_line: u32, end_column: u32) -> Self {
        Self { start_line, start_column, end_line, end_column }
    }
}

/// Scroll state describing the viewport position within the document.
///
/// Carries just enough information for a proportional scrollbar: the
/// viewport's current offset and the total scrollable extent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollModel {
    /// 1-based top line index currently visible.
    pub top_line: u32,

    /// Total number of lines in the document.
    pub total_lines: u32,

    /// Number of lines currently visible in the viewport (capacity).
    pub visible_line_count: u32,
}

impl ScrollModel {
    pub fn new(top_line: u32, total_lines: u32, visible_line_count: u32) -> Self {
        Self { top_line, total_lines, visible_line_count }
    }

    /// Proportion of the document currently visible, in [0.0, 1.0].
    pub fn viewport_ratio(&self) -> f32 {
        if self.total_lines == 0 {
            return 1.0;
        }
        (self.visible_line_count as f32 / self.total_lines as f32).clamp(0.0, 1.0)
    }

    /// Proportional scroll position, in [0.0, 1.0].
    pub fn scroll_proportion(&self) -> f32 {
        let max_top = self.total_lines.saturating_sub(self.visible_line_count).max(1);
        if max_top == 0 {
            return 0.0;
        }
        ((self.top_line.saturating_sub(1)) as f32 / max_top as f32).clamp(0.0, 1.0)
    }
}

/// Complete rendered document state, combining viewport, text content,
/// optional caret, selection, and scroll information.
///
/// This is the primary hand-off type from adapters to engine render paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedDocument {
    pub viewport: DocumentViewport,
    pub caret: Option<CaretModel>,
    pub selection: Option<SelectionModel>,
    pub scroll: Option<ScrollModel>,
}

impl RenderedDocument {
    pub fn new(viewport: DocumentViewport) -> Self {
        Self { viewport, caret: None, selection: None, scroll: None }
    }

    pub fn absent() -> Self {
        Self { viewport: DocumentViewport::absent(), caret: None, selection: None, scroll: None }
    }
}
