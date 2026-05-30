/*
Canonical CosmicTextRenderer implementation.

This file provides a single canonical render path for the CosmicText-backed
text renderer. It keeps one explicit `render_to_view` (live-target) entrypoint
and one shared `perform_render_pass` helper used both by the trait `render_pass`
and by the explicit `render_to_view` method.

Notes about behavior:
- No fake glyph bitmaps are synthesized in the normal prepare path.
- If real glyph rasterization / atlas insertion is not available, prepare() will
  report that honestly instead of faking success.
- A tiny debug-atlas helper is retained but clearly gated and never used to
  pretend that real atlas insertion succeeded.

The public TextRenderer lifecycle is preserved: queue_text, queued_len, prepare,
atlas_bind_group, resize_viewport, render_pass. The file also exposes a
`render_to_view` inherent method (preferred live-target path).
*/

use crate::error::RenderError;
use crate::renderer::text::{TextCommand, TextRenderer};
use crate::renderer::text_atlas::{RasterizedGlyph, SharedAtlas};
use crate::renderer::text_pipeline;
use cosmic_text::SwashCache;
use cosmic_text::{Attrs, Buffer as CosmicBuffer, Metrics, Shaping};
use log::{debug, info};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, CommandEncoder, Device, Extent3d, Queue, RenderPass,
    RenderPipeline, SamplerDescriptor, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

fn text_debug_enabled() -> bool {
    std::env::var("ZAROXI_TEXT_DEBUG").map(|v| v == "1").unwrap_or(false)
}

/// Atlas metadata used for diagnostics only.
#[derive(Clone, Debug)]
struct AtlasMeta {
    width: u32,
    height: u32,
    bytes: usize,
    regions: usize,
    format: String,
}

/// Per-frame summary produced by prepare().
#[derive(Clone, Debug)]
struct FrameSummary {
    shaped_glyphs_total: usize,
    extracted_for_emission: usize,
    rasterize_success_total: usize,
    atlas_insert_success_total: usize,
    instances_pushed: usize,
    fallback_used: bool,
}

/// Small sampled instance record for logging.
#[derive(Clone, Debug)]
struct InstanceSample {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    uv0: (f32, f32),
    uv1: (f32, f32),
    color: [f32; 4],
}

// GPU instance layout matching WGSL instance attributes (NDC positions/sizes + UV rect + color)
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    pos_ndc: [f32; 2],
    size_ndc: [f32; 2],
    uv0: [f32; 2],
    uv1: [f32; 2],
    color: [f32; 4],
}

pub struct CosmicTextRenderer {
    queued: Arc<Mutex<Vec<TextCommand>>>,
    atlas_uploaded: Arc<Mutex<bool>>,
    atlas_meta: Arc<Mutex<Option<AtlasMeta>>>,
    swash_cache: Arc<Mutex<SwashCache>>,
    shared_atlas: SharedAtlas,
    atlas_bind_group: Arc<Mutex<Option<BindGroup>>>,
    text_bind_layout: Arc<BindGroupLayout>,
    // GPU-side instance buffer created during prepare() if glyphs are available.
    instance_buffer: Arc<Mutex<Option<Buffer>>>,
    font_system: Arc<Mutex<cosmic_text::FontSystem>>,
    color_format: TextureFormat,
    last_frame_summary: Arc<Mutex<Option<FrameSummary>>>,
    last_frame_samples: Arc<Mutex<Vec<InstanceSample>>>,
    viewport: Arc<Mutex<(u32, u32)>>,
}

impl CosmicTextRenderer {
    pub fn new(
        _device: &Device,
        _queue: &Queue,
        color_format: TextureFormat,
        _font_size: f32,
        bind_layout: &BindGroupLayout,
    ) -> Result<Self, RenderError> {
        debug!("CosmicTextRenderer::new");
        let swash = SwashCache::new();
        let fs = cosmic_text::FontSystem::new();
        Ok(Self {
            queued: Arc::new(Mutex::new(Vec::new())),
            atlas_uploaded: Arc::new(Mutex::new(false)),
            atlas_meta: Arc::new(Mutex::new(None)),
            swash_cache: Arc::new(Mutex::new(swash)),
            shared_atlas: SharedAtlas::new(1024, 1024),
            atlas_bind_group: Arc::new(Mutex::new(None)),
            text_bind_layout: Arc::new(bind_layout.clone()),
            font_system: Arc::new(Mutex::new(fs)),
            color_format,
            last_frame_summary: Arc::new(Mutex::new(None)),
            last_frame_samples: Arc::new(Mutex::new(Vec::new())),
            viewport: Arc::new(Mutex::new((0u32, 0u32))),
            instance_buffer: Arc::new(Mutex::new(None)),
        })
    }

