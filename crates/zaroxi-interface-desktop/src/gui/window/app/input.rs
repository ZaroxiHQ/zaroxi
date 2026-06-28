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

/// Current explorer search query (from composition; empty when none).
fn current_explorer_query(app: &GuiApp) -> String {
    app.composition.as_ref().map(|c| c.explorer_search_query.clone()).unwrap_or_default()
}

/// Apply a new explorer search query: recompute the (filtered) tree, rebuild
/// work content, reset scroll to the top, and request a redraw. This is the
/// single entry point that keeps the filter, rendered list, and widget tree in
/// agreement.
fn set_explorer_search(app: &mut GuiApp, query: String) {
    let new_wc = if let Some(comp) = app.composition.as_mut() {
        comp.set_explorer_search_query(query);
        Some(comp.build_work_content())
    } else {
        None
    };
    if let Some(wc) = new_wc {
        app.work_content = Some(wc);
    }
    app.explorer_scroll_top = 0;
    // The result set changed: drop the keyboard selection and keep the caret
    // solid (reset its blink phase) while the user is typing.
    app.explorer_search_sel = None;
    app.explorer_caret_blink_epoch = std::time::Instant::now();
    app.invalidate(super::InvalidationFlags::content());
}

/// Move the keyboard selection within the filtered explorer list and scroll it
/// into view. `delta` is +1 (down) or -1 (up).
fn move_explorer_selection(app: &mut GuiApp, delta: isize) {
    let len = app.composition.as_ref().map(|c| c.cached_explorer_items.len()).unwrap_or(0);
    if len == 0 {
        return;
    }
    let next = match app.explorer_search_sel {
        // First move lands on the top visible row.
        None => app.explorer_scroll_top.min(len - 1),
        Some(cur) => (cur as isize + delta).clamp(0, len as isize - 1) as usize,
    };
    app.explorer_search_sel = Some(next);

    // Scroll into view using the last-rendered visible row count.
    let visible = app.explorer_visible_rows.max(1);
    if next < app.explorer_scroll_top {
        app.explorer_scroll_top = next;
    } else if next >= app.explorer_scroll_top + visible {
        app.explorer_scroll_top = next + 1 - visible;
    }
    app.invalidate(super::InvalidationFlags::content());
}

/// Translate a pressed keyboard logical key into zero or more `WidgetAction`s
/// and route editing commands to the rope-backed buffer.
/// Classify a key press for `ZAROXI_PERF_TRACE` event labelling, but only when
/// the editor is focused (so global shortcuts aren't mislabelled). Returns
/// `Some("edit")` for content-mutating keys, `Some("cursor_move")` for caret
/// movement, and `None` otherwise. Mirrors the dispatch in
/// [`handle_keyboard_press`]; used purely for instrumentation.
pub(crate) fn classify_editor_key(app: &GuiApp, logical_key: &Key) -> Option<&'static str> {
    if !editor_focused(app) {
        return None;
    }
    match logical_key {
        Key::Named(NamedKey::ArrowLeft)
        | Key::Named(NamedKey::ArrowRight)
        | Key::Named(NamedKey::ArrowUp)
        | Key::Named(NamedKey::ArrowDown)
        | Key::Named(NamedKey::Home)
        | Key::Named(NamedKey::End) => Some("cursor_move"),
        Key::Named(NamedKey::Backspace)
        | Key::Named(NamedKey::Delete)
        | Key::Named(NamedKey::Enter)
        | Key::Named(NamedKey::Space) => Some("edit"),
        Key::Character(text) if !app.ctrl_held => {
            if text.is_empty() || text.chars().any(|c| c.is_control() && c != '\t') {
                None
            } else {
                Some("edit")
            }
        }
        Key::Named(NamedKey::Tab) if !app.ctrl_held => Some("edit"),
        _ => None,
    }
}

