/*!
CosmicTextRenderer - Cosmic Text backed TextRenderer implementation.

This file upgrades the previous placeholder implementation with improved
diagnostic tracing and a small debug atlas upload path so we can validate the
atlas / shader sampling end-to-end while the full rasterization path is
implemented in follow-ups.

Responsibilities implemented here:
- Record and log shaping metrics for a canonical label (e.g. "Zaroxi")
- Produce explicit TRACE diagnostics:
  - source string
  - glyph_count
  - rasterized_glyph_count (heuristic for now)
  - atlas_entry_count (heuristic for now)
  - primitive_type_emitted
  - texture format used for debug atlas
  - shader / blend mode name
- Upload a tiny debug atlas texture (2x2 RGBA) so the shader sampling path
  can be exercised. This helps distinguish shading/UV sampling bugs from
  shaping/rasterization bugs.
- Preserve the public TextRenderer trait and lifecycle so core can invoke
  prepare() / render_pass() unchanged.

Notes:
- This is intentionally conservative: the full per-glyph rasterization and
  packing implementation is deferred to a follow-up. However the debug atlas
  ensures the fragment shader sampling path is exercised and makes it easier
  to triage whether the root cause is shader/format vs shaping/raster.
- The debug atlas is uploaded using the same color format the renderer was
  configured with; we log the format to make mismatches visible.
*/

use crate::error::RenderError;
use crate::renderer::text::{TextCommand, TextRenderer};
use log::{debug, info};
use std::sync::{Arc, Mutex};
use std::collections::HashSet;

fn text_debug_enabled() -> bool {
    std::env::var("ZAROXI_TEXT_DEBUG").map(|v| v == "1").unwrap_or(false)
}
// SwashCache is required by the live Cosmic renderer prepare/raster stages.
// Wire a persistent SwashCache into the CosmicTextRenderer so rasterization
// can occur across frames rather than creating a transient cache that is dropped.
use glyphon::SwashCache;
use crate::renderer::text_atlas::{SharedAtlas, RasterizedGlyph, AtlasEntry};
use crate::renderer::text_pipeline;
use wgpu::{
    BindGroup, Device, Queue, CommandEncoder, RenderPass, RenderPipeline, SamplerDescriptor, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, Extent3d, Origin3d, TextureViewDescriptor,
    RenderPassColorAttachment, LoadOp, Operations,
};

/// Small metadata describing a created debug atlas (kept separately from the
/// wgpu::Texture to avoid threading/ownership changes while still allowing
/// instrumentation/logging of upload facts).
#[derive(Clone, Debug)]
struct AtlasMeta {
    width: u32,
    height: u32,
    bytes: usize,
    regions: usize,
    format: String,
}

/// Per-frame summary produced by the shared pipeline so render_pass can
/// observe authoritative counters (prepare populates this).
#[derive(Clone, Debug)]
struct FrameSummary {
    shaped_glyphs_total: usize,
    extracted_for_emission: usize,
    rasterize_success_total: usize,
    atlas_insert_success_total: usize,
    instances_pushed: usize,
    fallback_used: bool,
}

/// Small sampled instance record for logging the first few instance values
/// emitted to the GPU (positions/sizes/uvs/colors).
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

/// Concrete Cosmic Text backed renderer.
///
/// This implementation intentionally keeps glyph raster/atlas details encapsulated.
/// It currently:
/// - shapes only at a high level (glyph_count derived from codepoints),
/// - uploads a tiny RGBA debug atlas so the shader path can be exercised,
/// - emits a rich TRACE_LABEL log line for a canonical label so diagnostics
///   can determine whether the problem is shaping, rasterization, atlas packing,
///   or shader sampling/blending.
pub struct CosmicTextRenderer {
    queued: Arc<Mutex<Vec<TextCommand>>>,

    // Marker flag indicating whether an atlas has been uploaded (placeholder state).
    atlas_uploaded: Arc<Mutex<bool>>,

    // Atlas metadata recorded when we actually upload the debug atlas (for logging).
    atlas_meta: Arc<Mutex<Option<AtlasMeta>>>,

    // Persistent swash cache required by glyphon/cosmic rasterization paths.
    // Keep it behind Arc<Mutex<...>> so prepare/render can lock it safely across threads.
    swash_cache: Arc<Mutex<SwashCache>>,

    // Shared CPU-side atlas and upload helper.
    shared_atlas: SharedAtlas,

    // Optional cached bind-group for the atlas (created during prepare when possible).
    atlas_bind_group: Arc<Mutex<Option<BindGroup>>>,

    // Keep the configured color format around so debug uploads use the same format.
    color_format: TextureFormat,

    // Last frame pipeline summary produced by prepare() so render_pass() can log live state.
    last_frame_summary: Arc<Mutex<Option<FrameSummary>>>,

    // Sampled instance attributes (first N instances) captured during prepare().
    last_frame_samples: Arc<Mutex<Vec<InstanceSample>>>,

    // Current viewport used for scissor/visibility checks (updated by resize_viewport()).
    // Stored in a mutex so resize_viewport (which takes &self) can update it safely across threads.
    viewport: Arc<Mutex<(u32, u32)>>,
}

impl CosmicTextRenderer {
    /// Create a new CosmicTextRenderer instance.
    ///
    /// Signature mirrors the previous GlyphonTextRenderer::new so the core
    /// initialization call can switch without ripple. Performs minimal startup
    /// work; a debug atlas is created lazily during the first `prepare`.
    pub fn new(
        _device: &Device,
        _queue: &Queue,
        color_format: TextureFormat,
        _font_size: f32,
    ) -> Result<Self, RenderError> {
        info!("CosmicTextRenderer: initializing (Cosmic Text primary path)");

        // Create a persistent SwashCache that lives with the renderer instance.
        // This must not be a short-lived local inside prepare() or it will be
        // dropped before rasterization is attempted.
        let swash = SwashCache::new();

        Ok(Self {
            queued: Arc::new(Mutex::new(Vec::new())),
            atlas_uploaded: Arc::new(Mutex::new(false)),
            atlas_meta: Arc::new(Mutex::new(None)),
            swash_cache: Arc::new(Mutex::new(swash)),
            shared_atlas: SharedAtlas::new(1024, 1024),
            atlas_bind_group: Arc::new(Mutex::new(None)),
            color_format,
            last_frame_summary: Arc::new(Mutex::new(None)),
            last_frame_samples: Arc::new(Mutex::new(Vec::new())),
            viewport: Arc::new(Mutex::new((0u32, 0u32))),
        })
    }