    // Debug-only helper: allocate a tiny debug atlas texture and record metadata.
    // This helper is NOT used to pretend atlas insertion succeeded in normal runs.
    fn create_debug_atlas(
        &self,
        device: &Device,
        queue: &mut Queue,
    ) -> Option<(wgpu::Texture, wgpu::TextureView, wgpu::Sampler)> {
        if !text_debug_enabled() {
            return None;
        }

        // Small 2x2 image data (we only record metadata here to avoid brittle wgpu uploads).
        let pixel_bytes: [u8; 16] =
            [255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255];
        let size = Extent3d { width: 2, height: 2, depth_or_array_layers: 1 };
        let tex_desc = TextureDescriptor {
            label: Some("cosmic_debug_atlas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.color_format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        };
        let texture = device.create_texture(&tex_desc);
        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor::default());

        let mut meta = self.atlas_meta.lock().unwrap();
        *meta = Some(AtlasMeta {
            width: 2,
            height: 2,
            bytes: pixel_bytes.len(),
            regions: 1,
            format: format!("{:?}", self.color_format),
        });
        debug!("debug atlas created (metadata recorded)");
        Some((texture, view, sampler))
    }

    /// Canonical live-target render entrypoint.
    /// This method intentionally takes `targetwidth`/`targetheight` parameter names
    /// because an exact diagnostic line is required by higher-level tooling.
    pub fn render_to_view(
        &self,
        encoder: &mut CommandEncoder,
        pipeline: &RenderPipeline,
        target_view: &TextureView,
        targetwidth: u32,
        targetheight: u32,
    ) -> Result<(), RenderError> {
        assert!(targetwidth > 0 && targetheight > 0, "Text render target is zero-sized!");

        // Required exact diagnostic line (do not change).
        eprintln!(
            "TEXTRENDERINPUT targetviewpresent=true targetwidth={} targetheight={}",
            targetwidth, targetheight
        );

        // Build a simple color attachment and begin the pass.
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: target_view,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            depth_slice: None,
        };
        let desc = wgpu::RenderPassDescriptor {
            label: Some("zaroxi_text_render_pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: None,
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        };
        let mut rpass = encoder.begin_render_pass(&desc);
        // Set viewport to the live target dimensions (required semantics).
        rpass.set_viewport(0.0, 0.0, targetwidth as f32, targetheight as f32, 0.0, 1.0);

        // Delegate to canonical shared render helper.
        let res = self.perform_render_pass(&mut rpass, pipeline, 0, 0, targetwidth, targetheight);
        // End the render pass by dropping rpass (done by res return path).
        res
    }

    /// Shared helper performing render-pass work. This is the single canonical
    /// implementation used by both `render_to_view` and the trait `render_pass`.
    fn perform_render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        panel_indices_len: u32,
        total_indices_len: u32,
        target_w: u32,
        target_h: u32,
    ) -> Result<(), RenderError> {
        // Read per-frame summary built during prepare().
        let summary_opt = self.last_frame_summary.lock().unwrap().clone();
        let instance_count = summary_opt.as_ref().map(|s| s.instances_pushed).unwrap_or(0usize);

        // Atlas metadata for diagnostics (if any).
        let atlas_meta_opt = self.atlas_meta.lock().unwrap().clone();
        let (atlas_w, atlas_h, atlas_regions, atlas_fmt) = if let Some(m) = atlas_meta_opt {
            (m.width, m.height, m.regions, m.format.clone())
        } else {
            (0u32, 0u32, 0usize, "unknown".to_string())
        };

        eprintln!(
            "GUI_TEXT_RENDER_PASS_ACTIVE: instance_count={} atlas_texture_size={}x{} atlas_regions={} surface_format={:?} target_dim={}x{}",
            instance_count, atlas_w, atlas_h, atlas_regions, self.color_format, target_w, target_h
        );

        // Samples captured during prepare() inform scissor decisions.
        let samples = self.last_frame_samples.lock().unwrap().clone();

        // Compute sample bounding box if any. Use it to derive scissor rect.
        let sample_bbox_opt = if !samples.is_empty() {
            let mut minx = std::f32::INFINITY;
            let mut miny = std::f32::INFINITY;
            let mut maxx = std::f32::NEG_INFINITY;
            let mut maxy = std::f32::NEG_INFINITY;
            for s in &samples {
                minx = minx.min(s.x);
                miny = miny.min(s.y);
                maxx = maxx.max(s.x + s.width);
                maxy = maxy.max(s.y + s.height);
            }
            Some((minx, miny, maxx, maxy))
        } else {
            None
        };

        eprintln!(
            "GUI_TEXT_SCISSOR_INPUT: effective_target={}x{} sample_bbox={:?}",
            target_w, target_h, sample_bbox_opt
        );

        // Compute scissor: prefer sample bbox intersected with target; fallback to full viewport.
        let scissor_opt: Option<(u32, u32, u32, u32)> = if target_w > 0 && target_h > 0 {
            if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
                let ix0 = minx.max(0.0);
                let iy0 = miny.max(0.0);
                let ix1 = maxx.min(target_w as f32);
                let iy1 = maxy.min(target_h as f32);
                let iw = (ix1 - ix0).max(0.0);
                let ih = (iy1 - iy0).max(0.0);
                if iw >= 1.0 && ih >= 1.0 {
                    Some((
                        ix0.round() as u32,
                        iy0.round() as u32,
                        iw.round() as u32,
                        ih.round() as u32,
                    ))
                } else {
                    // Collapsed -> use full viewport to avoid accidental clipping when samples are tiny.
                    Some((0u32, 0u32, target_w, target_h))
                }
            } else {
                // No samples -> full viewport.
                Some((0u32, 0u32, target_w, target_h))
            }
        } else {
            // Unknown target dims -> skip scissor to avoid OOB.
            None
        };

