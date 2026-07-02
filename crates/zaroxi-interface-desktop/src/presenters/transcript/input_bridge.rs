// Lightweight input-adapter helpers (Phase 4)
//
// These small adapters provide a minimal, well-named API that other parts of
// the interface/harness can call to update the engine-owned scene seam.
// They simply forward to the engine-scene helpers added for Phase 4 and keep
// interface-side wiring explicit and testable without importing internal
// engine modules everywhere.

pub fn handle_key_char(ch: char) {
    zaroxi_core_engine_scene::insert_char(ch);
}

pub fn handle_backspace() {
    zaroxi_core_engine_scene::backspace();
}

pub fn handle_arrow_up() {
    // move cursor up one line
    zaroxi_core_engine_scene::move_cursor(-1, 0);
}

pub fn handle_arrow_down() {
    // move cursor down one line
    zaroxi_core_engine_scene::move_cursor(1, 0);
}

pub fn handle_arrow_left() {
    // move cursor left one column
    zaroxi_core_engine_scene::move_cursor(0, -1);
}

pub fn handle_arrow_right() {
    // move cursor right one column
    zaroxi_core_engine_scene::move_cursor(0, 1);
}

pub fn handle_scroll_lines(delta: i32) {
    zaroxi_core_engine_scene::scroll_by_lines(delta);
}

/// Map a window-space mouse click into the engine scene cursor and publish it.
///
/// This wrapper mirrors the presenter's click->cursor mapping assumptions:
/// content_x/base_y are absolute window-space origins provided by the presenter
/// and char_w/line_h should match the presenter's deterministic metrics used
/// when emitting transcripts.
pub fn handle_mouse_click_and_place_cursor(
    click_x: u32,
    click_y: u32,
    content_x: u32,
    base_y: u32,
    char_w: u32,
    line_h: u32,
    content_inset: u32,
) {
    zaroxi_core_engine_scene::place_cursor_from_click(
        click_x,
        click_y,
        content_x,
        base_y,
        char_w,
        line_h,
        content_inset,
    );
}
