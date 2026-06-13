/*!
Keyboard input interpretation, modifier tracking, and mouse wheel
normalization helpers extracted from app.rs.

Phase: Rope-backed editor — arrows, backspace, delete, enter, and
printable characters now route through EditorBufferState.

Responsibilities:
- Translate winit keyboard events into editor editing operations
- Normalise MouseWheel deltas into pending scroll offsets on the composition
*/

use winit::event::MouseScrollDelta;
use winit::keyboard::{Key, NamedKey};

use zaroxi_core_engine_ui::WidgetAction;

use super::GuiApp;

/// Returns true when the editor content panel is the active focus target.
/// In the current architecture, the editor is always considered "focused"
/// for editing operations unless a modal/overlay is active.
fn editor_focused(app: &GuiApp) -> bool {
    // The editor has content (a buffer is open) and no command bar / picker is active.
    app.work_content.as_ref().and_then(|w| w.editor_body.as_ref()).is_some()
        && !app.picker_in_flight
}

/// Translate a pressed keyboard logical key into zero or more `WidgetAction`s
/// and route editing commands to the rope-backed buffer.
pub(crate) fn handle_keyboard_press(app: &mut GuiApp, logical_key: &Key) -> Vec<WidgetAction> {
    // ── Editor editing commands (only when editor has focus/content) ──
    if editor_focused(app) {
        match logical_key {
            // Cursor movement
            Key::Named(NamedKey::ArrowLeft) => {
                app.editor_buffer.clear_selection();
                app.editor_buffer.move_left();
                request_editor_redraw(app);
                return Vec::new();
            }
            Key::Named(NamedKey::ArrowRight) => {
                app.editor_buffer.clear_selection();
                app.editor_buffer.move_right();
                request_editor_redraw(app);
                return Vec::new();
            }
            Key::Named(NamedKey::ArrowUp) => {
                app.editor_buffer.clear_selection();
                app.editor_buffer.move_up();
                request_editor_redraw(app);
                return Vec::new();
            }
            Key::Named(NamedKey::ArrowDown) => {
                app.editor_buffer.clear_selection();
                app.editor_buffer.move_down();
                request_editor_redraw(app);
                return Vec::new();
            }
            Key::Named(NamedKey::Home) => {
                app.editor_buffer.clear_selection();
                app.editor_buffer.move_home();
                request_editor_redraw(app);
                return Vec::new();
            }
            Key::Named(NamedKey::End) => {
                app.editor_buffer.clear_selection();
                app.editor_buffer.move_end();
                request_editor_redraw(app);
                return Vec::new();
            }

            // Editing operations
            Key::Named(NamedKey::Backspace) => {
                app.editor_buffer.backspace();
                sync_editor_to_service(app);
                request_editor_redraw(app);
                return Vec::new();
            }
            Key::Named(NamedKey::Delete) => {
                app.editor_buffer.delete_forward();
                sync_editor_to_service(app);
                request_editor_redraw(app);
                return Vec::new();
            }
            Key::Named(NamedKey::Enter) => {
                app.editor_buffer.insert_newline();
                sync_editor_to_service(app);
                request_editor_redraw(app);
                return Vec::new();
            }

            // Printable characters
            Key::Character(text) if !app.ctrl_held => {
                // Skip control characters and empty text
                if text.is_empty() || text.chars().any(|c| c.is_control() && c != '\t') {
                    return Vec::new();
                }
                app.editor_buffer.insert_text(text.as_str());
                sync_editor_to_service(app);
                request_editor_redraw(app);
                return Vec::new();
            }

            // Tab in editor (insert tab character)
            Key::Named(NamedKey::Tab) if !app.ctrl_held => {
                app.editor_buffer.insert_text("\t");
                sync_editor_to_service(app);
                request_editor_redraw(app);
                return Vec::new();
            }

            _ => {}
        }
    }

    // ── Global keyboard shortcuts / widget navigation ──
    match logical_key {
        Key::Named(NamedKey::Tab) => {
            if let Some(ref mut tree) = app.widget_tree {
                if app.shift_held {
                    app.interaction.focus_previous(tree)
                } else {
                    app.interaction.focus_next(tree)
                }
            } else {
                Vec::new()
            }
        }
        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) => {
            if let Some(ref mut tree) = app.widget_tree {
                app.interaction.activate_focused(tree)
            } else {
                Vec::new()
            }
        }
        Key::Named(NamedKey::Escape) => {
            if let Some(ref mut tree) = app.widget_tree {
                if let Some(old) = app.interaction.focused_widget_idx {
                    tree.set_state_at(old, zaroxi_core_engine_ui::InteractionState::Normal);
                }
                app.interaction.focused_widget_idx = None;
                vec![WidgetAction::FocusChanged(None), WidgetAction::StateNeedsRedraw]
            } else {
                Vec::new()
            }
        }
        ref key if app.ctrl_held => match key {
            Key::Character(c) if c == "w" || c == "W" => {
                let wc = if let Some(comp) = app.composition.as_mut() {
                    let buf_id = comp.latest_metadata().and_then(|m| m.active_buffer.clone());
                    if let Some(ref id) = buf_id {
                        if comp.close_opened_buffer(id) {
                            Some(comp.build_work_content())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(wc) = wc {
                    app.set_work_content(wc);
                    app.needs_render = true;
                    if let Some(z) = app.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                }
                Vec::new()
            }
            Key::Character(c) if c == "c" || c == "x" => {
                let selection = app.editor_selection_range();
                if let Some(text) =
                    super::editor_interaction::copy_selected_text(&app.work_content, &selection)
                {
                    let _ = zaroxi_core_engine_clipboard::copy_text(&text);
                }
                // If Ctrl+X, also delete the selection
                if c == "x" {
                    app.editor_buffer.backspace(); // backspace handles selection deletion
                    sync_editor_to_service(app);
                    request_editor_redraw(app);
                }
                Vec::new()
            }
            Key::Character(c) if c == "v" => {
                match zaroxi_core_engine_clipboard::get_text() {
                    Ok(text) => {
                        app.editor_buffer.insert_text(&text);
                        sync_editor_to_service(app);
                        request_editor_redraw(app);
                    }
                    Err(e) => {
                        eprintln!("ZAROXI_CLIPBOARD: paste failed: {}", e);
                    }
                }
                Vec::new()
            }
            Key::Character(c) if c == "z" => {
                super::debug::gui_debug_fmt!(
                    "ZAROXI_UNDO: undo at cursor line={} col={}",
                    app.editor_cursor_line(),
                    app.editor_cursor_col()
                );
                Vec::new()
            }
            Key::Character(c) if c == "y" => {
                super::debug::gui_debug_fmt!(
                    "ZAROXI_REDO: redo at cursor line={} col={}",
                    app.editor_cursor_line(),
                    app.editor_cursor_col()
                );
                Vec::new()
            }
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Notify the workspace service about a text edit so the persisted state stays
/// in sync with the local rope. Rebuilds work_content so the render path picks
/// up the new content immediately.
fn sync_editor_to_service(app: &mut GuiApp) {
    // Rebuild work_content so the render path picks up the new content
    // immediately. The editor_buffer already has the authoritative content.
    if let Some(ref comp) = app.composition {
        let mut wc = comp.build_work_content();
        if let Some(ref mut body) = wc.editor_body {
            let new_lines: Vec<String> = app.editor_buffer.lines();
            body.lines = new_lines;
            body.cursor_line = app.editor_cursor_line();
            body.cursor_col = app.editor_cursor_col();
        }
        app.set_work_content(wc);
    }
}

/// Request a redraw for the editor after an editing operation.
fn request_editor_redraw(app: &mut GuiApp) {
    app.needs_render = true;
    if let Some(z) = app.maybe_window.as_ref() {
        let _ = z.window().request_redraw();
    }
}

/// Apply mouse-wheel delta to composition pending scroll state and trigger
/// a redraw.  Called from `window_event(MouseWheel)`.
///
/// Deltas are accumulated in logical pixels via `pending_vscroll_px` so that
/// trackpad events and wheel notches both produce smooth sub-line scrolling.
pub(crate) fn process_mouse_wheel(app: &mut GuiApp, delta: &MouseScrollDelta) {
    let scroll_px: f32 = match *delta {
        MouseScrollDelta::LineDelta(x, y) => {
            if app.shift_held {
                let h_px = x as f32 * 24.0;
                if let Some(ref mut comp) = app.composition {
                    comp.pending_hscroll_px -= h_px;
                }
            }
            y as f32 * 48.0 // 3 lines × 16 px/line
        }
        MouseScrollDelta::PixelDelta(pos) => {
            if app.shift_held {
                let h_px = pos.x as f32;
                if let Some(ref mut comp) = app.composition {
                    comp.pending_hscroll_px -= h_px;
                }
            }
            pos.y as f32
        }
    };

    if scroll_px.abs() > 0.01 {
        if let Some(ref mut comp) = app.composition {
            // Also maintain integer line-level accumulator for workspace API
            let lines = -scroll_px / 16.0;
            app.pending_scroll_frac += lines;
            let whole = app.pending_scroll_frac.trunc() as isize;
            if whole != 0 {
                comp.pending_scroll_lines += whole;
                app.pending_scroll_frac -= whole as f32;
            }
            // Pixel-level smooth accumulator
            comp.pending_vscroll_px += scroll_px;
            if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_SCROLL: wheel px_delta={:.1} pending_px={:.1} pending_lines={}",
                    scroll_px, comp.pending_vscroll_px, comp.pending_scroll_lines
                );
            }
        }
    }

    if let Some(ref mut comp) = app.composition {
        comp.pending_refresh_reason =
            Some(zaroxi_application_workspace::workspace_view::RefreshReason::CursorMoved);
    }
    if let Some(z) = app.maybe_window.as_ref() {
        app.needs_render = true;
        let _ = z.window().request_redraw();
    }
}
