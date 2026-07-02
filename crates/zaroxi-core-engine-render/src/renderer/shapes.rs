use crate::renderer::core::PanelColors;
use crate::renderer::debug::{
    DISABLE_TEXT_PASS, FIRST_GLYPH_LOGGED, FORCE_MAGENTA_SIDEBAR, LOGGED_EDITOR, LOGGED_SIDEBAR,
    LOGGED_SIDEBAR_PACKED, LOGGED_TITLEBAR, RENDER_DEBUG, render_debug_enabled,
};
use crate::renderer::geometry::{Vertex, push_colored_quad};
use log::{debug, info};
use std::sync::atomic::Ordering;
use wgpu;

// Shape helpers: build panel/background quads and submit the shape pass.
//
// These functions were extracted from core.rs as a move-only refactor. They
// preserve behavior and logging exactly as before.

/// Queue header + content quads for a single panel into the provided vertex/index vectors.
///
/// Returns Some(base_idx) where base_idx is the index (usize) of the first
/// vertex of the content quad if a content quad was pushed, otherwise None.
///
/// Extended for Phase 27: draws a border quad around the block when border_color
/// is set, producing thinner separator-like visual edges.
pub(crate) fn queue_panel_quads(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    block: &crate::UiBlock,
    sem: &PanelColors,
    screen_w: f32,
    screen_h: f32,
) -> Option<usize> {
    // Local layout metrics (header + content padding) were previously computed in core.rs.
    let header_h = if block.header_only {
        // Header-only blocks use their full height as the "header"
        block.rect.h
    } else {
        28.0f32
    };
    // Use the block rect supplied by the caller (app/runtime owns layout decisions).
    let target = block.rect;

    let hh = header_h.min(target.h.max(0.0));

    // ── Content area (full-width fill below header, drawn first so borders sit on top) ──
    let content_base_idx: Option<usize> = if !block.header_only {
        let cy = target.y + hh;
        let cw = target.w;
        let ch = (target.h - hh).max(0.0);
        let content_color: [f32; 4] = block.content_color.unwrap_or(sem.panel_background);

        if render_debug_enabled() {
            debug!("block '{}' content_color = {:?}", block.id, content_color);
        }

        if cw > 0.0 && ch > 0.0 {
            let base_idx = verts.len();
            push_colored_quad(
                verts,
                indices,
                target.x,
                cy,
                cw,
                ch,
                content_color,
                screen_w,
                screen_h,
                block.corner_radius,
            );
            Some(base_idx)
        } else {
            None
        }
    } else {
        None
    };

    // ── Header strip ──
    let header_color: [f32; 4] = block.header_color.unwrap_or(sem.panel_header_background);

    if render_debug_enabled() {
        debug!("block '{}' header_color = {:?}", block.id, header_color);
    }

    if hh > 0.0 {
        push_colored_quad(
            verts,
            indices,
            target.x,
            target.y,
            target.w,
            hh,
            header_color,
            screen_w,
            screen_h,
            block.corner_radius,
        );
    }

    // ── Border rendering (drawn last so borders sit on top of fills) ──
    if let (Some(border_color), w) = (block.border_color, block.border_width)
        && w > 0.0
        && target.w > 0.0
        && target.h > 0.0
    {
        push_colored_quad(
            verts,
            indices,
            target.x,
            target.y,
            target.w,
            w,
            border_color,
            screen_w,
            screen_h,
            0.0,
        );
        push_colored_quad(
            verts,
            indices,
            target.x,
            target.y + target.h - w,
            target.w,
            w,
            border_color,
            screen_w,
            screen_h,
            0.0,
        );
        push_colored_quad(
            verts,
            indices,
            target.x,
            target.y,
            w,
            target.h,
            border_color,
            screen_w,
            screen_h,
            0.0,
        );
        push_colored_quad(
            verts,
            indices,
            target.x + target.w - w,
            target.y,
            w,
            target.h,
            border_color,
            screen_w,
            screen_h,
            0.0,
        );
    }

    content_base_idx
}

/// Queue a left/right vertical split UI with a divider between them.
pub(crate) fn queue_split_panel_quads(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    target: &crate::renderer::core::Rect,
    sem: &PanelColors,
    screen_w: f32,
    screen_h: f32,
) {
    let _ = sem; // panel colors already handled by caller; divider only
    let divider_w = 1.0f32;
    let dx = target.x + (target.w - divider_w) / 2.0;
    push_colored_quad(
        verts,
        indices,
        dx,
        target.y,
        divider_w,
        target.h,
        [0.25, 0.25, 0.3, 1.0],
        screen_w,
        screen_h,
        0.0,
    );
}

/// Submit the shape pass (assumes vertex/index buffers already contain the
/// packed geometry at the expected offsets).
///
/// This helper sets the pipeline, binds vertex/index buffers and issues the
/// indexed draw with the supplied index count.
pub(crate) fn submit_shape_pass<'a>(
    rpass: &mut wgpu::RenderPass<'a>,
    shape_pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    panel_indices_len: u32,
) {
    rpass.set_pipeline(shape_pipeline);
    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    rpass.draw_indexed(0..panel_indices_len, 0, 0..1);
}
