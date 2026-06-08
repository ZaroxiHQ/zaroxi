/*!
Editor Phase 1 — Editor viewport definition.

The `EditorViewport` is the single source of truth for the editor content area
boundaries. Every projection, hit-test, and rendering operation that concerns
the editor text region must use this viewport's rects rather than recomputing
their own dimensions.

The `clip_rect` is the strict scissor boundary: no text, gutter, caret,
selection, or decoration may paint outside this rect.
*/
use zaroxi_interface_theme::theme::DesignTokens;

/// Defines the visible editor content region and its strict clip boundary.
///
/// `content_rect` is the raw content area rectangle (full editor body panel).
/// `clip_rect` is the content area inset by padding — text must not paint
/// outside this boundary.
#[derive(Debug, Clone, Copy)]
pub struct EditorViewport {
    pub content_rect: (f32, f32, f32, f32),
    pub clip_rect: (f32, f32, f32, f32),
    pub content_inset_x: f32,
    pub content_inset_y: f32,
}

impl EditorViewport {
    /// Construct from the editor content rect computed by the layout.
    ///
    /// The clip rect is the content rect inset by the standard content padding
    /// from DesignTokens (8px horizontal, 4px vertical).
    pub fn from_content_rect(content_rect: (f32, f32, f32, f32)) -> Self {
        let dt = DesignTokens::default();
        let inset_x = dt.spacing_sm; // 8.0
        let inset_y = dt.spacing_xs as f32; // 4.0

        let clip = (
            content_rect.0 + inset_x,
            content_rect.1 + inset_y,
            (content_rect.2 - inset_x * 2.0).max(0.0),
            (content_rect.3 - inset_y * 2.0).max(0.0),
        );

        Self { content_rect, clip_rect: clip, content_inset_x: inset_x, content_inset_y: inset_y }
    }

    /// Returns `true` if the given window-space point (px, py) falls inside
    /// the content rect (before inset).
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.content_rect.0
            && py >= self.content_rect.1
            && px < self.content_rect.0 + self.content_rect.2
            && py < self.content_rect.1 + self.content_rect.3
    }

    /// Returns `true` if the given window-space point falls inside the clip rect.
    pub fn clip_contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.clip_rect.0
            && py >= self.clip_rect.1
            && px < self.clip_rect.0 + self.clip_rect.2
            && py < self.clip_rect.1 + self.clip_rect.3
    }
}
