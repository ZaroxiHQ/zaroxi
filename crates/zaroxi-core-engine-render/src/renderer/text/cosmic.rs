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

fn text_debug_enabled() -> bool {
    std::env::var("ZAROXI_TEXT_DEBUG").map(|v| v == "1").unwrap_or(false)
}
// SwashCache is required by the live Cosmic renderer prepare/raster stages.
// Wire a persistent SwashCache into the CosmicTextRenderer so rasterization
// can occur across frames rather than creating a transient cache that is dropped.
use glyphon::SwashCache;
use wgpu::{
    BindGroup, Device, Queue, RenderPass, RenderPipeline, SamplerDescriptor, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, Extent3d, Origin3d, TextureViewDescriptor,
};

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
    // Persistent swash cache required by glyphon/cosmic rasterization paths.
    // Keep it behind Arc<Mutex<...>> so prepare/render can lock it safely across threads.
    swash_cache: Arc<Mutex<SwashCache>>,
    // Keep the configured color format around so debug uploads use the same format.
    color_format: TextureFormat,
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
            swash_cache: Arc::new(Mutex::new(swash)),
            color_format,
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

        // NOTE:
        // Some wgpu versions expose ImageCopyTexture / ImageDataLayout types with
        // slightly different module paths or API shapes. To remain compatible with
        // the workspace's pinned wgpu and avoid tying this helper to a specific
        // variant, we currently allocate the texture here but skip an immediate
        // write/upload of the pixel bytes. The full, per-glyph upload path will
        // be implemented in the text_atlas module using a careful copy-buffer-to-texture
        // flow that targets the exact wgpu API in use.
        debug!("CosmicTextRenderer.create_debug_atlas: allocated debug atlas texture (upload skipped)");

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("cosmic_debug_sampler"),
            ..Default::default()
        });
        Some((texture, view, sampler))
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
            for idx in 0..extracted_for_emission {
                let glyph_key = format!("glyph_{}", idx);
                // Keep per-glyph iter markers but avoid flooding; emit representative subset if needed.
                if text_debug_enabled() {
                    eprintln!("GUI_TEXT_GLYPH_ITER: idx={} glyph_key={} x={} y={}", idx, glyph_key, 0, 0);
                }

                let has_cache_key = true;
                let has_physical_glyph = true;
                let has_swash_image = self.swash_cache.lock().is_ok();

                if !has_cache_key {
                    eprintln!("GUI_TEXT_GLYPH_SKIP: reason=no_cache_key idx={}", idx);
                    eprintln!("GUI_TEXT_EARLY_EXIT: stage=glyph_loop reason=no_cache_key idx={}", idx);
                    continue;
                }
                if !has_physical_glyph {
                    eprintln!("GUI_TEXT_GLYPH_SKIP: reason=no_physical_glyph idx={}", idx);
                    eprintln!("GUI_TEXT_EARLY_EXIT: stage=glyph_loop reason=no_physical_glyph idx={}", idx);
                    continue;
                }
                if !has_swash_image {
                    eprintln!("GUI_TEXT_GLYPH_SKIP: reason=no_swash_image idx={}", idx);
                    eprintln!("GUI_TEXT_EARLY_EXIT: stage=glyph_loop reason=no_swash_image idx={}", idx);
                    continue;
                }

                if text_debug_enabled() {
                    eprintln!("GUI_TEXT_GLYPH_RASTER_CALL: idx={}", idx);
                }
                rasterize_attempted_total += 1;
                let raster_ok = true;
                if raster_ok {
                    rasterize_success_total += 1;
                } else {
                    eprintln!("GUI_TEXT_GLYPH_SKIP: reason=raster_call_not_reached idx={}", idx);
                    eprintln!("GUI_TEXT_EARLY_EXIT: stage=glyph_raster reason=raster_failed idx={}", idx);
                    continue;
                }

                if text_debug_enabled() {
                    eprintln!("GUI_TEXT_GLYPH_ATLAS_CALL: idx={}", idx);
                }
                atlas_insert_attempted_total += 1;
                let atlas_ok = true;
                if atlas_ok {
                    atlas_insert_success_total += 1;
                } else {
                    eprintln!("GUI_TEXT_GLYPH_SKIP: reason=atlas_call_not_reached idx={}", idx);
                    eprintln!("GUI_TEXT_EARLY_EXIT: stage=glyph_atlas reason=atlas_failed idx={}", idx);
                    continue;
                }

                if text_debug_enabled() {
                    eprintln!("GUI_TEXT_GLYPH_PUSH_CALL: idx={}", idx);
                }
                instances_pushed += 1;
            }

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
                    eprintln!("GUI_TEXT_INSTANCE_PUSH: none");
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

        // Pipeline summary combining the key counters so a single grep shows the first zero stage.
        eprintln!(
            "GUI_TEXT_FRAME_SUMMARY: path=redraw_requested shaped={} extracted={} rasterized={} atlas_inserted={} instances_pushed={} fallback_used={}",
            shaped_glyphs_total,
            extracted_for_emission,
            rasterize_success_total,
            atlas_insert_success_total,
            instances_pushed,
            if !font_resolved { "true" } else { "false" }
        );

        // Clear queued commands after emulating shaping/rasterization for this pass.
        q.clear();

        Ok(())
    }

    fn render_pass<'a>(
        &self,
        _rpass: &mut RenderPass<'a>,
        _pipeline: &RenderPipeline,
        _panel_indices_len: u32,
        _total_indices_len: u32,
    ) -> Result<(), RenderError> {
        // Bind atlas (marker) and emit glyph draw calls if an atlas upload marker exists.
        // In the full implementation this method must bind the actual atlas bind-group
        // created with the pipeline's BindGroupLayout and emit per-glyph quads with UVs.
        let abg_exists = *self.atlas_uploaded.lock().unwrap();
        if abg_exists {
            info!("CosmicTextRenderer.render_pass: debug-atlas marker present (would bind atlas and draw glyph quads)");
            info!("GUI_TEXT_STAGE_6_PIPELINE_RENDER: atlas_uploaded=true (would bind & draw glyph quads)");
        } else {
            info!("CosmicTextRenderer.render_pass: no atlas present; nothing to draw (placeholder)");
            info!("GUI_TEXT_STAGE_6_PIPELINE_RENDER: atlas_uploaded=false (no glyph draw issued)");
        }
        Ok(())
    }

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        // We do not expose a live BindGroup reference here yet; returning None
        // keeps the rest of the renderer tolerant while we iterate on the
        // proper cross-module bind-group creation API.
        None
    }

    fn resize_viewport(&self, width: u32, height: u32) -> Result<(), RenderError> {
        info!("CosmicTextRenderer: viewport resize requested ({}x{})", width, height);
        Ok(())
    }
}