pub(crate) fn handle_keyboard_press(app: &mut GuiApp, logical_key: &Key) -> Vec<WidgetAction> {
    // ── Explorer search box has keyboard focus: route typing to the filter ──
    if app.explorer_search_active {
        match logical_key {
            Key::Named(NamedKey::Escape) => {
                app.explorer_search_active = false;
                set_explorer_search(app, String::new());
            }
            Key::Named(NamedKey::ArrowDown) => {
                move_explorer_selection(app, 1);
            }
            Key::Named(NamedKey::ArrowUp) => {
                move_explorer_selection(app, -1);
            }
            Key::Named(NamedKey::Enter) => {
                // Open/toggle the keyboard-selected row (reusing the normal
                // activation path), then release focus. The filter persists.
                let sel = app.explorer_search_sel.take();
                app.explorer_search_active = false;
                if let Some(idx) = sel {
                    let id = zaroxi_core_engine_ui::WidgetId::list_item(10 + idx);
                    app.handle_actions(vec![zaroxi_core_engine_ui::WidgetAction::Activated(id)]);
                } else {
                    app.invalidate(super::InvalidationFlags::content());
                }
            }
            Key::Named(NamedKey::Backspace) => {
                let mut q = current_explorer_query(app);
                q.pop();
                set_explorer_search(app, q);
            }
            Key::Named(NamedKey::Space) => {
                let mut q = current_explorer_query(app);
                q.push(' ');
                set_explorer_search(app, q);
            }
            Key::Character(text) if !app.ctrl_held => {
                if !text.is_empty() && !text.chars().any(|c| c.is_control()) {
                    let mut q = current_explorer_query(app);
                    q.push_str(text.as_str());
                    set_explorer_search(app, q);
                }
            }
            _ => {}
        }
        // Search focus is exclusive: swallow all keys so the editor never sees them.
        return Vec::new();
    }

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
            Key::Named(NamedKey::Space) => {
                app.editor_buffer.insert_text(" ");
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
            // An active filter is the first thing Escape clears, even when the
            // search box doesn't hold keyboard focus.
            if !current_explorer_query(app).is_empty() {
                set_explorer_search(app, String::new());
                return vec![WidgetAction::StateNeedsRedraw];
            }
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
            // Ctrl+Shift+P: print the consolidated performance dashboard.
            Key::Character(c) if (c == "p" || c == "P") && app.shift_held => {
                app.dashboard();
                Vec::new()
            }
            // Ctrl+Shift+M: print a focused memory breakdown (RSS, VSZ,
            // glyph caches, buffers, wgpu limits, pressure, rope size).
            Key::Character(c) if (c == "m" || c == "M") && app.shift_held => {
                app.memory_report();
                Vec::new()
            }
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
                    app.request_open(wc);
                    app.invalidate(super::InvalidationFlags::content());
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

/// Update the visible work_content from the rope-backed editor buffer.
///
/// Distinguishes two edit classes:
/// - Content-only edit (line count unchanged): incremental update of only
///   the affected lines.  Fast path for normal single-character typing.
/// - Structural edit (line count changed, e.g. Enter, backspace-at-start):
///   full rebuild of work_content.lines from the rope.  Slower fallback
///   that guarantees correctness when line indices shift.
///
/// Tabs are expanded to spaces so cosmic-text rendering matches the column
/// model used for caret positioning and mouse hit-testing.
fn sync_editor_to_service(app: &mut GuiApp) {
    let cursor_line = app.editor_cursor_line();
    let cursor_col = app.editor_buffer.caret_vis_col();
    // Capture body line count before sync for structural-edit delta
    // computation in the large-file sync block.
    let pre_sync_body_line_count = app
        .work_content
        .as_ref()
        .and_then(|wc| wc.editor_body.as_ref())
        .map(|b| b.lines.len())
        .unwrap_or(0);

    if let Some(ref mut wc) = app.work_content
        && let Some(ref mut body) = wc.editor_body
    {
        body.cursor_line = cursor_line;
        body.cursor_col = cursor_col;

        let total = app.editor_buffer.line_count();
        let old_len = body.lines.len();
        let structural_edit = old_len != total;

        if structural_edit {
            // Structural edit: line count changed. For large files, use
            // the incremental range to update only affected lines from
            // the rope (now O(1) per line). Full rebuild only for small
            // files or when no incremental range is available.
            let tab_width = crate::gui::window::editor_buf::EditorBufferState::TAB_WIDTH;
            let incremental_range = app.editor_buffer.last_edit_line_range();
            let needs_full = incremental_range.is_none() || body.lines.is_empty();

            // Guard: never do a full lines_expanded() rebuild for files
            // over the huge threshold (50K lines).  Materialising all
            // lines with tab expansion would allocate & copy the entire
            // document on every structural edit (e.g. Enter key).
            let huge = total > super::HUGE_FILE_LINE_THRESHOLD;
            let force_incremental = huge && !needs_full;

            if (needs_full || total <= 5000) && !huge {
                body.lines = app.editor_buffer.lines_expanded();
            } else if let Some((first, last_excl)) = incremental_range {
                body.lines.resize(total, String::new());
                let last = last_excl.min(total + 1);
                let mut i = first.min(total.saturating_sub(1));
                while i < last {
                    let raw = app.editor_buffer.rope().line(i).unwrap_or_default();
                    let expanded = crate::gui::window::editor_buf::EditorBufferState::expand_tabs(
                        &raw, tab_width,
                    );
                    if i < body.lines.len() {
                        body.lines[i] = expanded;
                    }
                    i += 1;
                }
            } else if force_incremental {
                // Huge file with no incremental range: we must rebuild
                // but can't afford lines_expanded().  Resize and fill
                // by iterating the rope line by line (one pass, no
                // intermediate allocation of a full Vec<String>).
                body.lines.resize(total, String::new());
                for i in 0..total {
                    let raw = app.editor_buffer.rope().line(i).unwrap_or_default();
                    let expanded = crate::gui::window::editor_buf::EditorBufferState::expand_tabs(
                        &raw, tab_width,
                    );
                    body.lines[i] = expanded;
                }
            } else {
                body.lines = app.editor_buffer.lines_expanded();
            }

            // Invalidate all per-line caches — indices have shifted.
            app.cached_editor_data = None;
            app.cached_editor_lines_hash = 0;
            app.cached_line_hashes.clear();
            app.line_syntax_cache.clear();

            if std::env::var("ZAROXI_DEBUG_EDIT")
                .as_deref()
                .is_ok_and(|v| v == "1" || v == "structural")
            {
                eprintln!(
                    "ZAROXI_DEBUG_EDIT: structural_edit old_len={} new_len={} caches_cleared",
                    old_len, total,
                );
            }
            if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DEBUG_LARGE_FILE: structural_edit old={} new={} large_file_mode={}",
                    old_len, total, app.large_file_mode,
                );
            }
        } else if let Some((first, last_excl)) = app.editor_buffer.last_edit_line_range() {
            // Content-only edit: incremental update of changed lines.
            let last = last_excl.min(total);

            if body.lines.len() < total {
                body.lines.resize(total, String::new());
            } else if body.lines.len() > total {
                body.lines.truncate(total);
            }

            let tab_width = crate::gui::window::editor_buf::EditorBufferState::TAB_WIDTH;
            let mut i = first;
            while i < last {
                let raw = app.editor_buffer.rope().line(i).unwrap_or_default();
                let expanded =
                    crate::gui::window::editor_buf::EditorBufferState::expand_tabs(&raw, tab_width);
                if i < body.lines.len() {
                    body.lines[i] = expanded;
                }
                i += 1;
            }

            if std::env::var("ZAROXI_DEBUG_EDIT").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DEBUG_EDIT: content_edit first={} last={} total={}",
                    first, last, total,
                );
            }

            // In large-file mode, content-only edits must also clear the
            // render cache because the fast O(1) hash (line count +
            // boundary lines) will NOT detect edits in the middle of the
            // file.  Letting a stale editor_body_text through causes
            // render mismatches and eventual crashes on structural edits.
            if app.large_file_mode {
                app.cached_editor_data = None;
                app.cached_editor_lines_hash = 0;
                if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_DEBUG_LARGE_FILE: content-edit cache cleared (large_file_mode)"
                    );
                }
            }
        } else {
            // Full rebuild when no incremental range is available
            let new_lines = app.editor_buffer.lines_expanded();
            body.lines = new_lines;
        }
    }

    // For large files, sync edits from the rope back to doc_buffers
    // so the PieceTable stays in sync with user edits.
    if app.large_file_mode {
        if let Some(ref edit_range) = app.editor_buffer.last_edit_line_range() {
            let key = app
                .committed_active_file
                .as_deref()
                .and_then(|s| s.strip_prefix("buf:"))
                .map(|s| s.to_string());
            if let Some(path_str) = key {
                if let Some(db) = app.doc_buffers.get_mut(&path_str) {
                    let total = db.total_lines();
                    let (first, last_excl) = *edit_range;
                    let new_line_count = app.editor_buffer.line_count();
                    let structural = new_line_count != total;
                    let tab_width = crate::gui::window::editor_buf::EditorBufferState::TAB_WIDTH;

                    if structural {
                        // Structural edit: compute old/new ranges and do a
                        // single splice in doc_buffers.  The rope line count
                        // delta tells us whether lines were inserted or deleted.
                        let delta = pre_sync_body_line_count as isize - new_line_count as isize;
                        let old_range_lines = if delta > 0 {
                            // Deletion: old had more lines.
                            (last_excl.saturating_sub(first)) + delta as usize
                        } else {
                            // Insertion / no-op: old range was 1 line.
                            1usize
                        };
                        let old_last = (first + old_range_lines).min(total);
                        let new_last = last_excl.min(new_line_count).max(first);

                        let old_lines: Vec<String> = if first < total {
                            db.lines_in_range(first, old_last.saturating_sub(1))
                                .into_iter()
                                .map(|(_, s)| s)
                                .collect()
                        } else {
                            vec![]
                        };
                        let new_lines: Vec<String> = (first..new_last)
                            .map(|i| {
                                let raw = app.editor_buffer.rope().line(i).unwrap_or_default();
                                crate::gui::window::editor_buf::EditorBufferState::expand_tabs(
                                    &raw, tab_width,
                                )
                            })
                            .collect();

                        let old_text = old_lines.join("\n");
                        let new_text = new_lines.join("\n");

                        let byte_start = if first < total {
                            db.line_col_to_byte_offset(first, 0)
                        } else {
                            db.total_bytes()
                        };
                        let old_byte_len = old_text.len();

                        if old_byte_len > 0 {
                            db.delete(byte_start, byte_start + old_byte_len);
                        }
                        if !new_text.is_empty() {
                            if first >= total {
                                db.insert(byte_start, &format!("\n{}", new_text));
                            } else {
                                db.insert(byte_start, &new_text);
                            }
                        }

                        if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                            eprintln!(
                                "ZAROXI_DEBUG_LARGE_FILE: structural_splice first={} old_last={} new_last={} delta={} old_text_len={} new_text_len={} rope_lines={} doc_linesOLD={} doc_linesNEW={}",
                                first,
                                old_last,
                                new_last,
                                delta,
                                old_text.len(),
                                new_text.len(),
                                new_line_count,
                                total,
                                db.total_lines(),
                            );
                        }
                    } else {
                        // Content-only edit: iterate affected lines.
                        let old_last = last_excl.min(total).max(first);
                        let mut i = first;
                        while i < old_last {
                            let new_raw = app.editor_buffer.rope().line(i).unwrap_or_default();
                            let new_expanded =
                                crate::gui::window::editor_buf::EditorBufferState::expand_tabs(
                                    &new_raw, tab_width,
                                );
                            let old = db
                                .lines_in_range(i, i)
                                .into_iter()
                                .next()
                                .map(|(_, s)| s)
                                .unwrap_or_default();
                            if old != new_expanded {
                                let byte_start = db.line_col_to_byte_offset(i, 0);
                                let old_byte_len = old.len();
                                if old_byte_len > 0 {
                                    db.delete(byte_start, byte_start + old_byte_len);
                                }
                                if !new_expanded.is_empty() {
                                    db.insert(byte_start, &new_expanded);
                                }
                            }
                            i += 1;
                        }

                        if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                            eprintln!(
                                "ZAROXI_DEBUG_LARGE_FILE: content_edit_synced first={} last={} rope_lines={} doc_lines={}",
                                first, old_last, new_line_count, total,
                            );
                        }
                    }
                }
            }
        }
    }

    app.editor_buffer.clear_edit_line_range();

    // Schedule background tree-sitter parse for syntax highlighting.
    // Works for ALL file sizes — the worker processes on a background thread
    // and never blocks the UI. Stale results are safely discarded.
    app.schedule_background_parse();
}

