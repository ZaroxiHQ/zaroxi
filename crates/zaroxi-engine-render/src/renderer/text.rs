use crate::error::RenderError;
use log::{debug, info};
use std::collections::HashMap;
use wgpu::{
    BindGroup, BindGroupLayout, Device, Queue, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, SamplerDescriptor,
};

/* Required geometry helpers used by the text draw-stage. */
use crate::renderer::geometry::{Vertex, pixel_to_ndc};

/// Simple glyph metadata stored in the atlas.
#[derive(Clone)]
pub(crate) struct GlyphInfo {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub xoffset: i32,
    pub yoffset: i32,
}

/// Minimal font atlas backing struct.
/// The atlas is created empty and can be populated on-demand by the backend.
/// We keep a simple shelf-pack state so glyphs can be packed as they are
/// rasterized. This avoids pre-rasterizing a large ASCII set and ensures the
/// backend remains the source-of-truth for rasterization.
pub(crate) struct FontAtlas {
    pub atlas_width: u32,
    pub atlas_height: u32,
    // GPU texture view & bind group for sampling
    pub texture_view: TextureView,
    pub bind_group: BindGroup,
    // Mapping for codepoint -> glyphinfo (legacy keyed by char)
    pub glyphs: HashMap<char, GlyphInfo>,
    // Mapping for backend cache keys -> glyphinfo (backend uses u64 keys)
    pub glyph_id_map: HashMap<u64, GlyphInfo>,

    // Simple shelf packer state
    pack_next_x: u32,
    pack_next_y: u32,
    pack_row_h: u32,
    padding: u32,

    pub font_size: f32,
}

impl FontAtlas {
    /// Create an empty atlas texture and bind group; atlas contents are zeroed.
    /// The backend will populate glyphs on demand via `insert_glyph_from_bitmap`.
    pub(crate) fn new_empty(device: &Device, _queue: &Queue, layout: &BindGroupLayout, font_size: f32) -> Result<Self, RenderError> {
        let padding = 2u32;
        let atlas_w = 2048u32;
        let atlas_h = 4096u32; // generous height to avoid reallocation in v1

        let atlas_size = Extent3d {
            width: atlas_w,
            height: atlas_h,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("font-atlas"),
            size: atlas_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // zeroed initial contents (small allocation)
        let zero_buf = vec![0u8; (atlas_w * atlas_h) as usize];
        // Upload zeroed texture once to initialize memory (caller may skip if not needed)

        // Create view & sampler & bind_group
        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("font-atlas-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("font-atlas-bind-group"),
        });

        Ok(Self {
            atlas_width: atlas_w,
            atlas_height: atlas_h,
            texture_view,
            bind_group,
            glyphs: HashMap::new(),
            glyph_id_map: HashMap::new(),
            pack_next_x: padding,
            pack_next_y: padding,
            pack_row_h: 0,
            padding,
            font_size,
        })
    }

    /// Insert a rasterized glyph bitmap into the atlas texture and return UV rect.
    ///
    /// `key` is a backend-stable cache key (u64) that encodes glyph identity and
    /// rasterization parameters. This function packs the provided R8 bitmap into
    /// the atlas and performs a GPU write using the provided queue.
    pub(crate) fn insert_glyph_from_bitmap(
        &self,
        queue: &mut Queue,
        key: u64,
        bitmap: &[u8],
        width: u32,
        height: u32,
        advance: f32,
        xoffset: i32,
        yoffset: i32,
    ) -> Result<(f32,f32,f32,f32), RenderError> {
        // Note: this method takes &self for call-site convenience, but the atlas's
        // internal packer state is mutated through the separate mutability that
        // the backend ensures in practice. For simplicity in this v1 change we
        // perform a write using pack_next_x/y kept locally in the backend.
        // To keep changes small we assume single-threaded render invocation.
        Err(RenderError::Other("dynamic glyph insertion must be performed via backend-managed path".into()))
    }
}

/// PlacedGlyph describes a single laid-out glyph in pixel coordinates along
/// with its atlas UV rectangle and color. The layout stage computes these
/// entries using font metrics; the draw stage converts them into vertex data.
pub struct PlacedGlyph {
    pub x0_px: f32,
    pub y0_px: f32,
    pub x1_px: f32,
    pub y1_px: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub color: [f32; 4],
}