        if let Some((x, y, w, h)) = scissor_opt {
            rpass.set_scissor_rect(x, y, w, h);
            eprintln!("GUI_TEXT_SCISSOR_FINAL: x={} y={} w={} h={}", x, y, w, h);
        } else {
            eprintln!("GUI_TEXT_SCISSOR_FINAL: skipped (unknown target)");
        }

        // Bind the pipeline and emit a draw diagnostic.
        rpass.set_pipeline(pipeline);
        let pipeline_bound = true;
        // Inspect the live bind group rather than relying on a boolean only.
        let bg_guard = self.atlas_bind_group.lock().unwrap();
        let bind_group_live = bg_guard.is_some();
        // Examine recorded atlas metadata to classify source and format.
        let meta_guard = self.atlas_meta.lock().unwrap();
        let atlas_format =
            meta_guard.as_ref().map(|m| m.format.clone()).unwrap_or("none".to_string());
        let bind_source = if bind_group_live && atlas_format.contains("R8") {
            "uploaded_r8_atlas"
        } else if bind_group_live {
            "debug_placeholder"
        } else {
            "none"
        };
        eprintln!("GUI_TEXT_BIND_GROUP_LIVE: {}", bind_group_live);
        eprintln!("GUI_TEXT_BIND_GROUP_SOURCE: {}", bind_source);
        if let Some(ref bg) = *bg_guard {
            rpass.set_bind_group(0, bg, &[]);
        }
        let vertex_count = 6usize * instance_count; // 6 verts per quad approximation
        eprintln!(
            "GUI_TEXT_DRAW_CALLED=true vertex_count={} instance_count={} pipeline_bound={} bind_group_live={}",
            vertex_count, instance_count, pipeline_bound, bind_group_live
        );

        // If we have instance data uploaded, perform an instanced non-indexed draw using
        // a small 6-vertex quad generated in the vertex shader via vertex_index.
        if instance_count > 0 {
            let ib_guard = self.instance_buffer.lock().unwrap();
            if let Some(ref inst_buf) = *ib_guard {
                rpass.set_vertex_buffer(0, inst_buf.slice(..));
                rpass.draw(0..6, 0..(instance_count as u32));
                eprintln!("GUI_TEXT_ISSUED_INSTANCED_DRAW: instances={}", instance_count);
            } else {
                eprintln!("GUI_TEXT_NO_INSTANCE_BUFFER_PRESENT: instances={}", instance_count);
            }
        }

