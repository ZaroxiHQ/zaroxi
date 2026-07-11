/// Generic UI block visual description passed into the renderer.
///
/// This struct is renderer-facing and intentionally generic: it describes a
/// rectangular UI block with optional header & content visual hints. The
/// renderer must treat this data as authoritative and not interpret semantic
/// meanings like "sidebar" or "editor".
///
/// Extended for Phase 27: corner radius, border color/width, and surface role
/// allow the renderer to produce richer geometry without tying it to
/// application-layer concepts.
///
/// Large-file viewport rendering: when `content_line_offset` is set, the
/// `content` field carries only the visible window of text lines (plus overscan),
/// not the full document. The offset tells the renderer the absolute line number
/// of the first line in `content`, so cursor positioning and scroll-adjustment
/// remain correct without iterating the full document.
#[derive(Debug, Clone)]
pub struct UiBlock {
    pub id: String,
    pub title: String,
    pub content: String,
    pub visible: bool,
    pub rect: Rect,
    pub header_color: Option<[f32; 4]>,
    pub content_color: Option<[f32; 4]>,
    /// Corner radius for this surface (for future rounded-rect rendering).
    pub corner_radius: f32,
    /// Optional border color for this surface.
    pub border_color: Option<[f32; 4]>,
    /// Border width in pixels.
    pub border_width: f32,
    /// Whether this block is a header-only structural block.
    pub header_only: bool,
    /// Optional text color override for title/body text.
    pub text_color: Option<[f32; 4]>,
    /// Optional per-span colored content. Each entry is (text, color).
    /// When present, overrides the flat `content` field for body-text rendering.
    pub content_spans: Option<Vec<(String, [f32; 4])>>,
    /// Cursor line (0-based) for rendering the editor caret.
    pub cursor_line: Option<usize>,
    /// Cursor column (0-based) for rendering the editor caret.
    pub cursor_col: Option<usize>,
    /// Whether to render a line-highlight background on the cursor line.
    pub highlight_active_line: bool,
    /// Selection range as (start_line, start_col, end_line, end_col), 0-based.
    pub selection_range: Option<(usize, usize, usize, usize)>,
    /// Editor Phase 1: optional clip/scissor rect for viewport-bounded rendering.
    /// When set, the renderer should clip this block's content to the given rect.
    pub clip_rect: Option<Rect>,
    /// Editor Phase 2: x-axis scroll offset applied to the text draw origin
    /// when clip_rect is active. Subtracted from content_x so scrolled-right
    /// content shifts into the visible viewport. Full text is preserved.
    pub content_offset_x: f32,
    /// Editor Phase 3: y-axis scroll offset applied to the text draw origin.
    /// Subtracted from the text y-position when clip_rect is active so that
    /// scrolled-down content becomes visible at the top of the viewport.
    pub content_offset_y: f32,
    /// Absolute document line number of the first line in `content`.
    /// When set, `content` is a viewport-only window of lines (not the full
    /// document). The renderer adjusts its line-y counter so cursor/selection
    /// and line-highlight positioning remain absolute. When `None`, `content`
    /// carries the full document text (backward-compatible path).
    pub content_line_offset: Option<usize>,
    /// Terminal cell background runs: `(row, start_col, run_len, color)` in
    /// cell coordinates relative to the block's content area. Drawn as filled
    /// quads in the shape pass (behind the text) so the integrated terminal can
    /// render per-cell ANSI backgrounds without a bespoke pipeline. `None` for
    /// non-terminal blocks.
    pub terminal_cell_bg: Option<Vec<TerminalCellBg>>,
    /// When true, `cursor_line`/`cursor_col` render a full-cell block cursor
    /// (terminal style) instead of the thin editor caret bar.
    pub block_cursor: bool,
}

impl Default for UiBlock {
    /// A blank, visible block with no fills, text, or editor metadata.
    ///
    /// This is the canonical "neutral" block: callers build a concrete widget by
    /// overriding only the fields they care about via struct-update syntax, e.g.
    /// `UiBlock { id, rect, header_color: Some(c), ..Default::default() }`.
    /// Keeping the defaults here means new fields added to `UiBlock` get a sane
    /// value in one place instead of at every construction site.
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            content: String::new(),
            visible: true,
            rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 },
            header_color: None,
            content_color: None,
            corner_radius: 0.0,
            border_color: None,
            border_width: 0.0,
            header_only: false,
            text_color: None,
            content_spans: None,
            cursor_line: None,
            cursor_col: None,
            highlight_active_line: false,
            selection_range: None,
            clip_rect: None,
            content_offset_x: 0.0,
            content_offset_y: 0.0,
            content_line_offset: None,
            terminal_cell_bg: None,
            block_cursor: false,
        }
    }
}

use super::core::Rect;

/// A run of terminal cells sharing one background color, in cell coordinates
/// relative to a block's content area: `(row, start_col, run_len, rgba)`.
pub type TerminalCellBg = (usize, usize, usize, [f32; 4]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_block_is_visible_and_empty() {
        let b = UiBlock::default();
        assert!(b.visible, "default block must be visible");
        assert!(b.id.is_empty());
        assert!(b.header_color.is_none());
        assert!(b.content_spans.is_none());
        assert!(!b.header_only);
        assert_eq!(b.corner_radius, 0.0);
    }

    #[test]
    fn struct_update_overrides_only_named_fields() {
        let b = UiBlock {
            id: "x".to_string(),
            header_color: Some([1.0, 0.0, 0.0, 1.0]),
            header_only: true,
            ..Default::default()
        };
        assert_eq!(b.id, "x");
        assert_eq!(b.header_color, Some([1.0, 0.0, 0.0, 1.0]));
        assert!(b.header_only);
        assert!(b.visible);
        assert!(b.content_color.is_none());
    }
}
