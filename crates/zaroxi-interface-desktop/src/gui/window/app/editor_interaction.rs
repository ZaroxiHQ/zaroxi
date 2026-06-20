/*!
Editor cursor projection, selection helpers, and hit-testing.

Extracted from app.rs so editor interaction logic lives in one place
rather than being tangled with the winit event loop.

Phase: Rope-backed — uses EditorBufferState for canonical caret and selection.
*/

use winit::dpi::PhysicalPosition;

use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::WidgetId;
use zaroxi_core_engine_ui::layout_constants as lc;

use crate::gui::window::editor_shell::EditorViewport;

use super::GuiApp;

/// Project a winit cursor position to an editor (line, col) pair.
///
/// Returns None when the point does not fall inside the editor viewport.
/// Uses the actual monospace glyph advance from the font system for
/// column computation so the click-based caret lands on the same
/// visual position as the rendered text.
///
/// Uses the CLI-borrowed rope total_lines as the canonical line count
/// source when available, falling back to cv.lines.len() for backward compat.
pub(crate) fn project_editor_cursor(
    cursor_pos: PhysicalPosition<f64>,
    viewport: &EditorViewport,
    work_content: &Option<ShellWorkContent>,
    editor_scroll_offset: f32,
    char_w: f32,
    rope_total_lines: usize,
) -> Option<(usize, usize)> {
    let px = cursor_pos.x as f32;
    let py = cursor_pos.y as f32;

    if !viewport.contains_point(px, py) {
        return None;
    }

    let content_pad = lc::CONTENT_PAD_X;
    let header_h = lc::CONTENT_HEADER_H;
    let line_h = lc::LINE_HEIGHT;
    let rel_y = py - (viewport.content_rect.1 + header_h + content_pad);
    let rel_x = px - (viewport.content_rect.0 + content_pad);
    let visible_line = (rel_y / line_h).max(0.0) as usize;

    // Use the actual monospace advance from the font system so the
    // computed column matches the rendered glyph positions exactly.
    let col = (rel_x / char_w.max(0.001)).max(0.0) as usize;

    let usable_h = viewport.content_rect.3 - header_h - content_pad * 2.0;

    let total_lines = if rope_total_lines > 0 {
        rope_total_lines.max(1)
    } else {
        work_content
            .as_ref()
            .and_then(|w| w.editor_body.as_ref())
            .map(|cv| cv.lines.len().max(1))
            .unwrap_or(1)
    };
    let visible_lines_c = (usable_h / line_h).max(1.0) as usize;
    let max_scroll_c = (total_lines.saturating_sub(visible_lines_c)).max(1);
    let first_visible = (editor_scroll_offset * max_scroll_c as f32) as usize;
    let absolute_line = first_visible + visible_line;

    Some((absolute_line, col))
}

/// Extract selected text from the editor buffer state.
pub(crate) fn copy_selected_text(
    work_content: &Option<ShellWorkContent>,
    selection_range: &Option<(usize, usize, usize, usize)>,
) -> Option<String> {
    let (sl, sc, el, ec) = (*selection_range)?;
    let wc = work_content.as_ref()?;
    let body = wc.editor_body.as_ref()?;
    if body.lines.is_empty() {
        return None;
    }
    let mut selected = String::new();
    for line_idx in sl..=el {
        if line_idx >= body.lines.len() {
            break;
        }
        let line = &body.lines[line_idx];
        let start = if line_idx == sl { sc } else { 0 };
        let end = if line_idx == el { ec.min(line.len()) } else { line.len() };
        if start < end && start <= line.len() {
            selected.push_str(&line[start..end.min(line.len())]);
        }
        if line_idx < el {
            selected.push('\n');
        }
    }
    if selected.is_empty() { None } else { Some(selected) }
}

/// Called on MouseInput Pressed — begin a selection at the current cursor position
/// and set the caret though the rope-backed buffer.
/// The column from pixel projection is a visual column; convert it to a raw
/// character index using tab-aware mapping so the caret lands on the correct
/// logical character even when the line contains tabs.
///
/// Uses the actual monospace glyph advance from the font system for
/// column projection instead of a hardcoded stub, so the caret aligns
/// with the rendered glyphs.
pub(crate) fn init_selection_from_click(app: &mut GuiApp) {
    if let Some(pos) = app.interaction.cursor_pos_f32() {
        let phys = PhysicalPosition::new(pos.0 as f64, pos.1 as f64);
        if let Some(vp) = &app.editor_viewport {
            let char_w = app.monospace_advance_x().unwrap_or(lc::CHAR_WIDTH_STUB);
            let rope_lines = app.editor_buffer.total_lines();
            if let Some((line, vis_col)) = project_editor_cursor(
                phys,
                vp,
                &app.shell.work_content,
                app.interaction
                    .get_scroll_offset(&WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR }),
                char_w,
                rope_lines,
            ) {
                app.editor_buffer.set_caret_line_vis_col(line, vis_col);
                app.editor_buffer.begin_selection();
                app.invalidate(super::InvalidationFlags::input());
            }
        }
    }
}

/// Called on CursorMoved while selection is active — extend the selection range.
/// Converts visual column from pixel projection to raw char index for the rope.
/// Uses the actual monospace glyph advance from the font system.
pub(crate) fn update_drag_selection(app: &mut GuiApp, position: PhysicalPosition<f64>) {
    if !app.editor_buffer.selection_active {
        return;
    }
    if let Some(vp) = &app.editor_viewport {
        let char_w = app.monospace_advance_x().unwrap_or(lc::CHAR_WIDTH_STUB);
        let rope_lines = app.editor_buffer.total_lines();
        if let Some((line, vis_col)) = project_editor_cursor(
            position,
            vp,
            &app.shell.work_content,
            app.interaction
                .get_scroll_offset(&WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR }),
            char_w,
            rope_lines,
        ) {
            let line_str = app.editor_buffer.rope().line(line).unwrap_or_default();
            let raw_col = crate::gui::window::editor_buf::EditorBufferState::vis_to_raw_col(
                &line_str,
                vis_col,
                crate::gui::window::editor_buf::EditorBufferState::TAB_WIDTH,
            );
            let char_idx = app.editor_buffer.rope().line_col_to_char_index(line, raw_col);
            app.editor_buffer.extend_selection_to(char_idx);
            app.invalidate(super::InvalidationFlags::input());
        }
    }
}