/// Request a redraw for the editor after an editing operation.
fn request_editor_redraw(app: &mut GuiApp) {
    app.invalidate(super::InvalidationFlags::input());
}

/// Convert a wheel/trackpad delta into a signed explorer-row scroll step.
/// Positive = scroll the tree down (toward the bottom, larger offset);
/// negative = scroll up. A non-zero delta always moves at least one row so
/// small trackpad nudges still register.
fn explorer_wheel_rows(delta: &MouseScrollDelta) -> i32 {
    use crate::gui::window::editor_shell::constants::EXPLORER_ROW_H;
    let raw_y = match *delta {
        MouseScrollDelta::LineDelta(_x, y) => y * 3.0,
        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / (EXPLORER_ROW_H * 0.5),
    };
    if raw_y == 0.0 {
        return 0;
    }
    let mag = raw_y.abs().round().max(1.0) as i32;
    // Wheel-up (positive y) scrolls the tree toward the top (smaller offset).
    if raw_y > 0.0 { -mag } else { mag }
}

/// True when the pointer currently sits over the explorer sidebar region.
fn pointer_over_sidebar(app: &GuiApp) -> bool {
    pointer_over_region(app, zaroxi_core_engine_style::PanelRole::SidePanel)
}

/// True when the pointer currently sits over the tab strip region.
fn pointer_over_tab_strip(app: &GuiApp) -> bool {
    pointer_over_region(app, zaroxi_core_engine_style::PanelRole::ContentTabStrip)
}