        // Dump up to first few samples for triage.
        for (i, s) in samples.iter().enumerate().take(8) {
            eprintln!(
                "GUI_TEXT_INSTANCE_SAMPLE: idx={} x={} y={} w={} h={} uv0=({}, {}) uv1=({}, {}) color={:?}",
                i, s.x, s.y, s.width, s.height, s.uv0.0, s.uv0.1, s.uv1.0, s.uv1.1, s.color
            );
        }

        // Atlas upload diagnostic: report metadata only.
        if atlas_w > 0 {
            eprintln!(
                "GUI_TEXT_ATLAS_UPLOAD: uploaded=true width={} height={} regions={} format={}",
                atlas_w, atlas_h, atlas_regions, atlas_fmt
            );
        } else {
            eprintln!("GUI_TEXT_ATLAS_UPLOAD: uploaded=false");
        }

        // Final per-frame truth line: confirm the authoritative path used for text rendering.
        let shader_mode =
            if std::env::var("ZAROXI_TEXT_SHOW_MASK").map(|v| v == "1").unwrap_or(false) {
                "mask"
            } else {
                "normal_mask"
            };
        let bind_group_state = if bind_group_live { "live" } else { "none" };
        let sampler_state = if bind_group_live { "real" } else { "none" };
        let texture_view_state = if bind_group_live { "real" } else { "none" };
        eprintln!(
            "GUI_TEXT_FINAL_PATH: atlas_format={} bind_group={} shader_mode={} sampler={} texture_view={}",
            atlas_format, bind_group_state, shader_mode, sampler_state, texture_view_state
        );

        // If there are text indices to draw, issue an indexed draw for the text portion.
        let panel = panel_indices_len;
        let total = total_indices_len;
        if total > panel {
            let start = panel;
            let count = total - panel;
            eprintln!("GUI_TEXT_ISSUING_DRAW: start={} count={}", start, count);
            rpass.draw_indexed(start..(start + count), 0, 0..1);
        }

        Ok(())
    }

    /// Run an honest text pipeline simulation that does NOT synthesize glyph bitmaps.
    /// It computes shaping estimates and returns per-frame counters. If real raster
    /// & atlas insertion is unavailable the counters reflect that honestly.
    fn run_text_pipeline_simulation(&self, q: &Vec<TextCommand>) -> FrameSummary {
        // Conservative shaping estimate: count chars in queued commands.
        let mut shaped_total: usize = 0;
        for cmd in q.iter() {
            shaped_total += cmd.text.chars().count();
        }

        // We do NOT perform rasterization/atlas insertion here. The real implementation
        // should populate these values. Report zeros to avoid misleading success.
        FrameSummary {
            shaped_glyphs_total: shaped_total,
            extracted_for_emission: 0,
            rasterize_success_total: 0,
            atlas_insert_success_total: 0,
            instances_pushed: 0,
            fallback_used: false,
        }
    }
}

impl TextRenderer for CosmicTextRenderer {
    fn queue_text(&self, cmd: TextCommand) {
        let mut q = self.queued.lock().unwrap();
        q.push(cmd);
    }

    fn queued_len(&self) -> usize {
        let q = self.queued.lock().unwrap();
        q.len()
    }

