/*!
Thin text adapter for GUI-9.

This adapter reuses the existing Cosmic Text integration path that lives under
`zaroxi_core_engine_text::plain::layout_plain_lines` and the bundled monospace
metrics from `zaroxi_core_engine_font::load_bundled_monospace`.

Responsibilities:
- Layout a small set of label lines inside a given region using the project's
  established text layout path (Cosmic Text integration).
- Publish a minimal engine scene snapshot (zaroxi_core_engine_scene) so downstream
  engine backends that consume the scene can render real glyphs if available.
- Return a small set of DrawRect placeholders derived from the layout so the
  current overlay/clear_present path can display textual positions without
  requiring immediate rasterization here. This keeps the change small and
  forwards-compatible with backends that later render glyphs from the scene.

Note: This file purposely avoids creating a second text system. It reuses the
existing `zaroxi_core_engine_text` layout helpers and the bundled monospace font.
*/

use zaroxi_core_engine_font::load_bundled_monospace;
use zaroxi_core_engine_text::plain::layout_plain_lines;
use zaroxi_core_engine_scene::{EditorPrimitiveSet, TextPrimitive, ShellSceneModel};
use zaroxi_core_engine_scene;
use crate::gui::Theme;

/// Layout `lines` inside the rectangle (x,y,width,height) using the existing
/// Cosmic Text layout path and publish a minimal scene snapshot. Returns a set
/// of DrawRect placeholders (one per laid-out line) so the existing overlay
/// drawing path can show where text would appear.
///
/// This function is intentionally conservative: it does not attempt glyph
/// rasterization itself and instead delegates that responsibility to engine
/// seams that may later render the published scene.
pub fn layout_and_publish_text(
    x: u32,
    y: u32,
    _width: u32,
    _height: u32,
    lines: &[String],
    _theme: &Theme,
    // color_hex allows callers to select an appropriate contrast token from Theme.
    color_hex: &str,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    // Use the bundled monospace font metrics already present in the workspace.
    let font = load_bundled_monospace();

    // Use the existing Cosmic Text / engine text layout helper to produce
    // stable TextPrimitive positions. This is the existing integration path.
    let line_layout = layout_plain_lines(lines, &font, x, y, None);

    // Convert layout primitives into an EditorPrimitiveSet for the engine seam so
    // backends that query the scene can later render real glyphs.
    let mut set = EditorPrimitiveSet::new();
    for tp in line_layout.primitives.into_iter() {
        set.texts.push(TextPrimitive {
            x: tp.x,
            y: tp.y,
            text: tp.text,
            font_name: tp.font_name,
            max_width: tp.max_width,
        });
    }

    // Publish a minimal ShellSceneModel snapshot derived from the provided lines.
    // This mirrors the pattern used elsewhere in the presenter and enables engine
    // seams to pick up textual content without changing ownership.
    let scene_model = ShellSceneModel {
        text_lines: lines.to_vec(),
        viewport_top_line: 1,
        viewport_total_lines: lines.len() as u32,
        viewport_summary: None,
        cursor_line: None,
        cursor_column: None,
        selection_present: false,
        status_text: None,
        chrome_text: None,
        last_command: None,
        ai_status_present: false,
    };

    zaroxi_core_engine_scene::set_current_scene(scene_model);

    // Produce lightweight DrawRect placeholders for the overlay pipeline so existing
    // clear/present helpers can indicate textual regions. Use the theme border color
    // (parsed into wgpu::Color) as the placeholder glyph color for good contrast.
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let char_w = font.char_width;
    let line_h = font.line_height;

    for t in &set.texts {
        let w = (t.text.chars().count() as u32).saturating_mul(char_w).max(1);
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: t.x,
            y: t.y,
            width: w,
            height: line_h,
            // Use the caller-supplied color token for high-contrast text presentation.
            color: super::theme_adapter::parse_hex_color(color_hex),
        });
    }

    rects
}
