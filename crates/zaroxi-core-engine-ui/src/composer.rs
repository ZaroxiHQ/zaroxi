use crate::scene::RectPrimitive;
use zaroxi_core_engine_layout::layout::ShellLayout;

/// Build a simple, deterministic shell UI composed of:
/// - background (full window)
/// - top bar (fixed height)
/// - left sidebar
/// - editor area
/// - status bar
///
/// Returns a stable vector of RectPrimitive in paint order (background first).
pub fn build_shell_ui(window_w: u32, window_h: u32) -> Vec<RectPrimitive> {
    // Compute deterministic shell layout using the existing layout crate.
    let layout = ShellLayout::from_window_size(window_w, window_h);

    let mut rects: Vec<RectPrimitive> = Vec::new();

    // background (full window) — paint first
    rects.push(RectPrimitive::new(
        0.0,
        0.0,
        layout.window_size.width,
        layout.window_size.height,
        [13.0 / 255.0, 14.0 / 255.0, 17.0 / 255.0, 1.0],
    ));

    // top bar color
    rects.push(RectPrimitive::new(
        layout.titlebar.x,
        layout.titlebar.y,
        layout.titlebar.width,
        layout.titlebar.height,
        [0.18, 0.18, 0.22, 1.0],
    ));

    // sidebar color
    rects.push(RectPrimitive::new(
        layout.sidebar.x,
        layout.sidebar.y,
        layout.sidebar.width,
        layout.sidebar.height,
        [0.12, 0.12, 0.14, 1.0],
    ));

    // editor area color
    rects.push(RectPrimitive::new(
        layout.editor.x,
        layout.editor.y,
        layout.editor.width,
        layout.editor.height,
        [0.08, 0.09, 0.11, 1.0],
    ));

    // status bar color
    rects.push(RectPrimitive::new(
        layout.status_bar.x,
        layout.status_bar.y,
        layout.status_bar.width,
        layout.status_bar.height,
        [0.15, 0.15, 0.17, 1.0],
    ));

    rects
}
