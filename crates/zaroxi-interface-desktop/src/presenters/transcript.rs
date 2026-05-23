mod render;
mod editor_projection;
mod scene_snapshot;
mod input_bridge;
#[cfg(test)]
mod tests;

/// Re-export core public items so external callers keep the same API.
pub use render::ShellRenderTranscript;
pub use editor_projection::EditorLayoutSpec;
pub use input_bridge::{
    handle_key_char, handle_backspace, handle_arrow_up, handle_arrow_down, handle_arrow_left,
    handle_arrow_right, handle_scroll_lines, handle_mouse_click_and_place_cursor,
};

/// Backward-compatible free helper that delegates to the associated function
/// on ShellRenderTranscript. Kept for test seams and callers that used the
/// previous free-function surface.
pub fn build_editor_primitives_from_lines(
    content_x: u32,
    base_y: u32,
    editor_lines: &[String],
    editor_layout: Option<&EditorLayoutSpec>,
) -> zaroxi_core_engine_scene::EditorPrimitiveSet {
    render::ShellRenderTranscript::build_editor_primitives_from_lines(content_x, base_y, editor_lines, editor_layout)
}
