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
}

use super::core::Rect;