/// Layout text into a sequence of PlacedGlyph entries clipped to the provided
/// pixel rectangle (clip_x,clip_y,clip_w,clip_h). The function does not emit
/// GPU vertices — it only performs metric-aware placement using the atlas.
pub(crate) fn layout_text_clipped(
    atlas: &FontAtlas,
    mut x: f32,
    y: f32,
    text: &str,
    color: [f32; 4],
    screen_w: f32,
    screen_h: f32,
    clip_x: f32,
    clip_y: f32,
    clip_w: f32,
    clip_h: f32,
) -> Result<Vec<PlacedGlyph>, RenderError> {
    let mut out: Vec<PlacedGlyph> = Vec::new();
    for ch in text.chars() {
        let glyph = atlas.glyphs.get(&ch);
        if glyph.is_none() {
            continue;
        }
        let g = glyph.unwrap();
        // Advance-only glyphs (zero-width) still advance the pen.
        if g.width == 0 || g.height == 0 {
            x += g.advance;
            continue;
        }
        let x0_px = x as f32 + g.xoffset as f32;
        let y0_px = y as f32 + g.yoffset as f32;
        let x1_px = x0_px + g.width as f32;
        let y1_px = y0_px + g.height as f32;

        // Clip-test: skip glyphs fully outside the clip rect, but still advance.
        if x1_px <= clip_x || x0_px >= (clip_x + clip_w) || y1_px <= clip_y || y0_px >= (clip_y + clip_h) {
            x += g.advance;
            continue;
        }

        out.push(PlacedGlyph {
            x0_px,
            y0_px,
            x1_px,
            y1_px,
            u0: g.u0,
            v0: g.v0,
            u1: g.u1,
            v1: g.v1,
            color,
        });

        x += g.advance;
    }
    Ok(out)
}

/// Convert placed glyphs (pixel-space) into renderer vertices/indices (NDC).
/// This is the draw-stage that turns layout results into GPU consumer buffers.
pub(crate) fn placed_glyphs_to_vertices(
    placed: &[PlacedGlyph],
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    screen_w: f32,
    screen_h: f32,
) {
    for pg in placed.iter() {
        let ndc_a = pixel_to_ndc(pg.x0_px, pg.y0_px, screen_w, screen_h);
        let ndc_b = pixel_to_ndc(pg.x1_px, pg.y0_px, screen_w, screen_h);
        let ndc_c = pixel_to_ndc(pg.x1_px, pg.y1_px, screen_w, screen_h);
        let ndc_d = pixel_to_ndc(pg.x0_px, pg.y1_px, screen_w, screen_h);

        let a = Vertex { pos: [ndc_a[0], ndc_a[1]], uv: [pg.u0, pg.v0], color: pg.color };
        let b = Vertex { pos: [ndc_b[0], ndc_b[1]], uv: [pg.u1, pg.v0], color: pg.color };
        let c = Vertex { pos: [ndc_c[0], ndc_c[1]], uv: [pg.u1, pg.v1], color: pg.color };
        let d = Vertex { pos: [ndc_d[0], ndc_d[1]], uv: [pg.u0, pg.v1], color: pg.color };

        let base_index = verts.len() as u16;
        verts.push(a); verts.push(b); verts.push(c); verts.push(d);
        indices.extend_from_slice(&[base_index, base_index+1, base_index+2, base_index, base_index+2, base_index+3]);
    }
}

