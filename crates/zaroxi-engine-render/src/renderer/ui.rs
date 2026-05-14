use crate::Rect;
use zaroxi_theme::Color;

/// Generic UI block visual description passed into the renderer.
///
/// This struct is renderer-facing and intentionally generic: it describes a
/// rectangular UI block with optional header & content visual hints. The
/// renderer must treat this data as authoritative and not interpret semantic
/// meanings like "sidebar" or "editor".
#[derive(Debug, Clone)]
pub struct UiBlock {
    pub id: String,
    pub title: String,
    pub content: String,
    pub visible: bool,
    pub rect: Rect,
    pub header_color: Option<Color>,
    pub content_color: Option<Color>,
}