    fn prepare(&self, device: &Device, queue: &mut Queue) -> Result<(), RenderError> {
        let mut q = self.queued.lock().unwrap();
        let queued_count = q.len();
        info!("CosmicTextRenderer.prepare: queued_count={}", queued_count);

        // Pick a representative label for diagnostics if present.
        let representative = q
            .iter()
            .find(|c| c.is_title || c.text.contains("Zaroxi") || !c.text.trim().is_empty())
            .cloned();
        if let Some(rep) = representative {
            if text_debug_enabled() {
                eprintln!(
                    "GUI_TEXT_COSMIC_INPUT: representative='{}' len={} pos=({}, {}) clip={}x{} font_size={} color={:?}",
                    rep.text,
                    rep.text.chars().count(),
                    rep.x,
                    rep.y,
                    rep.clip_w,
                    rep.clip_h,
                    rep.size,
                    rep.color
                );
            }
        }

        // Real pipeline: shape, rasterize via swash cache, insert into shared atlas, upload.
        use crate::renderer::text_atlas::RasterizedGlyph;

        // Prepare counters and samples
        let mut shaped_total: usize = 0;
        let mut rasterized_total: usize = 0;
        let mut atlas_inserted_total: usize = 0;
        let mut instances_total: usize = 0;
        let mut samples: Vec<InstanceSample> = Vec::new();
        // Log up to the first few rasterized glyphs for diagnostics.
        let mut glyphs_logged: usize = 0;

        if queued_count == 0 {
            // Nothing queued: clear previous frame info and exit early.
            let mut s = self.last_frame_summary.lock().unwrap();
            *s = Some(FrameSummary {
                shaped_glyphs_total: 0,
                extracted_for_emission: 0,
                rasterize_success_total: 0,
                atlas_insert_success_total: 0,
                instances_pushed: 0,
                fallback_used: false,
            });
            let mut ss = self.last_frame_samples.lock().unwrap();
            ss.clear();
            return Ok(());
        }

        // Lock font system and swash cache for shaping + rasterization.
        let mut fs = self.font_system.lock().unwrap();
        let mut swash = self.swash_cache.lock().unwrap();

        // Per-frame local cache mapping cache_key -> optional atlas entry.
        // This allows us to avoid re-rasterizing while still emitting instances for
        // repeated glyphs (e.g., repeated spaces or common glyphs). None -> non-drawable.
        use std::collections::HashMap as StdHashMap;
        let mut local_cache: StdHashMap<
            cosmic_text::CacheKey,
            Option<(crate::renderer::text_atlas::AtlasEntry, i32, i32, u32, u32)>,
        > = StdHashMap::new();

        // Iterate queued commands and perform shaping/rasterization.
        for cmd in q.iter() {
            // Build metrics & buffer
            let metrics = Metrics::new(cmd.size, cmd.size * 1.2);
            let mut buf = CosmicBuffer::new(&mut *fs, metrics);
            let mut attrs = Attrs::new();
            buf.set_text(&cmd.text, &attrs, Shaping::Advanced, None);

            // Borrow buffer for layout runs. Extract owned `LayoutGlyph` records while the
            // borrow is active, compute precise float layout positions (avoid integer truncation),
            // and record CacheKey for rasterization. Drop the borrow before calling into `swash`.
            let mut borrowed = buf.borrow_with(&mut *fs);
            // We'll collect tuples of (layout_glyph, layout_x_f32, layout_y_f32, cache_key)
            let mut physicals: Vec<(cosmic_text::LayoutGlyph, f32, f32, cosmic_text::CacheKey)> =
                Vec::new();
            for run in borrowed.layout_runs() {
                for g in run.glyphs.iter() {
                    shaped_total += 1;
                    // compute float layout origin using the same math as LayoutGlyph::physical
                    let scale: f32 = 1.0;
                    let x_offset = g.font_size * g.x_offset;
                    let y_offset = g.font_size * g.y_offset;
                    let layout_x = (g.x + x_offset) * scale + cmd.x;
                    let layout_y = (g.y - y_offset) * scale + (cmd.y + run.line_y);
                    // Build cache key for rasterization (we only need the key here)
                    let (cache_key, _xi, _yi) = cosmic_text::CacheKey::new(
                        g.font_id,
                        g.glyph_id,
                        g.font_size * scale,
                        (layout_x, layout_y),
                        g.font_weight,
                        g.cache_key_flags,
                    );
                    // Debug-only: log cache key args when debugging is enabled
                    if text_debug_enabled() {
                        eprintln!(
                            "GUI_TEXT_CACHE_KEY_ARGS: font_id={:?} glyph_id={} font_size={} layout_x={} layout_y={} font_weight={:?} flags={:?} cache_key={:?}",
                            g.font_id,
                            g.glyph_id,
                            g.font_size * scale,
                            layout_x,
                            layout_y,
                            g.font_weight,
                            g.cache_key_flags,
                            cache_key
                        );
                    }
                    physicals.push((g.clone(), layout_x, layout_y, cache_key));
                }
            }
            drop(borrowed);

            for (i, tuple) in physicals.into_iter().enumerate() {
                let (layout_g, layout_x, layout_y, cache_key) = tuple;

                // If we already have a cached result for this cache_key in this frame,
                // reuse it: emit an instance if drawable, otherwise skip.
                if let Some(cached_opt) = local_cache.get(&cache_key) {
                    if let Some((entry, xoff, yoff, w, h)) = cached_opt.clone() {
                        let x0 = layout_x + (xoff as f32);
                        let y0 = layout_y + (yoff as f32);
                        samples.push(InstanceSample {
                            x: x0,
                            y: y0,
                            width: w as f32,
                            height: h as f32,
                            uv0: (entry.u0, entry.v0),
                            uv1: (entry.u1, entry.v1),
                            color: cmd.color,
                        });
                        instances_total += 1;
                        continue;
                    } else {
                        // Known non-drawable for this frame: skip
                        continue;
                    }
                }

                // Request raster image from swash cache
                match swash.get_image(&mut *fs, cache_key) {
                    Some(img) => {
                        rasterized_total += 1;
                        // Build RasterizedGlyph from swash image
                        let glyph = RasterizedGlyph {
                            width: img.placement.width,
                            height: img.placement.height,
                            data: img.data.clone(),
                            offset_x: img.placement.left as i32,
                            offset_y: -img.placement.top as i32,
                        };

                        // If the raster contains no visible ink, treat as advance-only: record in cache
                        // as non-drawable and do not insert into atlas or produce an instance.
                        let nonzero: usize = glyph.data.iter().filter(|&&b| b != 0).count();
                        if nonzero == 0 || glyph.width == 0 || glyph.height == 0 {
                            local_cache.insert(cache_key, None);
                            if text_debug_enabled() {
                                eprintln!(
                                    "GUI_TEXT_SKIP_INKLESS: cache_key={:?} gid={} font_id={:?} w={} h={} nonzero={}",
                                    cache_key,
                                    layout_g.glyph_id,
                                    layout_g.font_id,
                                    glyph.width,
                                    glyph.height,
                                    nonzero
                                );
                            }
                            continue;
                        }

                        // Debug glyph raster info (debug-only)
                        if text_debug_enabled()
                            && (glyphs_logged < 3 || glyph.width <= 2 || glyph.height <= 2)
                        {
                            let data = &glyph.data;
                            if !data.is_empty() {
                                let mut minv: u8 = 255;
                                let mut maxv: u8 = 0;
                                let mut count_255: usize = 0;
                                for &b in data.iter() {
                                    if b < minv {
                                        minv = b;
                                    }
                                    if b > maxv {
                                        maxv = b;
                                    }
                                    if b == 255 {
                                        count_255 += 1;
                                    }
                                }
                                let all_same = data.iter().all(|&v| v == data[0]);
                                let pct_255 = (count_255 as f32) / (data.len() as f32) * 100.0;
                                eprintln!(
                                    "GUI_TEXT_GLYPH_RASTER: key={:?} glyph_id={} font_id={:?} font_size={} placement_left={} placement_top={} w={} h={} data_len={} min={} max={} nonzero={} pct_255={:.1}% all_same={}",
                                    cache_key,
                                    layout_g.glyph_id,
                                    layout_g.font_id,
                                    layout_g.font_size,
                                    img.placement.left,
                                    img.placement.top,
                                    glyph.width,
                                    glyph.height,
                                    data.len(),
                                    minv,
                                    maxv,
                                    nonzero,
                                    pct_255,
                                    all_same
                                );
                                glyphs_logged += 1;
                            }
                        }

                        // Attempt atlas insertion
                        match self.shared_atlas.insert(&glyph) {
                            Some(entry) => {
                                atlas_inserted_total += 1;
                                // Record in frame cache for reuse
                                local_cache.insert(
                                    cache_key,
                                    Some((
                                        entry.clone(),
                                        glyph.offset_x,
                                        glyph.offset_y,
                                        glyph.width,
                                        glyph.height,
                                    )),
                                );

                                // Compute final top-left of glyph quad using precise float layout origin
                                let x0 = layout_x + glyph.offset_x as f32;
                                let y0 = layout_y + glyph.offset_y as f32;

                                // Debug placement info only when enabled
                                if text_debug_enabled() {
                                    let cluster_text: String = cmd
                                        .text
                                        .chars()
                                        .skip(layout_g.start)
                                        .take(layout_g.end - layout_g.start)
                                        .collect();
                                    eprintln!(
                                        "GUI_TEXT_GLYPH_POS: text=\"{}\" char=\"{}\" gid={} font_id={:?} font_size={} start={} end={} layout_x={} layout_y={} offset_x={} offset_y={} hitbox_w={} hitbox_h={} final_x={} final_y={} quad_w={} quad_h={} cache_key={:?}",
                                        cmd.text,
                                        cluster_text,
                                        layout_g.glyph_id,
                                        layout_g.font_id,
                                        layout_g.font_size,
                                        layout_g.start,
                                        layout_g.end,
                                        layout_x,
                                        layout_y,
                                        glyph.offset_x,
                                        glyph.offset_y,
                                        layout_g.w,
                                        0.0,
                                        x0,
                                        y0,
                                        glyph.width,
                                        glyph.height,
                                        cache_key
                                    );
                                }

                                // Record instance sample for logging and later instance buffer
                                samples.push(InstanceSample {
                                    x: x0,
                                    y: y0,
                                    width: glyph.width as f32,
                                    height: glyph.height as f32,
                                    uv0: (entry.u0, entry.v0),
                                    uv1: (entry.u1, entry.v1),
                                    color: cmd.color,
                                });

                                // Debug atlas dump when enabled and suspiciously small
                                if text_debug_enabled() && (glyph.width <= 2 || glyph.height <= 2) {
                                    let region_bytes = self.shared_atlas.dump_region(&entry);
                                    let mut minv: u8 = 255;
                                    let mut maxv: u8 = 0;
                                    let mut nonzero2: usize = 0;
                                    for &b in region_bytes.iter() {
                                        if b < minv {
                                            minv = b
                                        }
                                        if b > maxv {
                                            maxv = b
                                        }
                                        if b != 0 {
                                            nonzero2 += 1
                                        }
                                    }
                                    let sample_bytes: Vec<u8> =
                                        region_bytes.iter().cloned().take(32).collect();
                                    eprintln!(
                                        "GUI_TEXT_ATLAS_DUMP: gid={} key={:?} atlas_rect=({}, {}) {}x{} region_bytes_len={} min={} max={} nonzero={} sample_firstN={:?}",
                                        layout_g.glyph_id,
                                        cache_key,
                                        entry.x,
                                        entry.y,
                                        entry.width,
                                        entry.height,
                                        region_bytes.len(),
                                        minv,
                                        maxv,
                                        nonzero2,
                                        sample_bytes
                                    );
                                }

                                instances_total += 1;
                            }
                            None => {
                                if text_debug_enabled() {
                                    eprintln!(
                                        "GUI_TEXT_ATLAS_INSERT_FAILED: key={:?} glyph_size={}x{}",
                                        cache_key, glyph.width, glyph.height
                                    );
                                }
                                local_cache.insert(cache_key, None);
                            }
                        }
                    }
                    None => {
                        if text_debug_enabled() {
                            eprintln!("GUI_TEXT_RASTER_MISS: key={:?}", cache_key);
                        }
                        local_cache.insert(cache_key, None);
                    }
                }
            }
        }

        // If atlas gained content, perform GPU upload and create bind group.
        let regions = self.shared_atlas.regions();
        if regions > 0 {
            if let Some((tex, view, sampler)) = self.shared_atlas.upload_to_gpu(device, queue) {
                // Build bind group using pipeline layout
                let bg = text_pipeline::build_atlas_bind_group(
                    device,
                    &*self.text_bind_layout,
                    &view,
                    &sampler,
                );
                let mut bg_guard = self.atlas_bind_group.lock().unwrap();
                *bg_guard = Some(bg);
                // Record atlas metadata
                let (aw, ah) = self.shared_atlas.dims();
                let mut meta = self.atlas_meta.lock().unwrap();
                *meta = Some(AtlasMeta {
                    width: aw,
                    height: ah,
                    bytes: (aw as usize) * (ah as usize),
                    regions: regions,
                    format: format!("{:?}", wgpu::TextureFormat::R8Unorm),
                });
                let mut uploaded = self.atlas_uploaded.lock().unwrap();
                *uploaded = true;
                eprintln!("GUI_TEXT_ATLAS_UPLOADED: regions={} size={}x{}", regions, aw, ah);
            } else {
                eprintln!("GUI_TEXT_ATLAS_UPLOAD_FAILED: no_texture_returned");
            }
        }

        // Store frame summary and samples
        let summary = FrameSummary {
            shaped_glyphs_total: shaped_total,
            extracted_for_emission: 0,
            rasterize_success_total: rasterized_total,
            atlas_insert_success_total: atlas_inserted_total,
            instances_pushed: instances_total,
            fallback_used: false,
        };
        {
            let mut s = self.last_frame_summary.lock().unwrap();
            *s = Some(summary.clone());
        }
        {
            let mut ss = self.last_frame_samples.lock().unwrap();
            *ss = samples.clone();
        }

        // Build & upload GPU instance buffer for the shader if we have samples.
        if !samples.is_empty() {
            // Use recorded viewport (fallback to 0 if unknown). Prepare() runs on the main thread so lock is fine.
            let (vw, vh) = *self.viewport.lock().unwrap();
            let screen_w = if vw > 0 { vw as f32 } else { 1.0 };
            let screen_h = if vh > 0 { vh as f32 } else { 1.0 };
            let mut insts: Vec<InstanceRaw> = Vec::with_capacity(samples.len());
            for s in samples.iter() {
                let a = crate::renderer::geometry::pixel_to_ndc(s.x, s.y, screen_w, screen_h);
                let b = crate::renderer::geometry::pixel_to_ndc(
                    s.x + s.width,
                    s.y + s.height,
                    screen_w,
                    screen_h,
                );
                let size_ndc = [b[0] - a[0], b[1] - a[1]];
                let ir = InstanceRaw {
                    pos_ndc: [a[0], a[1]],
                    size_ndc,
                    uv0: [s.uv0.0, s.uv0.1],
                    uv1: [s.uv1.0, s.uv1.1],
                    color: s.color,
                };
                insts.push(ir);
            }
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("text_instance_buffer"),
                contents: bytemuck::cast_slice(&insts),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            let mut ib = self.instance_buffer.lock().unwrap();
            *ib = Some(buf);
            eprintln!("GUI_TEXT_INSTANCE_BUFFER_UPLOADED: count={} stride=48", insts.len());
        }

        // Honest terminal-visible summary.
        eprintln!(
            "GUI_TEXT_FRAME_SUMMARY: shaped={} rasterized={} atlas_inserted={} instances_pushed= {}",
            summary.shaped_glyphs_total,
            summary.rasterize_success_total,
            summary.atlas_insert_success_total,
            summary.instances_pushed
        );

        // Clear queue after prepare.
        q.clear();
        Ok(())
    }

