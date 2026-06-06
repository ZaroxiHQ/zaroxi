//! Engine-owned UiBlock construction for shell panels.
//!
//! Desktop places regions and wires up the block list but does NOT own visual
//! decisions like border policy or gutter styling. Those belong here in the
//! engine layer and are derived from `StyleTokens` and region geometry.
//!
//! Border policy: blocks emit NO borders by default. Internal panel boundaries
//! use color contrast for visual hierarchy. Explicit dividers should be placed
//! via the `Divider` primitive when needed.

use zaroxi_core_engine_render::UiBlock;
use zaroxi_core_engine_style::StyleTokens;

/// Build a plain filled panel block with no border.
pub fn make_panel_block(
    id: &str,
    title: &str,
    content: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    header_color: Option<[f32; 4]>,
    content_color: Option<[f32; 4]>,
    text_color: Option<[f32; 4]>,
    header_only: bool,
) -> UiBlock {
    UiBlock {
        id: id.to_string(),
        title: title.to_string(),
        content: content.to_string(),
        visible: true,
        rect: zaroxi_core_engine_render::Rect { x, y, w, h },
        header_color,
        content_color,
        corner_radius: 0.0,
        border_color: None,
        border_width: 0.0,
        header_only,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color,
    }
}

/// Build the editor gutter lane block.
///
/// The gutter uses the same background as the editor surface so it blends
/// seamlessly. Line numbers in `text_faint` provide the only visual distinction.
/// No border is emitted — the gutter is integrated into the editor column.
pub fn make_gutter_block(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    line_count: usize,
    tokens: &StyleTokens,
) -> UiBlock {
    let gutter_text =
        (1..=line_count.max(1)).map(|n| format!("{:>4}", n)).collect::<Vec<_>>().join("\n");

    UiBlock {
        id: "gutter_lane".to_string(),
        title: String::new(),
        content: gutter_text,
        visible: true,
        rect: zaroxi_core_engine_render::Rect { x, y, w, h },
        header_color: Some(tokens.editor_gutter_bg.to_array()),
        content_color: Some(tokens.editor_gutter_bg.to_array()),
        corner_radius: 0.0,
        border_color: None,
        border_width: 0.0,
        header_only: false,
        content_spans: None,
        cursor_line: None,
        cursor_col: None,
        highlight_active_line: false,
        selection_range: None,
        text_color: Some(tokens.text_faint.to_array()),
    }
}
