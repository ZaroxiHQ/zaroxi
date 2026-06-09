/*!
Keyboard input interpretation, modifier tracking, and mouse wheel
normalization helpers extracted from app.rs.

Responsibilities:
- Translate winit keyboard events into widget-model actions
  (Tab/Enter/Escape navigation, Ctrl+W/C/V/Z/Y shortcuts)
- Normalise MouseWheel deltas into pending scroll offsets
  on the composition
*/

use winit::event::MouseScrollDelta;
use winit::keyboard::{Key, NamedKey};

use zaroxi_core_engine_ui::WidgetAction;

use super::GuiApp;

/// Translate a pressed keyboard logical key into zero or more `WidgetAction`s.
///
/// Separating this from the event loop helps keep `window_event` readable.
pub(crate) fn handle_keyboard_press(app: &mut GuiApp, logical_key: &Key) -> Vec<WidgetAction> {
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
        Key::Named(NamedKey::ArrowDown) => {
            if let Some(ref mut tree) = app.widget_tree {
                app.interaction.focus_next_explorer_item(tree)
            } else {
                Vec::new()
            }
        }
        Key::Named(NamedKey::ArrowUp) => {
            if let Some(ref mut tree) = app.widget_tree {
                app.interaction.focus_prev_explorer_item(tree)
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
                if let Some(comp) = app.composition.as_mut() {
                    let buf_id = comp.latest_metadata().and_then(|m| m.active_buffer.clone());
                    if let Some(ref id) = buf_id {
                        if comp.close_opened_buffer(id) {
                            app.work_content = Some(comp.build_work_content());
                            app.needs_render = true;
                            if let Some(z) = app.maybe_window.as_ref() {
                                let _ = z.window().request_redraw();
                            }
                        }
                    }
                }
                Vec::new()
            }
            Key::Character(c) if c == "c" || c == "x" => {
                if let Some(text) = super::editor_interaction::copy_selected_text(
                    &app.work_content,
                    &app.selection_range,
                ) {
                    let _ = zaroxi_core_engine_clipboard::copy_text(&text);
                }
                Vec::new()
            }
            Key::Character(c) if c == "v" => {
                match zaroxi_core_engine_clipboard::get_text() {
                    Ok(text) => {
                        super::debug::gui_debug_fmt!(
                            "ZAROXI_CLIPBOARD: paste at line={} col={} len={}",
                            app.editor_cursor_line,
                            app.editor_cursor_col,
                            text.len()
                        );
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
                    app.editor_cursor_line,
                    app.editor_cursor_col
                );
                Vec::new()
            }
            Key::Character(c) if c == "y" => {
                super::debug::gui_debug_fmt!(
                    "ZAROXI_REDO: redo at cursor line={} col={}",
                    app.editor_cursor_line,
                    app.editor_cursor_col
                );
                Vec::new()
            }
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Apply mouse-wheel delta to composition pending scroll state and trigger
/// a redraw.  Called from `window_event(MouseWheel)`.
pub(crate) fn process_mouse_wheel(app: &mut GuiApp, delta: &MouseScrollDelta) {
    let scroll_lines = match *delta {
        MouseScrollDelta::LineDelta(x, y) => {
            if app.shift_held {
                let h_px = x as f32 * 24.0;
                if let Some(ref mut comp) = app.composition {
                    comp.pending_hscroll_px -= h_px;
                }
                0.0
            } else {
                y as f32 * 3.0
            }
        }
        MouseScrollDelta::PixelDelta(pos) => {
            if app.shift_held {
                let h_px = pos.x as f32;
                if let Some(ref mut comp) = app.composition {
                    comp.pending_hscroll_px -= h_px;
                }
                0.0
            } else {
                pos.y as f32 / 16.0
            }
        }
    };

    if scroll_lines.abs() > 0.01 {
        let delta_lines = -scroll_lines.round() as isize;
        if let Some(ref mut comp) = app.composition {
            comp.pending_scroll_lines += delta_lines;
            if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_SCROLL: wheel vdelta={} pending={} hdelta_px={}",
                    delta_lines, comp.pending_scroll_lines, comp.pending_hscroll_px
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