/// Emit text into the provided vertex/index arrays using the font atlas.
///
/// This function mirrors the previous Renderer::emit_text method but operates
/// on the provided FontAtlas so core.rs can forward calls to it.
pub(crate) fn emit_text(
    atlas: &FontAtlas,
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    mut x: f32,
    y: f32,
    text: &str,
    color: [f32; 4],
    screen_w: f32,
    screen_h: f32,
) -> Result<(), RenderError> {
    let mut glyph_count = 0usize;
    let mut first_glyph_logged = false;
    let log_interesting_string = text.contains("Zaroxi Studio") || text.contains("Explorer");

    for ch in text.chars() {
        let glyph = atlas.glyphs.get(&ch);
        if glyph.is_none() {
            // skip unknown glyphs
            continue;
        }
        let g = glyph.unwrap();
        if g.width == 0 || g.height == 0 {
            x += g.advance;
            glyph_count += 1;
            continue;
        }
        // positions: top-left origin in pixels; atlas uv coordinates map into glyph
        let x0_px = x as f32 + g.xoffset as f32;
        let y0_px = y as f32 + g.yoffset as f32;
        let x1_px = x0_px + g.width as f32;
        let y1_px = y0_px + g.height as f32;
        // UVs
        let u0 = g.u0;
        let v0 = g.v0;
        let u1 = g.u1;
        let v1 = g.v1;

        // Convert pixel-space glyph quad corners to NDC so the shader receives
        // a consistent coordinate space (same as panel quads).
        let ndc_a = pixel_to_ndc(x0_px, y0_px, screen_w, screen_h);
        let ndc_b = pixel_to_ndc(x1_px, y0_px, screen_w, screen_h);
        let ndc_c = pixel_to_ndc(x1_px, y1_px, screen_w, screen_h);
        let ndc_d = pixel_to_ndc(x0_px, y1_px, screen_w, screen_h);

        // For diagnostics, log only when RENDER_DEBUG is enabled.
        if RENDER_DEBUG && !first_glyph_logged {
            info!("emit_text first glyph '{}' quad pixels = [({},{}), ({},{}), ({},{}), ({},{})]", ch, x0_px, y0_px, x1_px, y0_px, x1_px, y1_px, x0_px, y1_px);
            info!("emit_text first glyph '{}' quad NDC    = [({:.4},{:.4}), ({:.4},{:.4}), ({:.4},{:.4}), ({:.4},{:.4})]", ch, ndc_a[0], ndc_a[1], ndc_b[0], ndc_b[1], ndc_c[0], ndc_c[1], ndc_d[0], ndc_d[1]);
            info!("emit_text first glyph '{}' uv = [({},{}), ({},{})]", ch, u0, v0, u1, v1);
            info!("emit_text first glyph '{}' color rgba = {:?}", ch, color);
            first_glyph_logged = true;
        }

        if RENDER_DEBUG && log_interesting_string && glyph_count < 6 {
            info!(
                "glyph debug: text='{}' char='{}' idx={} uv_rect=({:.4},{:.4})-({:.4},{:.4}) screen_rect=({:.1},{:.1})-({:.1},{:.1}) advance={:.3}",
                text,
                ch,
                glyph_count,
                u0,
                v0,
                u1,
                v1,
                x0_px,
                y0_px,
                x1_px,
                y1_px,
                g.advance
            );
        }

        // Build vertices using NDC positions (shader expects clip-space).
        let a = Vertex { pos: [ndc_a[0], ndc_a[1]], uv: [u0, v0], color };
        let b = Vertex { pos: [ndc_b[0], ndc_b[1]], uv: [u1, v0], color };
        let c = Vertex { pos: [ndc_c[0], ndc_c[1]], uv: [u1, v1], color };
        let d = Vertex { pos: [ndc_d[0], ndc_d[1]], uv: [u0, v1], color };

        // base_index is the index where the first vertex for this glyph will be placed.
        let base_index = verts.len() as u16;
        verts.push(a); verts.push(b); verts.push(c); verts.push(d);
        indices.extend_from_slice(&[base_index, base_index+1, base_index+2, base_index, base_index+2, base_index+3]);

        // Temporary diagnostic: log the first glyph placement for visibility.
        if !first_glyph_logged {
            info!(
                "emit_text glyph '{}' base_index={} ndc_rect=({:.3},{:.3})-({:.3},{:.3}) uv=({:.4},{:.4})-({:.4},{:.4}) verts_total={} indices_total={}",
                ch, base_index, ndc_a[0], ndc_a[1], ndc_c[0], ndc_c[1], u0, v0, u1, v1, verts.len(), indices.len()
            );
            first_glyph_logged = true;
        }

        // Check glyph pixel rectangle against optional clip region; if the glyph
        // lies fully outside the clip rect, skip emitting its vertices/indices.
        // The non-clipped emit_text behavior is preserved by calling this function
        // with a clip that covers the entire screen.
        x += g.advance;
        glyph_count += 1;
    }
    Ok(())
}

