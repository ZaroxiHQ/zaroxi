use crate::error::RenderError;
use log::{debug, info};
use std::collections::HashMap;
use wgpu::{
    BindGroup, BindGroupLayout, Device, Queue, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, SamplerDescriptor,
};
use fontdue::Font;
use crate::renderer::debug::{RENDER_DEBUG, TEXT_SAMPLER_NEAREST, FIRST_GLYPH_LOGGED};
use crate::renderer::geometry::{Vertex, pixel_to_ndc};

/// Simple glyph metadata stored in the atlas.
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
pub(crate) struct FontAtlas {
    pub atlas_width: u32,
    pub atlas_height: u32,
    // GPU texture view & bind group for sampling
    pub texture_view: TextureView,
    pub bind_group: BindGroup,
    pub glyphs: HashMap<char, GlyphInfo>,
    pub font_size: f32,
}

impl FontAtlas {
    /// Build an atlas from the bundled font bytes.
    pub(crate) fn new(device: &Device, queue: &Queue, layout: &BindGroupLayout, font_size: f32) -> Result<Self, RenderError> {
        // Load bundled font from workspace assets (crate-agnostic path).
        // Use CARGO_MANIFEST_DIR relative traversal to reach workspace root.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path = std::path::PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let font_data = std::fs::read(&font_path).map_err(|e| RenderError::Other(format!("failed to read font: {:?}", e)))?;

        // fontdue::Font::from_bytes returns a Font in the versions we depend on.
        // If this returns a Result in future versions, handle it similarly.
        let font = Font::from_bytes(font_data.as_slice(), fontdue::FontSettings::default())
            .map_err(|e| RenderError::Other(format!("fontdue load failed: {:?}", e)))?;

        // Rasterize ASCII range 32..=126
        let padding = 2;
        let atlas_w = 2048u32;
        let mut atlas_h = 256u32;
        let mut x = padding;
        let mut y = padding;
        let mut row_h = 0u32;

        // store bitmaps temporarily
        let mut placements: Vec<(char, Vec<u8>, u32, u32, i32, i32, f32)> = Vec::new();

        for c in 32u8..=126u8 {
            let ch = c as char;
            let (metrics, bitmap) = font.rasterize(ch, font_size);
            let w = metrics.width as u32;
            let h = metrics.height as u32;
            if w == 0 || h == 0 {
                // Still need to store advance and offsets
                placements.push((ch, Vec::new(), w, h, metrics.xmin, metrics.ymin, metrics.advance_width));
                continue;
            }
            if x + w + padding > atlas_w {
                // new row
                x = padding;
                y += row_h + padding;
                row_h = 0;
            }
            placements.push((ch, bitmap, w, h, metrics.xmin, metrics.ymin, metrics.advance_width));
            x += w + padding;
            row_h = row_h.max(h);
            atlas_h = atlas_h.max(y + row_h + padding);
        }

        // Create atlas R8 texture
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

        // create a cpu buffer for atlas (R8)
        let mut atlas_buf = vec![0u8; (atlas_w * atlas_h) as usize];

        // place glyphs
        x = padding;
        y = padding;
        row_h = 0;
        let mut glyphs = HashMap::new();

        for (ch, bitmap, w, h, xmin, ymin, advance) in placements {
            if w == 0 || h == 0 {
                // empty glyph -> store advance only
                glyphs.insert(ch, GlyphInfo {
                    u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0,
                    width: 0, height: 0,
                    advance,
                    xoffset: xmin, yoffset: ymin,
                });
                continue;
            }
            if x + w + padding > atlas_w {
                x = padding;
                y += row_h + padding;
                row_h = 0;
            }
            for row in 0..h {
                let dst_off = ((y + row) * atlas_w + x) as usize;
                let src_off = (row * w) as usize;
                atlas_buf[dst_off..dst_off + w as usize].copy_from_slice(&bitmap[src_off..src_off + w as usize]);
            }
            let u0 = x as f32 / atlas_w as f32;
            let v0 = y as f32 / atlas_h as f32;
            let u1 = (x + w) as f32 / atlas_w as f32;
            let v1 = (y + h) as f32 / atlas_h as f32;

            glyphs.insert(ch, GlyphInfo {
                u0, v0, u1, v1,
                width: w, height: h,
                advance,
                xoffset: xmin, yoffset: ymin,
            });

            x += w + padding;
            row_h = row_h.max(h);
        }

        // Upload atlas to GPU using queue.write_texture (direct write) with the
        // wgpu 29.0.3 texel-copy types. This keeps the renderer implementation
        // compact and avoids introducing a direct dependency on wgpu_types.
        if RENDER_DEBUG {
            info!(
                "font atlas upload: format=R8Unorm size={}x{} bytes_per_row={}",
                atlas_w, atlas_h, atlas_w
            );

            // Detailed atlas diagnostics: total bytes, non-zero bytes, max value,
            // and first non-zero index (if any). These help detect an entirely
            // blank atlas (glyph rasterization failure).
            let total_bytes = atlas_buf.len();
            let non_zero = atlas_buf.iter().filter(|&&b| b != 0u8).count();
            let max_val = *atlas_buf.iter().max().unwrap_or(&0u8);
            let first_non_zero = atlas_buf.iter().position(|&b| b != 0u8);
            info!(
                "font atlas stats bytes={} non_zero={} max={} first_non_zero={:?}",
                total_bytes, non_zero, max_val, first_non_zero
            );

            let first_n = std::cmp::min(8usize, atlas_buf.len());
            info!("font atlas first {} bytes: {:?}", first_n, &atlas_buf[..first_n]);
        }

        // Fail fast if the atlas is entirely blank; this indicates glyph rasterization
        // produced no coverage and must be investigated on the CPU side.
        let total_bytes = atlas_buf.len();
        let non_zero = atlas_buf.iter().filter(|&&b| b != 0u8).count();
        if non_zero == 0 {
            return Err(RenderError::Other(
                "font atlas is entirely zero; glyph rasterization/upload source is blank"
                    .to_string(),
            ));
        }

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_buf,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas_w),
                rows_per_image: Some(atlas_h),
            },
            atlas_size,
        );

        if RENDER_DEBUG {
            info!("font atlas upload completed ({}x{})", atlas_w, atlas_h);
        }

        let texture_view = texture.create_view(&TextureViewDescriptor::default());

        // Create a simple sampler for the atlas
        let filter = if TEXT_SAMPLER_NEAREST { wgpu::FilterMode::Nearest } else { wgpu::FilterMode::Linear };
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("font-atlas-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            // use defaults for mipmap behavior to avoid version mismatches
            ..Default::default()
        });

        if RENDER_DEBUG {
            info!("font atlas sampler: mag_filter={:?} min_filter={:?} address_mode=ClampToEdge", filter, filter);
        }

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

        info!("font atlas bind_group created; shader coverage channel assumed = .r");

        Ok(Self {
            atlas_width: atlas_w,
            atlas_height: atlas_h,
            texture_view,
            bind_group,
            glyphs,
            font_size,
        })
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
    font_atlas: &FontAtlas,
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    panel_indices_len: u32,
    total_indices_len: u32,
) {
    rpass.set_pipeline(text_pipeline);
    // Rebind the font atlas bind group (must be set after switching pipeline).
    rpass.set_bind_group(0, &font_atlas.bind_group, &[]);
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