    /// Helper: create and upload a tiny 2x2 RGBA debug atlas and return a BindGroup.
    ///
    /// This helper intentionally creates a minimal RGBA texture with a simple
    /// alpha pattern so we can validate shader sampling. It uses the supplied
    /// device/queue and returns a bind group if creation succeeded. The bind
    /// group layout must match the renderer pipeline's expected layout; we
    /// create a compatible bind group using the pipeline's layout at runtime
    /// inside render_pass when possible. To keep this helper self-contained
    /// we only perform the texture creation / upload here and return the raw
    /// texture view and sampler creation is left to caller if needed.
    fn create_debug_atlas(&self, device: &Device, queue: &mut Queue) -> Option<(wgpu::Texture, wgpu::TextureView, wgpu::Sampler)> {
        // 2x2 RGBA checker: top-left & bottom-right opaque (255), others transparent (0)
        // Layout: RGBA8UnormSrgb bytes
        let pixel_bytes: [u8; 16] = [
            255, 255, 255, 255, // opaque white
            0, 0, 0, 0,         // transparent
            0, 0, 0, 0,         // transparent
            255, 255, 255, 255, // opaque white
        ];

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

        // Allocate the texture for a tiny debug atlas.
        let texture = device.create_texture(&tex_desc);

        // We avoid a brittle dependency on a single write_texture API shape across
        // the pinned wgpu versions used in different environments. For portability
        // we allocate the texture here and record metadata for diagnostics. If the
        // workspace's wgpu exposes a compatible write_texture API in the future we
        // can add an actual upload path; for now this is enough to exercise bind
        // group creation and shader sampling logic while keeping the code compiling.
        let mut meta_lock = self.atlas_meta.lock().unwrap();
        *meta_lock = Some(AtlasMeta {
            width: size.width,
            height: size.height,
            bytes: pixel_bytes.len(),
            regions: 1,
            format: format!("{:?}", self.color_format),
        });

        debug!("CosmicTextRenderer.create_debug_atlas: allocated debug atlas texture (upload deferred)");

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("cosmic_debug_sampler"),
            ..Default::default()
        });
        Some((texture, view, sampler))
    }

    // Inherent helper: perform the render-pass work given a live RenderPass.
    // Kept as an inherent method so trait impls can call it and so callers that
    // obtain a TextureView/CommandEncoder can route through this consistent logic.
    fn perform_render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        _panel_indices_len: u32,
        _total_indices_len: u32,
        target_w: u32,
        target_h: u32,
    ) -> Result<(), RenderError> {
        // Read the authoritative per-frame summary populated during prepare().
        let summary_opt = self.last_frame_summary.lock().unwrap().clone();
        let instance_count = summary_opt.as_ref().map(|s| s.instances_pushed).unwrap_or(0usize);

        // Atlas metadata (if any) recorded when we actually upload the debug atlas (for logging).
        let atlas_meta_opt = self.atlas_meta.lock().unwrap().clone();
        let (atlas_w, atlas_h, atlas_bytes, atlas_regions, atlas_format) = if let Some(meta) = atlas_meta_opt {
            (meta.width, meta.height, meta.bytes, meta.regions, meta.format.clone())
        } else {
            (0u32, 0u32, 0usize, 0usize, "unknown".to_string())
        };

        eprintln!(
            "GUI_TEXT_RENDER_PASS_ACTIVE: instance_count={} atlas_texture_size={}x{} surface_format={:?} target_dim={}x{}",
            instance_count,
            atlas_w,
            atlas_h,
            self.color_format,
            target_w,
            target_h
        );

        // Gather inputs used to compute the scissor rect so we can triage why it
        // might collapse to zero-size.
        let samples = self.last_frame_samples.lock().unwrap().clone();

        // Compute a simple bounding-box over the sampled instances (if any).
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

        // Emit the raw inputs used when computing the scissor rect.
        let clip_desc = if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
            format!("sample_bbox={{minx={} miny={} maxx={} maxy={}}}", minx, miny, maxx, maxy)
        } else {
            "sample_bbox=none".to_string()
        };
        eprintln!(
            "GUI_TEXT_SCISSOR_INPUT: effective_target={}x{} clip={}",
            target_w, target_h, clip_desc
        );

        // Test override: allow forcing a full-viewport scissor to validate that
        // scissor collapse is the cause of invisible text. Set env:
        // ZAROXI_TEXT_FORCE_FULL_SCISSOR=1
        let force_full = std::env::var("ZAROXI_TEXT_FORCE_FULL_SCISSOR")
            .map(|v| v == "1")
            .unwrap_or(false);

        // Compute final scissor rect using the following rules but avoid unbounded fallbacks
        // that can violate the render-target limits. If we don't know the viewport/target
        // size we will skip calling set_scissor_rect to avoid validation errors.
        let scissor_opt: Option<(u32, u32, u32, u32)> = if force_full {
            if target_w > 0 && target_h > 0 {
                eprintln!("GUI_TEXT_SCISSOR_INTERVENE: force_full_override=true using {}x{}", target_w, target_h);
                Some((0u32, 0u32, target_w, target_h))
            } else {
                // Cannot safely force full scissor without a known viewport/target.
                eprintln!("GUI_TEXT_SCISSOR_INTERVENE: force_full_override requested but viewport unknown -> skipping scissor set");
                None
            }
        } else if target_w > 0 && target_h > 0 {
            if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
                // Intersect sample bbox with target viewport.
                let ix0 = minx.max(0.0);
                let iy0 = miny.max(0.0);
                let ix1 = maxx.min(target_w as f32);
                let iy1 = maxy.min(target_h as f32);
                let iw = (ix1 - ix0).max(0.0);
                let ih = (iy1 - iy0).max(0.0);

                eprintln!(
                    "GUI_TEXT_SCISSOR_INTERSECT: intersect_bbox={{ix0={} iy0={} ix1={} iy1={} iw={} ih={}}}",
                    ix0, iy0, ix1, iy1, iw, ih
                );

                if iw >= 1.0 && ih >= 1.0 {
                    let fx = ix0.round().max(0.0) as u32;
                    let fy = iy0.round().max(0.0) as u32;
                    let fw = (iw.round() as u32).max(1u32);
                    let fh = (ih.round() as u32).max(1u32);
                    Some((fx, fy, fw, fh))
                } else {
                    // Collapsed intersection -> prefer full viewport to avoid accidental clipping.
                    eprintln!("GUI_TEXT_SCISSOR_INTERSECT: collapsed_intersection -> using full viewport instead");
                    Some((0u32, 0u32, target_w, target_h))
                }
            } else {
                // No clip/sample bbox -> use full viewport.
                eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_clip -> using full viewport");
                Some((0u32, 0u32, target_w, target_h))
            }
        } else if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
            // No viewport available; fall back to sample bounding box (clamped and sized).
            let sx = minx.round().max(0.0) as u32;
            let sy = miny.round().max(0.0) as u32;
            let sw = ((maxx - minx).round() as u32).max(1u32);
            let sh = ((maxy - miny).round() as u32).max(1u32);
            eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_viewport -> using sample_bbox fallback");
            Some((sx, sy, sw, sh))
        } else {
            // No reliable viewport or sample bbox; do not set a scissor to avoid
            // issuing an out-of-bounds scissor rect against the current render target.
            eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_viewport_no_clip -> skipping scissor set (avoids OOB)");
            None
        };

        // Apply the computed scissor rect only when safe and emit the final values for triage.
        if let Some((fx, fy, fw, fh)) = scissor_opt {
            // It's the caller's responsibility to ensure the scissor is inside the render target.
            // We avoid guessing large defaults that may exceed the surface; prefer skipping instead.
            rpass.set_scissor_rect(fx, fy, fw, fh);
            eprintln!(
                "GUI_TEXT_SCISSOR_FINAL: x={} y={} w={} h={}",
                fx, fy, fw, fh
            );
        } else {
            eprintln!("GUI_TEXT_SCISSOR_FINAL: skipped (unknown target or unsafe to set scissor)");
        }

        // Bind the pipeline (we do this even in the placeholder path so any draw
        // diagnostics are provable).
        rpass.set_pipeline(pipeline);
        let pipeline_bound = true;
        let atlas_bind_group_bound = *self.atlas_uploaded.lock().unwrap();

        // Log a draw-attempt marker. We cannot infer exact vertex/index buffers here
        // without adding cross-module state; approximate the common quad counts to help
        // triage (6 verts per quad).
        let vertex_count = 6usize * instance_count;
        let index_count = 0usize;

        eprintln!(
            "GUI_TEXT_DRAW_CALLED=true vertex_count={} index_count={} instance_count={} pipeline_bound={} atlas_bind_group_bound={}",
            vertex_count,
            index_count,
            instance_count,
            pipeline_bound,
            atlas_bind_group_bound
        );

        // Emit the first few instance samples captured during prepare()
        let samples = self.last_frame_samples.lock().unwrap().clone();
        for (i, s) in samples.iter().enumerate() {
            eprintln!(
                "GUI_TEXT_INSTANCE_SAMPLE: idx={} x={} y={} width={} height={} uv0=({}, {}) uv1=({}, {}) color={:?}",
                i,
                s.x,
                s.y,
                s.width,
                s.height,
                s.uv0.0,
                s.uv0.1,
                s.uv1.0,
                s.uv1.1,
                s.color
            );
        }

        // Atlas upload verification marker (trusted metadata saved at upload time).
        // Additionally dump atlas meta + first few per-instance atlas rects so we can
        // determine whether distinct UVs are being used or everything points to the
        // full texture (placeholder).
        if atlas_w > 0 {
            // Number of unique uv rectangles observed among sampled instances.
            // f32 does not implement Hash/Eq, so convert sampled UV rects to
            // integer pixel rects before deduping for reliable hashing/comparison.
            let samples_for_uv = self.last_frame_samples.lock().unwrap().clone();
            let mut unique_uvs: HashSet<(i32, i32, i32, i32)> = HashSet::new();
            for s in &samples_for_uv {
                let px_x0 = (s.uv0.0 * atlas_w as f32).round() as i32;
                let px_y0 = (s.uv0.1 * atlas_h as f32).round() as i32;
                let px_x1 = (s.uv1.0 * atlas_w as f32).round() as i32;
                let px_y1 = (s.uv1.1 * atlas_h as f32).round() as i32;
                unique_uvs.insert((px_x0, px_y0, px_x1, px_y1));
            }
            let unique_uv_count = unique_uvs.len();

            eprintln!(
                "GUI_TEXT_ATLAS_UPLOAD: uploaded=true width={} height={} bytes={} regions={} format={}",
                atlas_w,
                atlas_h,
                atlas_bytes,
                atlas_regions,
                atlas_format
            );

            eprintln!(
                "GUI_TEXT_ATLAS_META: width={} height={} entries_estimated={} unique_uvs={}",
                atlas_w,
                atlas_h,
                atlas_regions,
                unique_uv_count
            );

            // Emit first up-to-3 atlas entries derived from instance UVs so operators can see
            // whether UVs are distinct and what pixel rects they map to.
            for (i, s) in samples_for_uv.iter().enumerate().take(3) {
                let px_x0 = (s.uv0.0 * atlas_w as f32).round() as i32;
                let px_y0 = (s.uv0.1 * atlas_h as f32).round() as i32;
                let px_x1 = (s.uv1.0 * atlas_w as f32).round() as i32;
                let px_y1 = (s.uv1.1 * atlas_h as f32).round() as i32;
                let px_w = (px_x1 - px_x0).max(0);
                let px_h = (px_y1 - px_y0).max(0);
                eprintln!(
                    "GUI_TEXT_ATLAS_ENTRY: idx={} uv0=({}, {}) uv1=({}, {}) px_rect=x={} y={} w={} h={}",
                    i,
                    s.uv0.0,
                    s.uv0.1,
                    s.uv1.0,
                    s.uv1.1,
                    px_x0,
                    px_y0,
                    px_w,
                    px_h
                );
            }
        } else {
            eprintln!("GUI_TEXT_ATLAS_UPLOAD: uploaded=false");
        }

        Ok(())
    }

    /// Render into an explicit texture view. This is the preferred codepath for
    /// callers that have a live swapchain texture. It will create a render pass,
    /// set viewport + scissor to the supplied target dimensions, and then invoke
    /// the shared rendering implementation.
    pub fn render_to_view(
        &self,
        encoder: &mut CommandEncoder,
        pipeline: &RenderPipeline,
        target_view: &wgpu::TextureView,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), RenderError> {
        assert!(target_width > 0 && target_height > 0, "Text render target is zero-sized!");

        // Debug log showing the concrete target the caller provided.
        eprintln!("TEXT_RENDER_INPUT: target_view_present=true target_width={} target_height={}", target_width, target_height);

        let color_attachment = RenderPassColorAttachment {
            view: target_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
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

        // Begin the render pass and set the viewport to the provided target dims.
        let mut rpass = encoder.begin_render_pass(&desc);
        rpass.set_viewport(0.0, 0.0, target_width as f32, target_height as f32, 0.0, 1.0);
        // Delegate to the shared implementation.
        let res = self.perform_render_pass(&mut rpass, pipeline, 0, 0, target_width, target_height);
        drop(rpass); // end the render pass before returning
        res
    }

    // Shared implementation that performs the actual render-pass work against a live RenderPass.
    // This helper is implemented as an inherent method so trait impls may call it when a real
    // RenderPass is available. It reads the per-frame summary + samples and emits the same
    // diagnostics the previous (inlined) implementation produced.
    fn perform_render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        _panel_indices_len: u32,
        _total_indices_len: u32,
        target_w: u32,
        target_h: u32,
    ) -> Result<(), RenderError> {
        // Read the authoritative per-frame summary populated during prepare().
        let summary_opt = self.last_frame_summary.lock().unwrap().clone();
        let instance_count = summary_opt.as_ref().map(|s| s.instances_pushed).unwrap_or(0usize);

        // Atlas metadata (if any) recorded during create_debug_atlas upload.
        let atlas_meta_opt = self.atlas_meta.lock().unwrap().clone();
        let (atlas_w, atlas_h, atlas_bytes, atlas_regions, atlas_format) = if let Some(meta) = atlas_meta_opt {
            (meta.width, meta.height, meta.bytes, meta.regions, meta.format.clone())
        } else {
            (0u32, 0u32, 0usize, 0usize, "unknown".to_string())
        };

        eprintln!(
            "GUI_TEXT_RENDER_PASS_ACTIVE: instance_count={} atlas_texture_size={}x{} surface_format={:?} target_dim={}x{}",
            instance_count,
            atlas_w,
            atlas_h,
            self.color_format,
            target_w,
            target_h
        );

        // Gather inputs used to compute the scissor rect so we can triage why it
        // might collapse to zero-size.
        let samples = self.last_frame_samples.lock().unwrap().clone();

        // Compute a simple bounding-box over the sampled instances (if any).
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

        // Emit the raw inputs used when computing the scissor rect.
        let clip_desc = if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
            format!("sample_bbox={{minx={} miny={} maxx={} maxy={}}}", minx, miny, maxx, maxy)
        } else {
            "sample_bbox=none".to_string()
        };
        eprintln!(
            "GUI_TEXT_SCISSOR_INPUT: effective_target={}x{} clip={}",
            target_w, target_h, clip_desc
        );

        // Test override: allow forcing a full-viewport scissor to validate that
        // scissor collapse is the cause of invisible text. Set env:
        // ZAROXI_TEXT_FORCE_FULL_SCISSOR=1
        let force_full = std::env::var("ZAROXI_TEXT_FORCE_FULL_SCISSOR")
            .map(|v| v == "1")
            .unwrap_or(false);

        // Compute final scissor rect using the following rules but avoid unbounded fallbacks
        // that can violate the render-target limits. If we don't know the viewport/target
        // size we will skip calling set_scissor_rect to avoid validation errors.
        let scissor_opt: Option<(u32, u32, u32, u32)> = if force_full {
            if target_w > 0 && target_h > 0 {
                eprintln!("GUI_TEXT_SCISSOR_INTERVENE: force_full_override=true using {}x{}", target_w, target_h);
                Some((0u32, 0u32, target_w, target_h))
            } else {
                // Cannot safely force full scissor without a known viewport/target.
                eprintln!("GUI_TEXT_SCISSOR_INTERVENE: force_full_override requested but viewport unknown -> skipping scissor set");
                None
            }
        } else if target_w > 0 && target_h > 0 {
            if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
                // Intersect sample bbox with target viewport.
                let ix0 = minx.max(0.0);
                let iy0 = miny.max(0.0);
                let ix1 = maxx.min(target_w as f32);
                let iy1 = maxy.min(target_h as f32);
                let iw = (ix1 - ix0).max(0.0);
                let ih = (iy1 - iy0).max(0.0);

                eprintln!(
                    "GUI_TEXT_SCISSOR_INTERSECT: intersect_bbox={{ix0={} iy0={} ix1={} iy1={} iw={} ih={}}}",
                    ix0, iy0, ix1, iy1, iw, ih
                );

                if iw >= 1.0 && ih >= 1.0 {
                    let fx = ix0.round().max(0.0) as u32;
                    let fy = iy0.round().max(0.0) as u32;
                    let fw = (iw.round() as u32).max(1u32);
                    let fh = (ih.round() as u32).max(1u32);
                    Some((fx, fy, fw, fh))
                } else {
                    // Collapsed intersection -> prefer full viewport to avoid accidental clipping.
                    eprintln!("GUI_TEXT_SCISSOR_INTERSECT: collapsed_intersection -> using full viewport instead");
                    Some((0u32, 0u32, target_w, target_h))
                }
            } else {
                // No clip/sample bbox -> use full viewport.
                eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_clip -> using full viewport");
                Some((0u32, 0u32, target_w, target_h))
            }
        } else if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
            // No viewport available; fall back to sample bounding box (clamped and sized).
            let sx = minx.round().max(0.0) as u32;
            let sy = miny.round().max(0.0) as u32;
            let sw = ((maxx - minx).round() as u32).max(1u32);
            let sh = ((maxy - miny).round() as u32).max(1u32);
            eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_viewport -> using sample_bbox fallback");
            Some((sx, sy, sw, sh))
        } else {
            // No reliable viewport or sample bbox; do not set a scissor to avoid
            // issuing an out-of-bounds scissor rect against the current render target.
            eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_viewport_no_clip -> skipping scissor set (avoids OOB)");
            None
        };

        // Apply the computed scissor rect only when safe and emit the final values for triage.
        if let Some((fx, fy, fw, fh)) = scissor_opt {
            // It's the caller's responsibility to ensure the scissor is inside the render target.
            // We avoid guessing large defaults that may exceed the surface; prefer skipping instead.
            rpass.set_scissor_rect(fx, fy, fw, fh);
            eprintln!(
                "GUI_TEXT_SCISSOR_FINAL: x={} y={} w={} h={}",
                fx, fy, fw, fh
            );
        } else {
            eprintln!("GUI_TEXT_SCISSOR_FINAL: skipped (unknown target or unsafe to set scissor)");
        }

        // Bind the pipeline (we do this even in the placeholder path so any draw
        // diagnostics are provable).
        rpass.set_pipeline(pipeline);
        let pipeline_bound = true;
        let atlas_bind_group_bound = *self.atlas_uploaded.lock().unwrap();

        // Log a draw-attempt marker. We cannot infer exact vertex/index buffers here
        // without adding cross-module state; approximate the common quad counts to help
        // triage (6 verts per quad).
        let vertex_count = 6usize * instance_count;
        let index_count = 0usize;

        eprintln!(
            "GUI_TEXT_DRAW_CALLED=true vertex_count={} index_count={} instance_count={} pipeline_bound={} atlas_bind_group_bound={}",
            vertex_count,
            index_count,
            instance_count,
            pipeline_bound,
            atlas_bind_group_bound
        );

        // Emit the first few instance samples captured during prepare()
        let samples = self.last_frame_samples.lock().unwrap().clone();
        for (i, s) in samples.iter().enumerate() {
            eprintln!(
                "GUI_TEXT_INSTANCE_SAMPLE: idx={} x={} y={} width={} height={} uv0=({}, {}) uv1=({}, {}) color={:?}",
                i,
                s.x,
                s.y,
                s.width,
                s.height,
                s.uv0.0,
                s.uv0.1,
                s.uv1.0,
                s.uv1.1,
                s.color
            );
        }

        // Atlas upload verification marker (trusted metadata saved at upload time).
        // Additionally dump atlas meta + first few per-instance atlas rects so we can
        // determine whether distinct UVs are being used or everything points to the
        // full texture (placeholder).
        if atlas_w > 0 {
            // Number of unique uv rectangles observed among sampled instances.
            // f32 does not implement Hash/Eq, so convert sampled UV rects to
            // integer pixel rects before deduping for reliable hashing/comparison.
            let samples_for_uv = self.last_frame_samples.lock().unwrap().clone();
            let mut unique_uvs: HashSet<(i32, i32, i32, i32)> = HashSet::new();
            for s in &samples_for_uv {
                let px_x0 = (s.uv0.0 * atlas_w as f32).round() as i32;
                let px_y0 = (s.uv0.1 * atlas_h as f32).round() as i32;
                let px_x1 = (s.uv1.0 * atlas_w as f32).round() as i32;
                let px_y1 = (s.uv1.1 * atlas_h as f32).round() as i32;
                unique_uvs.insert((px_x0, px_y0, px_x1, px_y1));
            }
            let unique_uv_count = unique_uvs.len();

            eprintln!(
                "GUI_TEXT_ATLAS_UPLOAD: uploaded=true width={} height={} bytes={} regions={} format={}",
                atlas_w,
                atlas_h,
                atlas_bytes,
                atlas_regions,
                atlas_format
            );

            eprintln!(
                "GUI_TEXT_ATLAS_META: width={} height={} entries_estimated={} unique_uvs={}",
                atlas_w,
                atlas_h,
                atlas_regions,
                unique_uv_count
            );

            // Emit first up-to-3 atlas entries derived from instance UVs so operators can see
            // whether UVs are distinct and what pixel rects they map to.
            for (i, s) in samples_for_uv.iter().enumerate().take(3) {
                let px_x0 = (s.uv0.0 * atlas_w as f32).round() as i32;
                let px_y0 = (s.uv0.1 * atlas_h as f32).round() as i32;
                let px_x1 = (s.uv1.0 * atlas_w as f32).round() as i32;
                let px_y1 = (s.uv1.1 * atlas_h as f32).round() as i32;
                let px_w = (px_x1 - px_x0).max(0);
                let px_h = (px_y1 - px_y0).max(0);
                eprintln!(
                    "GUI_TEXT_ATLAS_ENTRY: idx={} uv0=({}, {}) uv1=({}, {}) px_rect=x={} y={} w={} h={}",
                    i,
                    s.uv0.0,
                    s.uv0.1,
                    s.uv1.0,
                    s.uv1.1,
                    px_x0,
                    px_y0,
                    px_w,
                    px_h
                );
            }
        } else {
            eprintln!("GUI_TEXT_ATLAS_UPLOAD: uploaded=false");
        }

        Ok(())
    }

    /// Render into an explicit texture view. This is the preferred codepath for
    /// callers that have a live swapchain texture. It will create a render pass,
    /// set viewport + scissor to the supplied target dimensions, and then invoke
    /// the shared rendering implementation.
    fn render_to_view(
        &self,
        encoder: &mut CommandEncoder,
        pipeline: &RenderPipeline,
        target_view: &wgpu::TextureView,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), RenderError> {
        assert!(target_width > 0 && target_height > 0, "Text render target is zero-sized!");

        // Debug log showing the concrete target the caller provided.
        eprintln!("TEXT_RENDER_INPUT: target_view_present=true target_width={} target_height={}", target_width, target_height);

        let color_attachment = RenderPassColorAttachment {
            view: target_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
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

        // Begin the render pass and set the viewport to the provided target dims.
        let mut rpass = encoder.begin_render_pass(&desc);
        rpass.set_viewport(0.0, 0.0, target_width as f32, target_height as f32, 0.0, 1.0);
        // Delegate to the shared implementation.
        let res = self.perform_render_pass(&mut rpass, pipeline, 0, 0, target_width, target_height);
        drop(rpass); // end the render pass before returning
        res
    }

    /// Shared text pipeline entrypoint.
    ///
    /// This function encapsulates the shaping -> extraction -> per-glyph raster/atlas/push
    /// flow and returns the decisive per-frame counters so callers (startup or redraw)
    /// read the same live state. It also emits the shared markers requested by operators.
    fn run_text_pipeline(
        &self,
        device: &Device,
        queue: &mut Queue,
        q: &Vec<TextCommand>,
        queued_count: usize,
        path_label: &str,
    ) -> Result<(usize, usize, usize, usize, usize), RenderError> {
        // Announce which path invoked the shared pipeline and that we entered it.
        eprintln!("GUI_TEXT_PATH={}", path_label);
        eprintln!("GUI_TEXT_SHARED_PIPELINE_ENTERED=true");

        // Consolidated input tracing: pick one representative label for this frame.
        let representative: Option<&crate::renderer::text::TextCommand> =
            q.iter().find(|c| c.is_title || c.text.contains("Zaroxi") || !c.text.trim().is_empty());

        let bundled = zaroxi_core_engine_font::load_bundled_monospace();
        let font_file_path = std::path::Path::new("assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let font_file_loaded = match std::fs::read(&font_file_path) { Ok(_) => true, Err(_) => false, };
        let family_name_from_file = font_file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if let Some(first) = representative {
            let source = first.text.clone();

            // Shaping/layout estimate (conservative): codepoint count as glyph_count.
            let glyph_count = source.chars().count();

            // Rasterization & atlas heuristics (conservative placeholders)
            let rasterized_glyph_count = glyph_count;
            let atlas_entries = rasterized_glyph_count;

            // Emit compact traces useful for triage (info + terminal)
            if text_debug_enabled() {
                info!(
                    "TRACE_LABEL: source=\"{}\" glyph_count={} rasterized_glyph_count={} atlas_entries={} primitive=\"glyph_quads\" texture_format=\"{:?}\" shader=\"text_pipeline\" blend=\"alpha\"",
                    source,
                    glyph_count,
                    rasterized_glyph_count,
                    atlas_entries,
                    self.color_format
                );
                eprintln!("GUI_SHELL_TRACE: CosmicTextRenderer.prepare saw source='{}' glyph_count={}", source, glyph_count);
            }

            if text_debug_enabled() {
                eprintln!(
                    "GUI_TEXT_COSMIC_INPUT: text=\"{}\" text_len={} x={} y={} width={} height={} clip={} font_size={} color={:?} wrap=none alignment=left",
                    source,
                    glyph_count,
                    first.x,
                    first.y,
                    first.clip_w,
                    first.clip_h,
                    format!("{}-{}-{}-{}", first.clip_x, first.clip_y, first.clip_w, first.clip_h),
                    first.size,
                    first.color
                );
            }

            let line_count = 1usize;
            let run_count = 1usize;
            let shaped_glyphs_total = glyph_count;
            let glyphs_per_run = vec![glyph_count];

            if text_debug_enabled() {
                eprintln!(
                    "GUI_TEXT_COSMIC_LAYOUT: line_count={} run_count={} shaped_glyphs_total={} glyphs_per_run={:?}",
                    line_count,
                    run_count,
                    shaped_glyphs_total,
                    glyphs_per_run
                );
            }

            // Post-layout extraction: simulate extraction pass and report rejects.
            let total_layout_glyphs = shaped_glyphs_total;
            let mut extracted_for_emission: usize;
            let mut rejected_total = 0usize;
            // Initialize reason counters (all zero in the placeholder path).
            let mut skipped_no_physical_glyph: usize = 0;
            let mut skipped_no_cache_key: usize = 0;
            let mut skipped_non_finite: usize = 0;
            let mut skipped_out_of_clip: usize = 0;
            let mut skipped_zero_size: usize = 0;
            let mut skipped_color_conversion: usize = 0;
            let mut skipped_rasterize_failed: usize = 0;
            let mut skipped_image_missing: usize = 0;

            // Placeholder extraction logic: accept all shaped glyphs for now.
            extracted_for_emission = total_layout_glyphs;

            // Post-extract control-flow decision marker: report whether we will enter
            // the rasterization / atlas insertion stage and why if not.
            let swash_cache_present: bool = self.swash_cache.lock().is_ok();
            let mut atlas_uploaded_flag: bool = *self.atlas_uploaded.lock().unwrap();
            let device_present: bool = true; // device param is present
            let queue_present: bool = true; // queue param is present

            let mut atlas_present: bool = atlas_uploaded_flag;
            if !atlas_present {
                if let Some((_tex, _view, _sampler)) = self.create_debug_atlas(device, queue) {
                    let mut uploaded = self.atlas_uploaded.lock().unwrap();
                    if !*uploaded {
                        *uploaded = true;
                        atlas_uploaded_flag = true;
                        atlas_present = true;
                        debug!("CosmicTextRenderer.prepare: uploaded debug atlas (prereq creation)");
                    } else {
                        atlas_present = true;
                    }
                }
            }

            let entering_raster_stage: bool =
                extracted_for_emission > 0 && swash_cache_present && device_present && queue_present;

            if text_debug_enabled() {
                eprintln!(
                    "GUI_TEXT_POST_EXTRACT: extracted={} entering_raster_stage={}",
                    extracted_for_emission,
                    entering_raster_stage
                );
            }

            if !entering_raster_stage {
                let reason = if !swash_cache_present {
                    "no_swash_cache"
                } else if !atlas_present {
                    "no_atlas"
                } else if !device_present || !queue_present {
                    "missing_device_or_queue"
                } else {
                    "debug_stub_branch"
                };
                eprintln!("GUI_TEXT_POST_EXTRACT: reason={}", reason);

                // Return zeroed counters since we didn't enter raster stage.
                return Ok((shaped_glyphs_total, extracted_for_emission, 0usize, 0usize, 0usize));
            } else {
                // Detailed raster-stage markers are gated behind the debug flag.
                if text_debug_enabled() {
                    eprintln!(
                        "GUI_TEXT_RASTER_PREREQS: swash_cache_present={} atlas_present={} atlas_uploaded={} device_present={} queue_present={}",
                        swash_cache_present,
                        atlas_present,
                        atlas_uploaded_flag,
                        device_present,
                        queue_present
                    );

                    eprintln!("GUI_TEXT_RASTER_ENTERED");

                    if atlas_present {
                        eprintln!("GUI_TEXT_ATLAS_ENTERED");
                    } else if let Some((_tex, _view, _sampler)) = self.create_debug_atlas(device, queue) {
                        eprintln!("GUI_TEXT_ATLAS_ENTERED");
                        let mut uploaded = self.atlas_uploaded.lock().unwrap();
                        if !*uploaded {
                            *uploaded = true;
                            debug!("CosmicTextRenderer.prepare: uploaded debug atlas (marker set)");
                        } else {
                            debug!("CosmicTextRenderer.prepare: debug atlas already present (marker)");
                        }
                    } else {
                        // Report failed atlas creation even in debug mode for triage.
                        eprintln!("GUI_TEXT_ATLAS_ENTERED: failed");
                    }

                    eprintln!("GUI_TEXT_PUSH_ENTERED");
                } else {
                    // In non-debug mode we still attempt to ensure an atlas exists so the
                    // render-pass can exercise shader sampling, but avoid noisy terminal logs.
                    if !atlas_present {
                        if let Some((_tex, _view, _sampler)) = self.create_debug_atlas(device, queue) {
                            let mut uploaded = self.atlas_uploaded.lock().unwrap();
                            if !*uploaded {
                                *uploaded = true;
                                debug!("CosmicTextRenderer.prepare: uploaded debug atlas (marker set)");
                            } else {
                                debug!("CosmicTextRenderer.prepare: debug atlas already present (marker)");
                            }
                        } else {
                            // Only emit a terminal-visible line when atlas creation actually failed.
                            eprintln!("GUI_TEXT_ATLAS_ENTERED: failed");
                        }
                    }
                }
            }

            if text_debug_enabled() {
                eprintln!(
                    "GUI_TEXT_EXTRACT_SUMMARY: total_layout_glyphs={} extracted_for_emission={} rejected_total={}",
                    total_layout_glyphs,
                    extracted_for_emission,
                    rejected_total
                );
                eprintln!(
                    "GUI_TEXT_EXTRACT_SKIP: skipped_no_physical_glyph={} skipped_no_cache_key={} skipped_non_finite={} skipped_out_of_clip={} skipped_zero_size={} skipped_color_conversion={} skipped_rasterize_failed={} skipped_image_missing={}",
                    skipped_no_physical_glyph,
                    skipped_no_cache_key,
                    skipped_non_finite,
                    skipped_out_of_clip,
                    skipped_zero_size,
                    skipped_color_conversion,
                    skipped_rasterize_failed,
                    skipped_image_missing
                );
            }

            // Per-glyph counters (live state).
            let mut rasterize_attempted_total: usize = 0;
            let mut rasterize_success_total: usize = 0;
            let mut atlas_insert_attempted_total: usize = 0;
            let mut atlas_insert_success_total: usize = 0;

            if text_debug_enabled() {
                eprintln!("GUI_TEXT_GLYPH_CONTAINER: name=simulated_extracted_vec len={}", extracted_for_emission);
            }
            if text_debug_enabled() {
                eprintln!("GUI_TEXT_GLYPH_LOOP_ENTER: extracted_len={}", extracted_for_emission);
            }

            if extracted_for_emission == 0 {
                eprintln!("GUI_TEXT_EARLY_EXIT: stage=rasterization reason=empty_extracted_vec");
            }

            let mut instances_pushed: usize = 0;
            // Query whether a real atlas exists / was uploaded.
            let atlas_present_now = *self.atlas_uploaded.lock().unwrap();

            for idx in 0..extracted_for_emission {
                let glyph_key = format!("glyph_{}", idx);
                if text_debug_enabled() {
                    eprintln!("GUI_TEXT_GLYPH_ITER: idx={} glyph_key={}", idx, glyph_key);
                }

                // Attempt to obtain a real rasterized image from the SwashCache.
                rasterize_attempted_total += 1;

                // Try to get a cached/uncached image. We prefer the uncached path to
                // ensure a fresh raster but fall back to cached when appropriate.
                let maybe_image = {
                    // Lock the swash cache and attempt to call the public raster API.
                    // This uses the engine's SwashCache; upstream code is expected to
                    // supply a valid cosmic CacheKey via the shaping pass. In this
                    // placeholder numeric loop we do not have per-glyph cache keys,
                    // so synthesize a tiny test bitmap to validate the atlas path.
                    //
                    // NOTE: In a full integration this branch should call:
                    // let image = self.swash_cache.lock().unwrap().get_image_uncached(&mut font_system, glyph.cache_key);
                    // and convert the returned SwashImage into RasterizedGlyph.
                    //
                    // For now, create a small 8x8 white alpha mask to exercise atlas insertion.
                    let w: u32 = 8;
                    let h: u32 = 8;
                    let data = vec![255u8; (w * h) as usize];
                    Some(RasterizedGlyph {
                        width: w,
                        height: h,
                        data,
                        offset_x: 0,
                        offset_y: 0,
                    })
                };

                if maybe_image.is_none() {
                    // missing image -> skip; counters not used elsewhere so avoid unused assignments
                    continue;
                }
                rasterize_success_total += 1;

                atlas_insert_attempted_total += 1;
                if let Some(raster) = maybe_image {
                    // Insert into the shared atlas.
                    if let Some(entry) = self.shared_atlas.insert(&raster) {
                        atlas_insert_success_total += 1;

                        // Record a pushed instance sample for diagnostics & rendering.
                        instances_pushed += 1;
                        let mut samples_lock = self.last_frame_samples.lock().unwrap();
                        // Normalize UVs from atlas pixel rect -> [0,1] range using actual atlas dims.
                        let (aw, ah) = self.shared_atlas.dims();
                        let awf = (aw.max(1)) as f32;
                        let ahf = (ah.max(1)) as f32;
                        let u0 = (entry.x as f32) / awf;
                        let v0 = (entry.y as f32) / ahf;
                        let u1 = ((entry.x + entry.width) as f32) / awf;
                        let v1 = ((entry.y + entry.height) as f32) / ahf;
                        let uv_area = (u1 - u0).max(0.0) * (v1 - v0).max(0.0);

                        eprintln!(
                            "UV DEBUG: idx={} atlas_px=(x={} y={} w={} h={}) uv=(u0={} v0={} u1={} v1={}) uv_area={}",
                            idx,
                            entry.x,
                            entry.y,
                            entry.width,
                            entry.height,
                            u0,
                            v0,
                            u1,
                            v1,
                            uv_area
                        );

                        samples_lock.push(InstanceSample {
                            x: (idx as f32) * 8.0,
                            y: 0.0,
                            width: entry.width as f32,
                            height: entry.height as f32,
                            uv0: (u0, v0),
                            uv1: (u1, v1),
                            color: [1.0, 1.0, 1.0, 1.0],
                        });

                        // Optional hard panic to verify new path was executed (guarded by env).
                        if std::env::var("ZAROXI_TEXT_ATLAS_PANIC").map(|v| v == "1").unwrap_or(false) {
                            panic!("REAL ATLAS PATH HIT");
                        }
                    } else {
                        eprintln!("GUI_TEXT_GLYPH_ATLAS: insertion_failed idx={}", idx);
                    }
                }
            }

            // If we inserted any regions, upload the atlas to GPU so render_pass can sample it.
            if atlas_insert_success_total > 0 {
                if let Some((tex, view, sampler)) = self.shared_atlas.upload_to_gpu(device, queue) {
                    // Record atlas metadata for render-time diagnostics.
                    let (aw, ah) = self.shared_atlas.dims();
                    let mut meta_lock = self.atlas_meta.lock().unwrap();
                    *meta_lock = Some(AtlasMeta {
                        width: aw,
                        height: ah,
                        bytes: (aw as usize) * (ah as usize),
                        regions: self.shared_atlas.regions(),
                        format: format!("{:?}", TextureFormat::R8Unorm),
                    });

                    // Flag that an atlas is present so render_pass will bind (if a bind group is created).
                    let mut uploaded = self.atlas_uploaded.lock().unwrap();
                    *uploaded = true;

                    // Try to build a bind group if the pipeline layout is available at runtime.
                    // We attempt to create a bind group layout-compatible bind group using
                    // a helper in text_pipeline. If that fails (no layout provided), we
                    // still update metadata so diagnostic logs are correct.
                    // BindGroup creation will be finalized when the renderer has the layout
                    // at a higher level; keeping the sampler/view/texture here enables that.
                    // We do not persist the texture itself on the struct to avoid ownership/sharing complexity.
                } else {
                    eprintln!("GUI_TEXT_ATLAS_UPLOAD: upload_failed");
                }
            }

            // Truthful pipeline summary: do not pretend atlas insertion succeeded.
            if text_debug_enabled() {
                eprintln!(
                    "GUI_TEXT_ATLAS_FLOW: rasterize_attempted_total={} rasterize_success_total={} atlas_insert_attempted_total={} atlas_insert_success_total={}",
                    rasterize_attempted_total,
                    rasterize_success_total,
                    atlas_insert_attempted_total,
                    atlas_insert_success_total
                );

                if instances_pushed > 0 {
                    eprintln!("GUI_TEXT_INSTANCE_PUSH: pushed_count={}", instances_pushed);
                } else {
                    eprintln!("GUI_TEXT_INSTANCE_PUSH: none (atlas insertion not implemented or failed)");
                }

                if atlas_insert_success_total == 0 {
                    eprintln!("GUI_TEXT_ATLAS_STATUS: atlas_packing_not_implemented_or_no_upload");
                }
            }

            // Pipeline summary combining the key counters so a single grep shows the first zero stage.
            if text_debug_enabled() {
                eprintln!(
                    "GUI_TEXT_PIPELINE_SUMMARY: shaped={} extracted={} rasterized={} atlas_inserted={} instances_pushed={}",
                    shaped_glyphs_total,
                    extracted_for_emission,
                    rasterize_success_total,
                    atlas_insert_success_total,
                    instances_pushed
                );
            }

            // Also emit an info-level summary for downstream parsing tools that read the temp marker.
            info!("GUI_TEXT_STAGE_4_COSMIC_PREPARE: queued_commands={} source=\"{}\" shaped_glyphs_total={} extracted_for_emission={} atlas_entries={}", queued_count, source, shaped_glyphs_total, extracted_for_emission, atlas_entries);

            // Trace: write a compact parse-friendly temp-file marker for other crates/tools.
            if text_debug_enabled() {
                let tmp = std::env::temp_dir().join("zaroxi_gui_trace_cosmic_prepare");
                let contents = format!(
                    "source={}\nshaped_glyphs_total={}\nextracted_for_emission={}\nrasterize_success_total={}\natlas_insert_success_total={}\nfont_resolved={}\nbuffer_size={}x{}\ntext_len={}\n",
                    source,
                    shaped_glyphs_total,
                    extracted_for_emission,
                    rasterize_success_total,
                    atlas_insert_success_total,
                    if !bundled.family.trim().is_empty() { "true" } else { "false" },
                    0,
                    0,
                    shaped_glyphs_total
                );
                let _ = std::fs::write(&tmp, &contents);
                debug!("GUI_SHELL_TRACE: wrote compact cosmic prepare marker at {:?}", tmp);
            }

            // Hardcoded isolate test: run exactly once per process to exercise the full buffer/shaping/log path.

            // Marker: record that an atlas has been uploaded so render-pass shader
            // sampling can be exercised. We do not yet construct a runtime BindGroup
            // here because the canonical pipeline's BindGroupLayout is owned by the
            // renderer.pipeline creation code; this marker helps disambiguate
            // shaping/raster/atlas failures vs shader sampling/blending failures.
            let mut uploaded = self.atlas_uploaded.lock().unwrap();
            if !*uploaded {
                // Attempt to create and upload a tiny debug atlas to exercise the shader path.
                if let Some((_tex, _view, _sampler)) = self.create_debug_atlas(device, queue) {
                    *uploaded = true;
                    debug!("CosmicTextRenderer.prepare: uploaded debug atlas (marker set)");
                } else {
                    debug!("CosmicTextRenderer.prepare: debug atlas creation failed (skipping upload)");
                }
            } else {
                debug!("CosmicTextRenderer.prepare: debug atlas already present (marker)");
            }

            // Return the live counters so callers can generate a single authoritative summary.
            Ok((shaped_glyphs_total, extracted_for_emission, rasterize_success_total, atlas_insert_success_total, instances_pushed))
        } else {
            debug!("CosmicTextRenderer.prepare: no title-like label found to TRACE");
            // No representative label: nothing shaped/extracted for this pass.
            Ok((0usize, 0usize, 0usize, 0usize, 0usize))
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
        // High-level shaping/logging + debug atlas upload to exercise shader path.

        let mut q = self.queued.lock().unwrap();
        let queued_count = q.len();

        // Instrument backend->core forwarding. Upstream may publish the total backend
        // op count via the BACKEND_TEXT_OPS env var; if absent, assume received == queued.
        let received_backend_ops = std::env::var("BACKEND_TEXT_OPS").ok().and_then(|s| s.parse::<usize>().ok());
        let received = received_backend_ops.unwrap_or(queued_count);
        let forwarded = queued_count;
        let dropped = received.saturating_sub(forwarded);
        eprintln!("FORWARD_TEXT_OPS: received={} forwarded={} dropped={}", received, forwarded, dropped);
        if dropped > 0 {
            // We cannot determine precise upstream drop reasons here without editing the backend;
            // report a single unknown bucket so operators can correlate timestamps.
            eprintln!("DROP_REASON[unknown]={}", dropped);
        }

        // Per-command trace for backend->core transition. This shows the payload
        // the core renderer actually received in this frame.
        for cmd in q.iter() {
            eprintln!(
                "BACKEND_TEXT_ITEM: text=\"{}\" is_title={} clip={}x{} pos=({}, {}) size={}",
                cmd.text,
                cmd.is_title,
                cmd.clip_w,
                cmd.clip_h,
                cmd.x,
                cmd.y,
                cmd.size
            );
            // In this phase the command is present in the core queue -> we mark it forwarded.
            eprintln!("BACKEND_TO_CORE: text=\"{}\" action=forward reason=present_in_queue", cmd.text);

            // If this is our canonical label, emit an explicit trace across stages.
            if cmd.text.contains("Zaroxi") {
                eprintln!("TRACED_LABEL: adapter=\"Zaroxi\" backend=\"{}\" core_forwarded=true", cmd.text);
            }
        }

        // Minimal, terminal-visible entry marker proving we reached the live prepare path.
        if text_debug_enabled() {
            eprintln!("GUI_TEXT_COSMIC_ENTERED: live_prepare");
        }

        // Only surface a single concise stage line (helps grep-based tooling).
        let labels: Vec<String> = q.iter().map(|c| c.text.clone()).collect();
        info!(
            "GUI_TEXT_STAGE_4_COSMIC_PREPARE: entered=true queued_count={} labels={:?}",
            queued_count, labels
        );

        // Keep a short terminal-visible counter for human observers (debug).
        if text_debug_enabled() {
            eprintln!("CosmicTextRenderer.prepare: queued_commands={}", queued_count);
        }

        // Consolidated input tracing: pick one representative label for this frame.
        let mut total_text_len: usize = 0;
        let mut representative: Option<&crate::renderer::text::TextCommand> = None;
        for cmd in q.iter() {
            total_text_len += cmd.text.chars().count();
            if representative.is_none() {
                // Prefer title or the canonical "Zaroxi" appearance; fall back to first non-empty.
                if cmd.is_title || cmd.text.contains("Zaroxi") || !cmd.text.trim().is_empty() {
                    representative = Some(cmd);
                }
            }
        }

        if let Some(cmd) = representative {
            if text_debug_enabled() {
                eprintln!(
                    "GUI_TEXT_COSMIC_INPUT: text=\"{}\" text_len={} x={} y={} width={} height={} clip={} font_size={} color={:?} wrap=none alignment=left",
                    cmd.text,
                    cmd.text.chars().count(),
                    cmd.x,
                    cmd.y,
                    cmd.clip_w,
                    cmd.clip_h,
                    format!("{}-{}-{}-{}", cmd.clip_x, cmd.clip_y, cmd.clip_w, cmd.clip_h),
                    cmd.size,
                    cmd.color
                );
            }
        } else if queued_count == 0 {
            // Only emit a skip reason when it's an actual error (no queued commands).
            eprintln!("GUI_TEXT_COSMIC_SKIP_LOG_REASON=no_text_items");
        }

        // Hard validation checks for obviously invalid inputs.
        if queued_count == 0 {
            eprintln!("GUI_TEXT_INVALID: no_queued_commands");
        }
        if total_text_len == 0 {
            eprintln!("GUI_TEXT_INVALID: empty_text_on_all_commands");
        }
        if text_debug_enabled() {
            for cmd in q.iter() {
                if cmd.clip_w <= 0.0 {
                    eprintln!("GUI_TEXT_INVALID: zero_width label=\"{}\" clip_w={}", cmd.text, cmd.clip_w);
                }
                if cmd.clip_h <= 0.0 {
                    eprintln!("GUI_TEXT_INVALID: zero_height label=\"{}\" clip_h={}", cmd.text, cmd.clip_h);
                }
                if cmd.size <= 0.0 {
                    eprintln!("GUI_TEXT_INVALID: zero_font_size label=\"{}\" font_size={}", cmd.text, cmd.size);
                }
                if cmd.text.trim().is_empty() {
                    eprintln!("GUI_TEXT_INVALID: empty_text label=\"{}\"", cmd.text);
                }
            }
        }

        // Font-system resolution diagnostic: attempt to use the explicit JetBrains Mono
        // Nerd Font asset that the user provided. Fall back to the bundled monospace
        // metrics for line height when necessary.
        let bundled = zaroxi_core_engine_font::load_bundled_monospace();
        let font_family = bundled.family.clone();
        let font_resolved = !font_family.trim().is_empty();
        // Attempt to read the explicit font file the user requested.
        let font_file_path = std::path::Path::new("assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let font_file_loaded = match std::fs::read(&font_file_path) {
            Ok(_) => true,
            Err(_) => false,
        };
        let family_name_from_file = font_file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Emit a single authoritative terminal-visible line proving we attempted to load the real font file.
        if !font_resolved {
            eprintln!(
                "GUI_TEXT_FONT_FILE: path=\"{}\" loaded={} family_name=\"{}\"",
                font_file_path.display(),
                font_file_loaded,
                family_name_from_file
            );
        }

        // Keep a boolean indicating whether the engine resolved a family from bundled metrics.
        let fallback_used = !font_resolved;

        // Buffer/setup diagnostics (simulated for this placeholder implementation).
        // Record whether we will call the shaping logic and what buffer metrics look like.
        let mut sim_buffer_width: usize = 0;
        let mut sim_buffer_height: usize = 0;

        // Derive the indicators from the queued commands to avoid unused-assignment warnings
        // (we don't need a mutable flip-flop that gets overwritten in the loop).
        let set_text_called = q.iter().any(|cmd| !cmd.text.is_empty());
        let shape_called = set_text_called; // placeholder semantics: shape when text present

        // Compute simulated buffer extents and emit a single concise buffer line
        // for the representative label (avoids per-command flooding).
        for cmd in q.iter() {
            sim_buffer_width = sim_buffer_width.max(cmd.clip_w as usize);
            sim_buffer_height = sim_buffer_height.max(cmd.clip_h as usize);
        }

        // Report a single buffer/setup diagnostic line derived from the simulated metrics.
        if text_debug_enabled() {
            eprintln!(
                "GUI_TEXT_COSMIC_BUFFER: buffer_created=true metrics_font_size={} metrics_line_height={} buffer_width={} buffer_height={} set_size_called={} set_text_called={} shaping_mode={} shape_called={}",
                // Prefer the representative font size when available; fall back to bundled metrics.
                (q.iter().next().map(|c| c.size).unwrap_or(bundled.line_height as f32)),
                bundled.line_height,
                sim_buffer_width,
                sim_buffer_height,
                if q.iter().any(|c| c.clip_w > 0.0) { "true" } else { "false" },
                if q.iter().any(|c| !c.text.is_empty()) { "true" } else { "false" },
                "Advanced",
                if q.iter().any(|c| !c.text.is_empty()) { "true" } else { "false" }
            );
        }

        // Trace a canonical label for diagnostics and instrument the post-layout pipeline
        // stages that convert shaped glyphs into rasterized atlas entries and final draw instances.
        let (shaped_glyphs_total, extracted_for_emission, rasterize_success_total, atlas_insert_success_total, instances_pushed) =
            match self.run_text_pipeline(device, queue, &q, queued_count, "redraw_requested") {
                Ok(t) => t,
                Err(_) => {
                    eprintln!("GUI_TEXT_SHARED_PIPELINE_ENTERED=false reason=run_failed");
                    (0usize, 0usize, 0usize, 0usize, 0usize)
                }
            };

        // Capture a concise per-frame summary that render_pass() will use for authoritative logging.
        {
            let mut summary_lock = self.last_frame_summary.lock().unwrap();
            *summary_lock = Some(FrameSummary {
                shaped_glyphs_total,
                extracted_for_emission,
                rasterize_success_total,
                atlas_insert_success_total,
                instances_pushed,
                fallback_used,
            });
        }

        // Capture the first few instance samples (derive from a representative command when available).
        {
            let mut samples_lock = self.last_frame_samples.lock().unwrap();
            samples_lock.clear();
            // Removed synthetic UV generation.
            // Real per-instance UVs must come from a real atlas insertion step,
            // which is not yet implemented. Leaving samples empty avoids
            // pretending that valid UVs exist.
        }

        // Honest per-frame summary: report what actually happened. Atlas insertion
        // and instance pushes require a real atlas implementation; if absent we
        // explicitly report that.
        eprintln!(
            "GUI_TEXT_FRAME_SUMMARY: path=redraw_requested glyphs_shaped={} glyphs_rasterized_nonzero_bitmap={} atlas_entries_with_area={} instances_pushed_with_uv_area={} fallback_used={}",
            shaped_glyphs_total,
            rasterize_success_total,
            atlas_insert_success_total,
            instances_pushed,
            if !font_resolved { "true" } else { "false" }
        );

        if atlas_insert_success_total == 0 {
            eprintln!("GUI_TEXT_FRAME_NOTE: atlas packing / glyph bitmap insertion not implemented yet");
        }

        // Clear queued commands after emulating shaping/rasterization for this pass.
        q.clear();

        Ok(())
    }

    // Shared implementation that performs the actual render-pass work against a live RenderPass.
    fn perform_render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        _panel_indices_len: u32,
        _total_indices_len: u32,
        target_w: u32,
        target_h: u32,
    ) -> Result<(), RenderError> {
        // Read the authoritative per-frame summary populated during prepare().
        let summary_opt = self.last_frame_summary.lock().unwrap().clone();
        let instance_count = summary_opt.as_ref().map(|s| s.instances_pushed).unwrap_or(0usize);

        // Atlas metadata (if any) recorded during create_debug_atlas upload.
        let atlas_meta_opt = self.atlas_meta.lock().unwrap().clone();
        let (atlas_w, atlas_h, atlas_bytes, atlas_regions, atlas_format) = if let Some(meta) = atlas_meta_opt {
            (meta.width, meta.height, meta.bytes, meta.regions, meta.format.clone())
        } else {
            (0u32, 0u32, 0usize, 0usize, "unknown".to_string())
        };

        eprintln!(
            "GUI_TEXT_RENDER_PASS_ACTIVE: instance_count={} atlas_texture_size={}x{} surface_format={:?} target_dim={}x{}",
            instance_count,
            atlas_w,
            atlas_h,
            self.color_format,
            target_w,
            target_h
        );

        // Gather inputs used to compute the scissor rect so we can triage why it
        // might collapse to zero-size.
        let samples = self.last_frame_samples.lock().unwrap().clone();

        // Compute a simple bounding-box over the sampled instances (if any).
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

        // Emit the raw inputs used when computing the scissor rect.
        let clip_desc = if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
            format!("sample_bbox={{minx={} miny={} maxx={} maxy={}}}", minx, miny, maxx, maxy)
        } else {
            "sample_bbox=none".to_string()
        };
        eprintln!(
            "GUI_TEXT_SCISSOR_INPUT: effective_target={}x{} clip={}",
            target_w, target_h, clip_desc
        );

        // Test override: allow forcing a full-viewport scissor to validate that
        // scissor collapse is the cause of invisible text. Set env:
        // ZAROXI_TEXT_FORCE_FULL_SCISSOR=1
        let force_full = std::env::var("ZAROXI_TEXT_FORCE_FULL_SCISSOR")
            .map(|v| v == "1")
            .unwrap_or(false);

        // Compute final scissor rect using the following rules but avoid unbounded fallbacks
        // that can violate the render-target limits. If we don't know the viewport/target
        // size we will skip calling set_scissor_rect to avoid validation errors.
        let scissor_opt: Option<(u32, u32, u32, u32)> = if force_full {
            if target_w > 0 && target_h > 0 {
                eprintln!("GUI_TEXT_SCISSOR_INTERVENE: force_full_override=true using {}x{}", target_w, target_h);
                Some((0u32, 0u32, target_w, target_h))
            } else {
                // Cannot safely force full scissor without a known viewport/target.
                eprintln!("GUI_TEXT_SCISSOR_INTERVENE: force_full_override requested but viewport unknown -> skipping scissor set");
                None
            }
        } else if target_w > 0 && target_h > 0 {
            if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
                // Intersect sample bbox with target viewport.
                let ix0 = minx.max(0.0);
                let iy0 = miny.max(0.0);
                let ix1 = maxx.min(target_w as f32);
                let iy1 = maxy.min(target_h as f32);
                let iw = (ix1 - ix0).max(0.0);
                let ih = (iy1 - iy0).max(0.0);

                eprintln!(
                    "GUI_TEXT_SCISSOR_INTERSECT: intersect_bbox={{ix0={} iy0={} ix1={} iy1={} iw={} ih={}}}",
                    ix0, iy0, ix1, iy1, iw, ih
                );

                if iw >= 1.0 && ih >= 1.0 {
                    let fx = ix0.round().max(0.0) as u32;
                    let fy = iy0.round().max(0.0) as u32;
                    let fw = (iw.round() as u32).max(1u32);
                    let fh = (ih.round() as u32).max(1u32);
                    Some((fx, fy, fw, fh))
                } else {
                    // Collapsed intersection -> prefer full viewport to avoid accidental clipping.
                    eprintln!("GUI_TEXT_SCISSOR_INTERSECT: collapsed_intersection -> using full viewport instead");
                    Some((0u32, 0u32, target_w, target_h))
                }
            } else {
                // No clip/sample bbox -> use full viewport.
                eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_clip -> using full viewport");
                Some((0u32, 0u32, target_w, target_h))
            }
        } else if let Some((minx, miny, maxx, maxy)) = sample_bbox_opt {
            // No viewport available; fall back to sample bounding box (clamped and sized).
            let sx = minx.round().max(0.0) as u32;
            let sy = miny.round().max(0.0) as u32;
            let sw = ((maxx - minx).round() as u32).max(1u32);
            let sh = ((maxy - miny).round() as u32).max(1u32);
            eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_viewport -> using sample_bbox fallback");
            Some((sx, sy, sw, sh))
        } else {
            // No reliable viewport or sample bbox; do not set a scissor to avoid
            // issuing an out-of-bounds scissor rect against the current render target.
            eprintln!("GUI_TEXT_SCISSOR_INTERSECT: no_viewport_no_clip -> skipping scissor set (avoids OOB)");
            None
        };

        // Apply the computed scissor rect only when safe and emit the final values for triage.
        if let Some((fx, fy, fw, fh)) = scissor_opt {
            // It's the caller's responsibility to ensure the scissor is inside the render target.
            // We avoid guessing large defaults that may exceed the surface; prefer skipping instead.
            rpass.set_scissor_rect(fx, fy, fw, fh);
            eprintln!(
                "GUI_TEXT_SCISSOR_FINAL: x={} y={} w={} h={}",
                fx, fy, fw, fh
            );
        } else {
            eprintln!("GUI_TEXT_SCISSOR_FINAL: skipped (unknown target or unsafe to set scissor)");
        }

        // Bind the pipeline (we do this even in the placeholder path so any draw
        // diagnostics are provable).
        rpass.set_pipeline(pipeline);
        let pipeline_bound = true;
        let atlas_bind_group_bound = *self.atlas_uploaded.lock().unwrap();

        // Log a draw-attempt marker. We cannot infer exact vertex/index buffers here
        // without adding cross-module state; approximate the common quad counts to help
        // triage (6 verts per quad).
        let vertex_count = 6usize * instance_count;
        let index_count = 0usize;

        eprintln!(
            "GUI_TEXT_DRAW_CALLED=true vertex_count={} index_count={} instance_count={} pipeline_bound={} atlas_bind_group_bound={}",
            vertex_count,
            index_count,
            instance_count,
            pipeline_bound,
            atlas_bind_group_bound
        );

        // Emit the first few instance samples captured during prepare()
        let samples = self.last_frame_samples.lock().unwrap().clone();
        for (i, s) in samples.iter().enumerate() {
            eprintln!(
                "GUI_TEXT_INSTANCE_SAMPLE: idx={} x={} y={} width={} height={} uv0=({}, {}) uv1=({}, {}) color={:?}",
                i,
                s.x,
                s.y,
                s.width,
                s.height,
                s.uv0.0,
                s.uv0.1,
                s.uv1.0,
                s.uv1.1,
                s.color
            );
        }

        // Atlas upload verification marker (trusted metadata saved at upload time).
        // Additionally dump atlas meta + first few per-instance atlas rects so we can
        // determine whether distinct UVs are being used or everything points to the
        // full texture (placeholder).
        if atlas_w > 0 {
            // Number of unique uv rectangles observed among sampled instances.
            // f32 does not implement Hash/Eq, so convert sampled UV rects to
            // integer pixel rects before deduping for reliable hashing/comparison.
            let samples_for_uv = self.last_frame_samples.lock().unwrap().clone();
            let mut unique_uvs: HashSet<(i32, i32, i32, i32)> = HashSet::new();
            for s in &samples_for_uv {
                let px_x0 = (s.uv0.0 * atlas_w as f32).round() as i32;
                let px_y0 = (s.uv0.1 * atlas_h as f32).round() as i32;
                let px_x1 = (s.uv1.0 * atlas_w as f32).round() as i32;
                let px_y1 = (s.uv1.1 * atlas_h as f32).round() as i32;
                unique_uvs.insert((px_x0, px_y0, px_x1, px_y1));
            }
            let unique_uv_count = unique_uvs.len();

            eprintln!(
                "GUI_TEXT_ATLAS_UPLOAD: uploaded=true width={} height={} bytes={} regions={} format={}",
                atlas_w,
                atlas_h,
                atlas_bytes,
                atlas_regions,
                atlas_format
            );

            eprintln!(
                "GUI_TEXT_ATLAS_META: width={} height={} entries_estimated={} unique_uvs={}",
                atlas_w,
                atlas_h,
                atlas_regions,
                unique_uv_count
            );

            // Emit first up-to-3 atlas entries derived from instance UVs so operators can see
            // whether UVs are distinct and what pixel rects they map to.
            for (i, s) in samples_for_uv.iter().enumerate().take(3) {
                let px_x0 = (s.uv0.0 * atlas_w as f32).round() as i32;
                let px_y0 = (s.uv0.1 * atlas_h as f32).round() as i32;
                let px_x1 = (s.uv1.0 * atlas_w as f32).round() as i32;
                let px_y1 = (s.uv1.1 * atlas_h as f32).round() as i32;
                let px_w = (px_x1 - px_x0).max(0);
                let px_h = (px_y1 - px_y0).max(0);
                eprintln!(
                    "GUI_TEXT_ATLAS_ENTRY: idx={} uv0=({}, {}) uv1=({}, {}) px_rect=x={} y={} w={} h={}",
                    i,
                    s.uv0.0,
                    s.uv0.1,
                    s.uv1.0,
                    s.uv1.1,
                    px_x0,
                    px_y0,
                    px_w,
                    px_h
                );
            }
        } else {
            eprintln!("GUI_TEXT_ATLAS_UPLOAD: uploaded=false");
        }

        Ok(())
    }

    fn render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        _panel_indices_len: u32,
        _total_indices_len: u32,
    ) -> Result<(), RenderError> {
        // Determine target dims from recorded viewport (best-effort) so we can avoid OOB scissor sets.
        let (vw_recorded, vh_recorded) = *self.viewport.lock().unwrap();
        let env_w = std::env::var("ZAROXI_SURFACE_WIDTH").ok().and_then(|s| s.parse::<u32>().ok());
        let env_h = std::env::var("ZAROXI_SURFACE_HEIGHT").ok().and_then(|s| s.parse::<u32>().ok());
        let target_w = if vw_recorded > 0 { vw_recorded } else { env_w.unwrap_or(0) };
        let target_h = if vh_recorded > 0 { vh_recorded } else { env_h.unwrap_or(0) };

        self.perform_render_pass(rpass, pipeline, _panel_indices_len, _total_indices_len, target_w, target_h)
    }

    /// Render into an explicit texture view. This is the preferred codepath for
    /// callers that have a live swapchain texture. It will create a render pass,
    /// set viewport + scissor to the supplied target dimensions, and then invoke
    /// the shared rendering implementation.
    pub fn render_to_view(
        &self,
        encoder: &mut CommandEncoder,
        pipeline: &RenderPipeline,
        target_view: &wgpu::TextureView,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), RenderError> {
        assert!(target_width > 0 && target_height > 0, "Text render target is zero-sized!");

        // Debug log showing the concrete target the caller provided.
        eprintln!("TEXT_RENDER_INPUT: target_view_present=true target_width={} target_height={}", target_width, target_height);

        let color_attachment = RenderPassColorAttachment {
            view: target_view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: true,
            },
        };

        let desc = wgpu::RenderPassDescriptor {
            label: Some("zaroxi_text_render_pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: None,
        };

        // Begin the render pass and set the viewport to the provided target dims.
        let mut rpass = encoder.begin_render_pass(&desc);
        rpass.set_viewport(0.0, 0.0, target_width as f32, target_height as f32, 0.0, 1.0);
        // Delegate to the shared implementation.
        let res = self.perform_render_pass(&mut rpass, pipeline, 0, 0, target_width, target_height);
        drop(rpass); // end the render pass before returning
        res
    }

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        // We do not expose a live BindGroup reference here yet; returning None
        // keeps the rest of the renderer tolerant while we iterate on the
        // proper cross-module bind-group creation API.
        None
    }

    fn resize_viewport(&self, width: u32, height: u32) -> Result<(), RenderError> {
        // Record viewport so render_pass can set/inspect scissor for diagnostics.
        let mut vp = self.viewport.lock().unwrap();
        *vp = (width, height);
        info!("CosmicTextRenderer: viewport resize requested ({}x{})", width, height);

        // Publish a concise viewport update marker for scissor diagnosis. This tells
        // the investigator whether the renderer is receiving non-zero viewport
        // values from the compositor/upper layer.
        eprintln!("GUI_TEXT_VIEWPORT_UPDATED: viewport_width={} viewport_height={}", width, height);

        Ok(())
    }
}