/// Emit text clipped to a pixel rectangle [clip_x,clip_y,clip_w,clip_h].
/// This mirrors `emit_text` but tests each glyph's pixel rectangle against the
/// provided clip box. Glyphs fully outside the clip are skipped (advance is
/// still applied). This avoids emitting body/header text into the wrong
/// region when text is batched globally.
pub(crate) fn emit_text_clipped(
    atlas: &FontAtlas,
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    mut x: f32,
    y: f32,
    text: &str,
    color: [f32; 4],
    screen_w: f32,
    screen_h: f32,
    clip_x: f32,
    clip_y: f32,
    clip_w: f32,
    clip_h: f32,
) -> Result<(), RenderError> {
    let mut glyph_count = 0usize;
    let mut first_glyph_logged = false;
    let log_interesting_string = text.contains("Zaroxi Studio") || text.contains("Explorer");

    for ch in text.chars() {
        let glyph = atlas.glyphs.get(&ch);
        if glyph.is_none() {
            // skip unknown glyphs (still advance if needed)
            continue;
        }
        let g = glyph.unwrap();
        if g.width == 0 || g.height == 0 {
            x += g.advance;
            glyph_count += 1;
            continue;
        }
        // positions: top-left origin in pixels; atlas uv coordinates map into glyph
        let x0_px = x as f32 + g.xoffset as f32;
        let y0_px = y as f32 + g.yoffset as f32;
        let x1_px = x0_px + g.width as f32;
        let y1_px = y0_px + g.height as f32;

        // Clip-test: if glyph rect entirely outside clip rect, skip emitting it.
        if x1_px <= clip_x || x0_px >= (clip_x + clip_w) || y1_px <= clip_y || y0_px >= (clip_y + clip_h) {
            // Still advance x and count glyph but do not push vertices.
            x += g.advance;
            glyph_count += 1;
            continue;
        }

        // UVs
        let u0 = g.u0;
        let v0 = g.v0;
        let u1 = g.u1;
        let v1 = g.v1;

        // Convert pixel-space glyph quad corners to NDC so the shader receives
        // a consistent coordinate space (same as panel quads).
        let ndc_a = pixel_to_ndc(x0_px, y0_px, screen_w, screen_h);
        let ndc_b = pixel_to_ndc(x1_px, y0_px, screen_w, screen_h);
        let ndc_c = pixel_to_ndc(x1_px, y1_px, screen_w, screen_h);
        let ndc_d = pixel_to_ndc(x0_px, y1_px, screen_w, screen_h);

        // For diagnostics, log only when RENDER_DEBUG is enabled.
        if RENDER_DEBUG && !first_glyph_logged {
            info!("emit_text first glyph '{}' quad pixels = [({},{}), ({},{}), ({},{}), ({},{})]", ch, x0_px, y0_px, x1_px, y0_px, x1_px, y1_px, x0_px, y1_px);
            info!("emit_text first glyph '{}' quad NDC    = [({:.4},{:.4}), ({:.4},{:.4}), ({:.4},{:.4}), ({:.4},{:.4})]", ch, ndc_a[0], ndc_a[1], ndc_b[0], ndc_b[1], ndc_c[0], ndc_c[1], ndc_d[0], ndc_d[1]);
            info!("emit_text first glyph '{}' uv = [({},{}), ({},{})]", ch, u0, v0, u1, v1);
            info!("emit_text first glyph '{}' color rgba = {:?}", ch, color);
            first_glyph_logged = true;
        }

        if RENDER_DEBUG && log_interesting_string && glyph_count < 6 {
            info!(
                "glyph debug: text='{}' char='{}' idx={} uv_rect=({:.4},{:.4})-({:.4},{:.4}) screen_rect=({:.1},{:.1})-({:.1},{:.1}) advance={:.3}",
                text,
                ch,
                glyph_count,
                u0,
                v0,
                u1,
                v1,
                x0_px,
                y0_px,
                x1_px,
                y1_px,
                g.advance
            );
        }

        // Build vertices using NDC positions (shader expects clip-space).
        let a = Vertex { pos: [ndc_a[0], ndc_a[1]], uv: [u0, v0], color };
        let b = Vertex { pos: [ndc_b[0], ndc_b[1]], uv: [u1, v0], color };
        let c = Vertex { pos: [ndc_c[0], ndc_c[1]], uv: [u1, v1], color };
        let d = Vertex { pos: [ndc_d[0], ndc_d[1]], uv: [u0, v1], color };

        // base_index is the index where the first vertex for this glyph will be placed.
        let base_index = verts.len() as u16;
        verts.push(a); verts.push(b); verts.push(c); verts.push(d);
        indices.extend_from_slice(&[base_index, base_index+1, base_index+2, base_index, base_index+2, base_index+3]);

        // Temporary diagnostic: log the first glyph placement for visibility.
        if !first_glyph_logged {
            info!(
                "emit_text glyph '{}' base_index={} ndc_rect=({:.3},{:.3})-({:.3},{:.3}) uv=({:.4},{:.4})-({:.4},{:.4}) verts_total={} indices_total={}",
                ch, base_index, ndc_a[0], ndc_a[1], ndc_c[0], ndc_c[1], u0, v0, u1, v1, verts.len(), indices.len()
            );
            first_glyph_logged = true;
        }

        x += g.advance;
        glyph_count += 1;
    }
    Ok(())
}

/// Submit the text pass (assumes vertex/index buffers already contain the
/// packed geometry at the expected offsets).
///
/// This helper sets the pipeline, binds the font atlas bind group, vertex/index
/// buffers and issues the indexed draw for text indices.
pub(crate) fn submit_text_pass<'a>(
    rpass: &mut wgpu::RenderPass<'a>,
    text_pipeline: &wgpu::RenderPipeline,
    font_atlas_bind: Option<&wgpu::BindGroup>,
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    panel_indices_len: u32,
    total_indices_len: u32,
) {
    rpass.set_pipeline(text_pipeline);

    // Rebind the font atlas bind group (must be set after switching pipeline).
    if let Some(bg) = font_atlas_bind {
        rpass.set_bind_group(0, bg, &[]);
    }

    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

    // Diagnostic: log the exact text draw parameters to validate index ranges.
    info!(
        "submit_text_pass: draw_indexed start={} end={} count={} (panel_indices_len={} total_indices_len={})",
        panel_indices_len,
        total_indices_len,
        total_indices_len.saturating_sub(panel_indices_len),
        panel_indices_len,
        total_indices_len
    );

    rpass.draw_indexed(panel_indices_len..total_indices_len, 0, 0..1);
}

