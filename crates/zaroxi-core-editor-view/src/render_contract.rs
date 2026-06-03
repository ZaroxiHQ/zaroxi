//! Editor render contract: formalizes the boundary between editor state and
//! the rendering pipeline.
//!
//! Design:
//! - `EditorRenderContract` carries pure input data (no layout, no geometry).
//! - `EditorRenderMetrics` carries deterministic font/layout constants.
//! - Together they define everything needed to project editor primitives.
//!
//! These types are intentionally app-neutral: they describe a cursor-and-text
//! editing surface without any IDE-specific concepts.

/// Formal input contract for the editor rendering pipeline.
///
/// Carries only semantic, non-visual data: visible lines, viewport position,
/// and optional cursor/selection state. No layout, coordinates, or rendering
/// resources are present.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorRenderContract {
    /// Visible text lines (top-to-bottom).
    pub visible_lines: Vec<String>,

    /// 1-based index of the topmost visible document line.
    pub top_line: u32,

    /// Optional cursor line (1-based).
    pub cursor_line: Option<u32>,

    /// Optional cursor column (0-based).
    pub cursor_column: Option<u32>,

    /// Optional selection range (start_line, start_col, end_line, end_col)
    /// with 1-based lines and 0-based columns.
    pub selection: Option<(u32, u32, u32, u32)>,
}

impl EditorRenderContract {
    pub fn new(
        visible_lines: Vec<String>,
        top_line: u32,
        cursor_line: Option<u32>,
        cursor_column: Option<u32>,
        selection: Option<(u32, u32, u32, u32)>,
    ) -> Self {
        Self { visible_lines, top_line, cursor_line, cursor_column, selection }
    }

    /// An absent contract: no text, no cursor, no selection.
    pub fn absent() -> Self {
        Self {
            visible_lines: Vec::new(),
            top_line: 1,
            cursor_line: None,
            cursor_column: None,
            selection: None,
        }
    }

    /// True when the contract carries at least one visible line.
    pub fn has_content(&self) -> bool {
        !self.visible_lines.is_empty()
    }

    /// Number of visible lines in the contract.
    pub fn visible_line_count(&self) -> u32 {
        self.visible_lines.len() as u32
    }
}

/// Deterministic font/layout metrics used for editor primitive projection.
///
/// These constants define a monospace grid: character width, line height,
/// gutter width, and the content inset applied by presenters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorRenderMetrics {
    /// Width of a single monospace character in pixels.
    pub char_width: u32,

    /// Height of a single text line in pixels.
    pub line_height: u32,

    /// Width of the gutter region in pixels.
    pub gutter_width: u32,

    /// Content inset (padding) in pixels from the content rect edge.
    pub content_inset: u32,
}

impl EditorRenderMetrics {
    /// Default deterministic metrics matching the presenter's current constants.
    pub const DEFAULT: Self =
        Self { char_width: 8, line_height: 16, gutter_width: 48, content_inset: 6 };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_preserves_input_fields() {
        let c = EditorRenderContract::new(
            vec!["hello".into(), "world".into()],
            5,
            Some(6),
            Some(3),
            Some((4, 0, 6, 10)),
        );
        assert!(c.has_content());
        assert_eq!(c.visible_line_count(), 2);
        assert_eq!(c.top_line, 5);
        assert_eq!(c.cursor_line, Some(6));
        assert_eq!(c.cursor_column, Some(3));
        assert_eq!(c.selection, Some((4, 0, 6, 10)));
    }

    #[test]
    fn absent_contract_is_empty() {
        let c = EditorRenderContract::absent();
        assert!(!c.has_content());
        assert_eq!(c.visible_line_count(), 0);
        assert_eq!(c.cursor_line, None);
    }

    #[test]
    fn default_metrics_are_stable() {
        let m = EditorRenderMetrics::DEFAULT;
        assert_eq!(m.char_width, 8);
        assert_eq!(m.line_height, 16);
        assert_eq!(m.gutter_width, 48);
        assert_eq!(m.content_inset, 6);
    }
}
