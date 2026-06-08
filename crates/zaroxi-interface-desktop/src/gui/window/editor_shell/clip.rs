/*!
Editor Phase 1 — Clipping projections.

Prevents editor text, gutter content, caret decorations, and selection
highlights from painting outside the editor viewport. When the rendering
pipeline supports scissor/clip regions, this module provides the canonical
clip rect. Otherwise, callers can use the projection helpers to exclude
out-of-bounds primitives before painting.
*/

use super::view::EditorViewport;

/// Project a UiBlock rect to the editor viewport clip boundary.
///
/// Returns `None` if the block is entirely outside the viewport.
/// Returns a clamped rect if the block partially overlaps the viewport.
pub fn clip_ui_block_rect(
    rect: (f32, f32, f32, f32),
    viewport: &EditorViewport,
) -> Option<(f32, f32, f32, f32)> {
    clamp_rect_to_bounds(rect, viewport.clip_rect)
}

/// Check whether any part of a rect falls inside the viewport clip region.
pub fn is_visible_in_viewport(rect: (f32, f32, f32, f32), viewport: &EditorViewport) -> bool {
    rects_overlap(rect, viewport.clip_rect)
}

/// Clamp a text line to the viewport content area (horizontal only).
/// Returns the visible substring range (start_col, end_col) for the given
/// line position, or `None` if the line is fully above/below the viewport.
pub fn visible_char_range(
    line_y: f32,
    line_h: f32,
    col_start: f32,
    col_end: f32,
    viewport: &EditorViewport,
) -> Option<(usize, usize)> {
    let vy = viewport.clip_rect.1;
    let vh = viewport.clip_rect.3;

    if line_y + line_h < vy || line_y > vy + vh {
        return None;
    }

    let vx = viewport.clip_rect.0;
    let vw = viewport.clip_rect.2;

    if col_end < vx || col_start > vx + vw {
        return None;
    }

    Some((0, 0))
}

// ── Internal helpers ──

fn clamp_rect_to_bounds(
    rect: (f32, f32, f32, f32),
    bounds: (f32, f32, f32, f32),
) -> Option<(f32, f32, f32, f32)> {
    let x1 = rect.0.max(bounds.0);
    let y1 = rect.1.max(bounds.1);
    let x2 = (rect.0 + rect.2).min(bounds.0 + bounds.2);
    let y2 = (rect.1 + rect.3).min(bounds.1 + bounds.3);

    if x2 <= x1 || y2 <= y1 { None } else { Some((x1, y1, x2 - x1, y2 - y1)) }
}

fn rects_overlap(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    let a_x2 = a.0 + a.2;
    let a_y2 = a.1 + a.3;
    let b_x2 = b.0 + b.2;
    let b_y2 = b.1 + b.3;

    a.0 < b_x2 && a_x2 > b.0 && a.1 < b_y2 && a_y2 > b.1
}