fn pointer_over_region(app: &GuiApp, role: zaroxi_core_engine_style::PanelRole) -> bool {
    let Some((cx, cy)) = app.interaction.cursor_pos_f32() else {
        return false;
    };
    let regions = app.layout_controller.shell_regions();
    crate::gui::region_dispatch::find_region_by_role(regions, role)
        .map(|r| {
            let sx = r.rect.x as f32;
            let sy = r.rect.y as f32;
            let sw = r.rect.width as f32;
            let sh = r.rect.height as f32;
            cx >= sx && cx < sx + sw && cy >= sy && cy < sy + sh
        })
        .unwrap_or(false)
}

/// Apply mouse-wheel delta to composition pending scroll state and trigger
/// a redraw.  Called from `window_event(MouseWheel)`.
///
/// Deltas are accumulated in logical pixels via `pending_vscroll_px` so that
/// trackpad events and wheel notches both produce smooth sub-line scrolling.
pub(crate) fn process_mouse_wheel(app: &mut GuiApp, delta: &MouseScrollDelta) {
    // When the pointer is over the explorer sidebar, scroll the file tree
    // instead of the editor. The offset is clamped against the viewport in the
    // render path; here we only nudge it (saturating at the top).
    if pointer_over_sidebar(app) {
        let rows = explorer_wheel_rows(delta);
        if rows != 0 {
            if rows > 0 {
                app.explorer_scroll_top = app.explorer_scroll_top.saturating_add(rows as usize);
            } else {
                app.explorer_scroll_top = app.explorer_scroll_top.saturating_sub((-rows) as usize);
            }
            app.invalidate(super::InvalidationFlags::scroll());
        }
        return;
    }

    // When the pointer is over the tab strip and tabs overflow, scroll the
    // tab strip horizontally instead of the editor.
    if pointer_over_tab_strip(app) {
        let scroll_px: f32 = match *delta {
            MouseScrollDelta::LineDelta(_, y) => y * 48.0,
            MouseScrollDelta::PixelDelta(pos) => {
                // Horizontal mouse wheel on the tab strip scrolls tabs.
                if pos.x.abs() > pos.y.abs() { pos.x as f32 } else { pos.y as f32 }
            }
        };
        if scroll_px.abs() > 0.01 {
            app.tab_state.scroll_offset -= scroll_px;
            app.tab_state.scroll_offset = app.tab_state.scroll_offset.max(0.0);
            app.cockpit_status_fingerprint = 0;
            app.needs_render = true;
        }
        return;
    }

    let scroll_px: f32 = match *delta {
        MouseScrollDelta::LineDelta(x, y) => {
            if app.shift_held {
                let h_px = x * 24.0;
                if let Some(ref mut comp) = app.composition {
                    comp.pending_hscroll_px -= h_px;
                }
            }
            y * 48.0 // 3 lines × 16 px/line
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

    if scroll_px.abs() > 0.01
        && let Some(ref mut comp) = app.composition
    {
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

    if let Some(ref mut comp) = app.composition {
        comp.pending_refresh_reason =
            Some(zaroxi_application_workspace::workspace_view::RefreshReason::CursorMoved);
    }
    app.invalidate(super::InvalidationFlags::scroll());
}