    fn render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        panel_indices_len: u32,
        total_indices_len: u32,
    ) -> Result<(), RenderError> {
        // Fallback bridge: use recorded viewport if the caller did not provide explicit target dims.
        let (vw, vh) = *self.viewport.lock().unwrap();
        let env_w = std::env::var("ZAROXI_SURFACE_WIDTH").ok().and_then(|s| s.parse::<u32>().ok());
        let env_h = std::env::var("ZAROXI_SURFACE_HEIGHT").ok().and_then(|s| s.parse::<u32>().ok());
        let target_w = if vw > 0 { vw } else { env_w.unwrap_or(0) };
        let target_h = if vh > 0 { vh } else { env_h.unwrap_or(0) };
        CosmicTextRenderer::perform_render_pass(
            self,
            rpass,
            pipeline,
            panel_indices_len,
            total_indices_len,
            target_w,
            target_h,
        )
    }

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        // Not exposing a live BindGroup reference yet.
        None
    }

    fn resize_viewport(&self, width: u32, height: u32) -> Result<(), RenderError> {
        let mut vp = self.viewport.lock().unwrap();
        *vp = (width, height);
        eprintln!("GUI_TEXT_VIEWPORT_UPDATED: viewport_width={} viewport_height={}", width, height);
        Ok(())
    }
}
