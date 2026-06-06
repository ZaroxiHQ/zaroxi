//! Engine-owned UiBlock construction for shell panels.
//!
//! Desktop places regions and wires up the block list but does NOT own visual
//! decisions like border policy or gutter styling. Those belong here in the
//! engine layer and are derived from `StyleTokens` and region geometry.
//!
//! Border policy: blocks emit NO borders by default. Internal panel boundaries
//! use color contrast for visual hierarchy. Explicit dividers should be placed
//! via the `Divider` primitive when needed.

use zaroxi_core_editor_gutter::GutterModel;
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
/// Delegates line‑number formatting to `GutterModel` from the `zaroxi-core-editor-gutter`
/// crate so the gutter crate owns the layout math and label policy. The block
/// constructor only wires the formatted text into a `UiBlock` with the correct
/// background color and no border — the gutter blends seamlessly into the editor.
pub fn make_gutter_block(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    line_count: usize,
    tokens: &StyleTokens,
) -> UiBlock {
    let model = GutterModel::new(w as u32);
    let gutter_text: String = (1..=line_count.max(1))
        .map(|n| model.line_number_string(n as u32))
        .collect::<Vec<_>>()
        .join("\n");

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
