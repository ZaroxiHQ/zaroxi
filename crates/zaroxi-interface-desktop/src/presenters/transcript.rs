mod editor_projection;
mod input_bridge;
pub mod render;
mod scene_snapshot;
#[cfg(test)]
mod tests;

pub use editor_projection::{EditorLayoutSpec, build_primitives_from_contract};
pub use input_bridge::{
    handle_arrow_down, handle_arrow_left, handle_arrow_right, handle_arrow_up, handle_backspace,
    handle_key_char, handle_mouse_click_and_place_cursor, handle_scroll_lines,
};
/// Re-export core public items so external callers keep the same API.
pub use render::ShellRenderTranscript;

/// Backward-compatible free helper that delegates to the associated function
/// on ShellRenderTranscript. Kept for test seams and callers that used the
/// previous free-function surface.
pub fn build_editor_primitives_from_lines(
    content_x: u32,
    base_y: u32,
    editor_lines: &[String],
    editor_layout: Option<&EditorLayoutSpec>,
) -> zaroxi_core_engine_scene::EditorPrimitiveSet {
    render::ShellRenderTranscript::build_editor_primitives_from_lines(
        content_x,
        base_y,
        editor_lines,
        editor_layout,
    )
}
