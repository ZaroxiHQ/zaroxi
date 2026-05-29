use crate::renderer::core::Rect;
use crate::renderer::debug::{
    DISABLE_TEXT_PASS, FIRST_GLYPH_LOGGED, FORCE_MAGENTA_SIDEBAR, LOGGED_EDITOR, LOGGED_SIDEBAR,
    LOGGED_SIDEBAR_PACKED, LOGGED_TITLEBAR, RENDER_DEBUG, VALIDATION_SCENE, render_debug_enabled,
};
use crate::renderer::geometry::{Vertex, color_to_rgba, push_colored_quad};
use log::{debug, info};
use std::sync::atomic::Ordering;
use wgpu;
use zaroxi_interface_theme::SemanticColors;

/// Shape helpers: build panel/background quads and submit the shape pass.
///
/// These functions were extracted from core.rs as a move-only refactor. They
/// preserve behavior and logging exactly as before.

/// Queue header + content quads for a single panel into the provided vertex/index vectors.
///
/// Returns Some(base_idx) where base_idx is the index (usize) of the first
/// vertex of the content quad if a content quad was pushed, otherwise None.
pub(crate) fn queue_panel_quads(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    block: &crate::UiBlock,
    sem: &SemanticColors,
    screen_w: f32,
    screen_h: f32,
) -> Option<usize> {
    // Local layout metrics (header + content padding) were previously computed in core.rs.
    let header_h = 28.0f32;
    let content_padding = 8.0f32;

    // Use the block rect supplied by the caller (app/runtime owns layout decisions).
    let target = block.rect;

    // Header strip at the top of the block rect
    let hx = target.x;
    let hy = target.y;
    let hw = target.w;
    let hh = header_h.min(target.h.max(0.0));

    // Header color is supplied by the UiBlock visual hint; fallback to semantic token.
    let header_color: [f32; 4] = block
        .header_color
        .map(|c| color_to_rgba(&c))
        .unwrap_or(color_to_rgba(&sem.panel_header_background));

    if render_debug_enabled() {
        debug!("block '{}' header_color = {:?}", block.id, header_color);
    }

    push_colored_quad(verts, indices, hx, hy, hw, hh, header_color, screen_w, screen_h);

    // Content inset: a smaller block inside the panel for visual differentiation
    let cx = target.x + content_padding;
    let cy = target.y + hh + content_padding;
    let cw = (target.w - content_padding * 2.0).max(0.0);
    let ch = (target.h - hh - content_padding * 2.0).max(0.0);

    // Content color is supplied by the UiBlock visual hint; fallback to semantic token.
    let content_color: [f32; 4] = block
        .content_color
        .map(|c| color_to_rgba(&c))
        .unwrap_or(color_to_rgba(&sem.panel_background));

    if render_debug_enabled() {
        debug!("block '{}' content_color = {:?}", block.id, content_color);
    }

    // Use the content color directly; renderer should not introduce UI role overrides.
    let effective_color: [f32; 4] = content_color;

    if cw > 0.0 && ch > 0.0 {
        let base_idx = verts.len();
        push_colored_quad(verts, indices, cx, cy, cw, ch, effective_color, screen_w, screen_h);
        Some(base_idx)
    } else {
        None
    }
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
