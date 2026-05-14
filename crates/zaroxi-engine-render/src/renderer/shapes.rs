use std::sync::atomic::Ordering;
use wgpu;
use log::{debug, info};
use crate::renderer::geometry::{push_colored_quad, Vertex, color_to_rgba};
use crate::renderer::debug::{
    render_debug_enabled, RENDER_DEBUG, FIRST_GLYPH_LOGGED, LOGGED_TITLEBAR, LOGGED_SIDEBAR,
    LOGGED_EDITOR, LOGGED_SIDEBAR_PACKED, FORCE_MAGENTA_SIDEBAR, DISABLE_TEXT_PASS, VALIDATION_SCENE,
};
use zaroxi_theme::SemanticColors;
use crate::renderer::core::Rect;

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
    panel: &zaroxi_app::view_model::RenderPanel,
    target: Rect,
    sem: &SemanticColors,
    screen_w: f32,
    screen_h: f32,
) -> Option<usize> {
    // Local layout metrics (header + content padding) were previously computed in core.rs.
    let header_h = 28.0f32;
    let content_padding = 8.0f32;

    // Header strip at the top of the panel rect
    let hx = target.x;
    let hy = target.y;
    let hw = target.w;
    let hh = header_h.min(target.h.max(0.0));

    // Choose a semantic header color per-panel (defaults -> panel_header_background).
    let header_color: [f32; 4] = match panel.id.as_str() {
        "titlebar" => color_to_rgba(&sem.title_bar_background),
        "sidebar" => color_to_rgba(&sem.panel_header_background),
        "editor" => color_to_rgba(&sem.panel_header_background),
        "right_panel" => color_to_rgba(&sem.panel_header_background),
        "bottom_panel" => color_to_rgba(&sem.panel_header_background),
        "status_bar" => color_to_rgba(&sem.panel_header_background),
        _ => color_to_rgba(&sem.panel_header_background),
    };

    if render_debug_enabled() {
        debug!("panel '{}' header_color = {:?}", panel.id, header_color);
    }

    // One-shot CPU-side logging of header quad color for the titlebar (startup only).
    if panel.id.as_str() == "titlebar" {
        if LOGGED_TITLEBAR.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            info!("cpu quad sample (titlebar) header_color = {:?}", header_color);
        }
    }

    push_colored_quad(verts, indices, hx, hy, hw, hh, header_color, screen_w, screen_h);

    // Content inset: a smaller block inside the panel for visual differentiation
    let cx = target.x + content_padding;
    let cy = target.y + hh + content_padding;
    let cw = (target.w - content_padding * 2.0).max(0.0);
    let ch = (target.h - hh - content_padding * 2.0).max(0.0);

    // Choose a semantic content/background color per-panel.
    let content_color: [f32; 4] = match panel.id.as_str() {
        "titlebar" => color_to_rgba(&sem.app_chrome_background),
        "sidebar" => color_to_rgba(&sem.sidebar_background),
        "editor" => color_to_rgba(&sem.editor_background),
        "right_panel" => color_to_rgba(&sem.assistant_panel_background),
        "bottom_panel" => color_to_rgba(&sem.panel_background),
        "status_bar" => color_to_rgba(&sem.status_bar_background),
        _ => color_to_rgba(&sem.panel_background),
    };

    if render_debug_enabled() {
        debug!("panel '{}' content_color = {:?}", panel.id, content_color);
    }

    // One-shot CPU-side logging of content quad color for sidebar/editor (startup only).
    match panel.id.as_str() {
        "sidebar" => {
            if LOGGED_SIDEBAR.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                info!("cpu quad sample (sidebar) content_color = {:?}", content_color);
            }
        }
        "editor" => {
            if LOGGED_EDITOR.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                info!("cpu quad sample (editor) content_color = {:?}", content_color);
            }
        }
        _ => {}
    }

    // Support a forced-magenta experiment for the sidebar only.
    let effective_color: [f32; 4] = if FORCE_MAGENTA_SIDEBAR && panel.id.as_str() == "titlebar" {
        [1.0, 0.0, 1.0, 1.0]
    } else if FORCE_MAGENTA_SIDEBAR && panel.id.as_str() == "sidebar" {
        [1.0, 0.0, 1.0, 1.0]
    } else {
        content_color
    };

    if FORCE_MAGENTA_SIDEBAR && panel.id.as_str() == "sidebar" {
        info!("FORCE_MAGENTA_SIDEBAR enabled: overriding sidebar content_color -> {:?}", effective_color);
    }

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
