use wgpu;
/// Shared geometry helpers for the renderer.
///
/// This module contains small, broadly useful types and helpers that are
/// shared between shape and text code paths: the Vertex POD, vertex buffer
/// layout descriptor, NDC conversion helper and a compact quad packing helper.
///
/// Move-only: these functions are extracted from core.rs to keep geometry
/// concerns isolated during the refactor.

/// Vertex for textured quad.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // pos
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Convert theme Color -> renderer [f32;4]
pub(crate) fn color_to_rgba(c: &zaroxi_theme::Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

/// Convert pixel coordinates (top-left origin) -> NDC clip space used by the shader.
/// NDC x: -1..1 left->right, NDC y: -1..1 bottom->top. We map UI top-left into NDC by flipping Y.
pub(crate) fn pixel_to_ndc(px: f32, py: f32, sw: f32, sh: f32) -> [f32; 2] {
    let nx = (px / sw) * 2.0 - 1.0;
    let ny = 1.0 - (py / sh) * 2.0;
    [nx, ny]
}

/// Push a simple axis-aligned colored quad (pixel coords) into the provided
/// vertex & index vectors. This mirrors the previous nested helper extracted
/// from core.rs and preserves the exact vertex packing/layout expected by the
/// shaders.
pub(crate) fn push_colored_quad(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [f32; 4],
    screen_w: f32,
    screen_h: f32,
) {
    let base = verts.len() as u16;
    let a = pixel_to_ndc(x, y, screen_w, screen_h);
    let b = pixel_to_ndc(x + w, y, screen_w, screen_h);
    let c = pixel_to_ndc(x + w, y + h, screen_w, screen_h);
    let d = pixel_to_ndc(x, y + h, screen_w, screen_h);

    let v0 = Vertex { pos: a, uv: [0.0, 0.0], color };
    let v1 = Vertex { pos: b, uv: [0.0, 0.0], color };
    let v2 = Vertex { pos: c, uv: [0.0, 0.0], color };
    let v3 = Vertex { pos: d, uv: [0.0, 0.0], color };

    verts.push(v0);
    verts.push(v1);
    verts.push(v2);
    verts.push(v3);
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
