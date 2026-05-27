use crate::error::RenderError;
use crate::renderer::geometry::color_to_rgba;
#[cfg(feature = "legacy_cosmic")]
use crate::renderer::text::{FontAtlas, GlyphInfo, PlacedGlyph};
use log::{debug, info};
use std::collections::HashMap;
use std::num::NonZeroU32;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue};

/* explicit re-exports expected by the backend code; cosmic-text crate is
provided as "cosmic-text" in Cargo.toml but the Rust module path is
`cosmic_text`. Import the commonly used types here so the file uses them
directly. */
#[cfg(feature = "legacy_cosmic")]
use cosmic_text::{Buffer, FontSystem, SwashCache};

/// A minimal backend boundary trait for text shaping/layout/rasterization.
///
/// Implementations are responsible for:
/// - shaping & layout (producing placed glyphs with pixel coordinates + atlas UVs)
/// - rasterization / atlas interactions (atlas is internal to the backend)
///
/// The renderer consumes only placed glyphs produced by a backend instance.
#[cfg(feature = "legacy_cosmic")]
pub trait TextBackend: Send + Sync {
    /// Layout text clipped to a pixel rectangle. Returns placed glyphs in
    /// pixel coordinates ready for placement conversion into GPU vertices.
    ///
    /// The backend is allowed to perform rasterization and upload into its
    /// internal atlas using the provided queue. The renderer passes a mutable
    /// reference to the queue so the backend can perform GPU uploads.
    fn layout_text_clipped(
        &self,
        queue: &mut Queue,
        x: f32,
        y: f32,
        text: &str,
        color: [f32; 4],
        screen_w: f32,
        screen_h: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Result<Vec<PlacedGlyph>, RenderError>;

    /// Return a reference to the backend-managed atlas bind group (if available)
    /// so the renderer can bind it for the text pass.
    fn atlas_bind_group(&self) -> Option<&BindGroup>;
}

//
// CosmicTextBackend
//
// Implements TextBackend using cosmic-text as the source of shaping/layout
// and baseline/metric information. Glyph rasterization is backed by cosmic-text's
// swash cache and the backend populates an internal GPU atlas on demand.
//
// This file now includes a small FontPolicy layer that makes explicit the
// font family preferences and fallback chains used by the backend. The policy
// is intentionally small and configurable so the backend can request text
// layout using an explicit role (UI, Mono, Symbols) rather than assuming a
// single implicit font for everything.
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Roles used by the renderer to request different font fallbacks.
#[derive(Debug, Clone, Copy)]
pub enum FontRole {
    /// Default UI text (menus, labels).
    Ui,
    /// Monospace text for editor/terminal.
    Mono,
    /// Symbol / Nerd Font-compatible fallbacks for icons/glyphs.
    Symbols,
}

/// Lightweight font policy describing preferred families and fallback chains.
///
/// The policy intentionally stores family names (strings) so it can be adapted
/// to different platforms and asset layouts without coupling to cosmic-text's
/// internal types here. The backend remains responsible for registering font
/// bytes with cosmic-text and for using cosmic-text's fallback facilities at
/// layout time.
#[derive(Debug, Clone)]
pub struct FontPolicy {
    pub ui_families: Vec<String>,
    pub mono_families: Vec<String>,
    pub symbol_families: Vec<String>,
}

impl FontPolicy {
    /// Construct a default policy, optionally seeding known workspace asset names
    /// discovered relative to CARGO_MANIFEST_DIR. This function does not mutate
    /// the cosmic-text FontSystem; it only produces a configuration the backend
    /// will consult when shaping/layout is requested.
    pub fn default_with_assets(manifest_dir: &str) -> Self {
        let mut ui = vec![
            "Inter".to_string(),
            "Noto Sans".to_string(),
            "Segoe UI".to_string(),
            "Arial".to_string(),
        ];
        let mut mono = vec![
            "JetBrains Mono".to_string(),
            "Fira Code".to_string(),
            "Menlo".to_string(),
            "Courier New".to_string(),
        ];
        let mut symbols = vec![
            // prefer known Nerd Font naming patterns but allow graceful degradation
            "JetBrainsMono Nerd Font".to_string(),
            "Nerd Font".to_string(),
            "Segoe UI Symbol".to_string(),
            "Symbola".to_string(),
        ];

        // If the workspace includes a bundled Nerd-style font, prefer it.
        let fonts_dir = PathBuf::from(manifest_dir).join("../../assets/fonts");
        let candidate = fonts_dir.join("JetBrainsMonoNerdFont-Regular.ttf");
        if candidate.exists() {
            // push the exact asset filename as a hint so consumers can register it
            // or use its bytes when initializing the FontSystem.
            let name = "JetBrainsMonoNerdFont-Regular".to_string();
            // Prefer bundled asset early in the mono and symbol lists.
            mono.insert(0, name.clone());
            symbols.insert(0, name);
        }

        Self { ui_families: ui, mono_families: mono, symbol_families: symbols }
    }

    /// Return the family list for a particular role in preference order.
    pub fn families_for_role(&self, role: FontRole) -> &[String] {
        match role {
            FontRole::Ui => &self.ui_families,
            FontRole::Mono => &self.mono_families,
            FontRole::Symbols => &self.symbol_families,
        }
    }
}

static LAYOUT_LOG_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn cosmic_color_to_rgba(c: cosmic_text::Color) -> [f32; 4] {
    // cosmic_text::Color is a newtype over u32 with packed RGBA.
    let rgba = c.0;
    let r = ((rgba >> 24) & 0xff) as f32 / 255.0;
    let g = ((rgba >> 16) & 0xff) as f32 / 255.0;
    let b = ((rgba >> 8) & 0xff) as f32 / 255.0;
    let a = (rgba & 0xff) as f32 / 255.0;
    [r, g, b, a]
}

#[cfg(feature = "legacy_cosmic")]
pub struct CosmicTextBackend {
    // cosmic-text's FontSystem is the shaping/layout/fallback engine.
    // Wrapped in a Mutex so the TextBackend can borrow it mutably while
    // `layout_text_clipped` is called through an `&self` reference.
    font_system: Mutex<cosmic_text::FontSystem>,
    // swash-backed raster cache from cosmic-text (used to rasterize glyph bitmaps)
    swash_cache: Mutex<cosmic_text::SwashCache>,
    // GPU atlas and associated metadata (managed by the backend)
    atlas: FontAtlas,
    // Mapping from a stable cache key -> glyph placement/meta in the atlas.
    // Key encodes glyph identity + raster-size-related inputs.
    glyph_cache_keys: Mutex<HashMap<u64, GlyphInfo>>,
    // Font selection/fallback policy used by this backend.
    font_policy: FontPolicy,

    // Diagnostics: whether the bundled font was successfully registered.
    bundled_font_loaded: bool,
    // Resolved family name discovered from fontdb (if any).
    resolved_family: Option<String>,
}

#[cfg(feature = "legacy_cosmic")]
impl CosmicTextBackend {
    /// Create a new CosmicTextBackend and create an empty GPU atlas using
    /// the provided bind group layout so the backend can upload glyphs on-demand.
    pub fn new(
        device: &Device,
        queue: &Queue,
        layout: &BindGroupLayout,
        font_size: f32,
    ) -> Result<Self, RenderError> {
        // Initialize FontSystem and register the bundled TTF into a fontdb
        // database so cosmic-text can resolve the exact family name we intend to use.
        // This explicitly prefers the workspace-bundled JetBrainsMono Nerd Font
        // as the primary UI/mono family.
        let mut fs = cosmic_text::FontSystem::new();

        // Try to register bundled font using fontdb and attach the database to the FontSystem.
        // This uses fontdb 0.23 compatible APIs.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path =
            PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let mut db = fontdb::Database::new();
        let mut bundled_loaded = false;
        if font_path.exists() {
            // Use fontdb::Database::load_font_file to register the on-disk TTF with the database.
            // This API returns Result<(), std::io::Error>.
            match db.load_font_file(&font_path) {
                Ok(()) => {
                    bundled_loaded = true;
                    debug!(
                        "CosmicTextBackend: bundled font loaded into fontdb from '{}'",
                        font_path.display()
                    );
                }
                Err(e) => {
                    debug!(
                        "CosmicTextBackend: fontdb failed to load bundled font '{}': {:?}",
                        font_path.display(),
                        e
                    );
                }
            }
        } else {
            debug!("CosmicTextBackend: bundled font not found at '{}'", font_path.display());
        }

        // NOTE: we keep the fontdb::Database locally (db) and do not attempt to
        // attach it into FontSystem via non-portable helpers. We will query `db`
        // directly below to determine the resolved family name for diagnostics.

        // Build a default font policy. This captures preferred family names and
        // a symbol/nerd-font fallback chain. The policy is purely a configuration
        // object; the FontSystem remains the authoritative shaping/fallback engine.
        let font_policy = FontPolicy::default_with_assets(".");

        // Initialize swash cache (cosmic-text wrapper that exposes swash rasterization).
        let swash_cache = Mutex::new(cosmic_text::SwashCache::new());

        // Determine the exact family name that will be used for the bundled font.
        // Prefer "JetBrainsMono Nerd Font" but query the attached font database to
        // discover the resolved family name to use in attributes/queries.
        let mut resolved_family: Option<String> = None;
        {
            // Query the local fontdb::Database we created above to discover registered families.
            // Collect discovered family names from faces. FaceInfo.families is Vec<(String, Language)>;
            // use the first family name for each face as the primary discovered family.
            let matches: Vec<String> = db
                .faces()
                .filter_map(|face| face.families.get(0).map(|(name, _lang)| name.clone()))
                .collect();

            if matches.iter().any(|m| m == "JetBrainsMono Nerd Font") {
                resolved_family = Some("JetBrainsMono Nerd Font".to_string());
            } else if !matches.is_empty() {
                resolved_family = Some(matches[0].clone());
            }

            debug!("CosmicTextBackend: fontdb discovered families (sample) = {:?}", matches);
        }

        if let Some(ref fam) = resolved_family {
            debug!("CosmicTextBackend: using resolved bundled family '{}'", fam);
        } else {
            debug!(
                "CosmicTextBackend: no bundled family resolved; will attempt to use default FontSystem fallbacks"
            );
        }

        // Create an empty GPU atlas that the backend will populate on demand.
        let atlas = FontAtlas::new_empty(device, queue, layout, font_size)?;

        Ok(Self {
            font_system: Mutex::new(fs),
            swash_cache,
            atlas,
            glyph_cache_keys: Mutex::new(HashMap::new()),
            font_policy,
            bundled_font_loaded: bundled_loaded,
            resolved_family,
        })
    }

    /// Build a stable cache key for a shaped glyph. The key must include:
    ///  - a glyph identity (cluster/shape id)
    ///  - the rasterization size (font size)
    ///  - any subpixel or raster-alignment inputs (here we include round(y*64))
    fn glyph_cache_key(glyph_id: u32, font_size: f32, subpixel_y: i32) -> u64 {
        let a = glyph_id as u64;
        let b = (font_size.to_bits() as u64) << 32;
        let c = (subpixel_y as u64) & 0xffffffff;
        a ^ b ^ c
    }
}

#[cfg(feature = "legacy_cosmic")]
impl TextBackend for CosmicTextBackend {
    fn layout_text_clipped(
        &self,
        _queue: &mut Queue,
        x: f32,
        y: f32,
        text: &str,
        color: [f32; 4],
        _screen_w: f32,
        _screen_h: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Result<Vec<PlacedGlyph>, RenderError> {
        // Diagnostic gated logging: avoid spamming every frame. We allow a small
        // number of initial logs to help diagnose the first several distinct
        // text emission events after process start.
        let should_log = LAYOUT_LOG_COUNTER.fetch_add(1, Ordering::SeqCst) < 8;

        if should_log {
            info!(
                "CosmicTextBackend: bundled_font_loaded={} resolved_family={}",
                self.bundled_font_loaded,
                self.resolved_family.as_deref().unwrap_or("None"),
            );
            info!(
                "CosmicTextBackend: incoming text=\"{}\" font_size={}",
                text, self.atlas.font_size
            );
            debug!(
                "CosmicTextBackend: note: using cosmic-text layout + swash rasterization for rendering"
            );
        }

        // Validate clip rectangle semantics early: callers MUST pass (x, y, width, height).
        // A zero-or-negative width/height indicates a caller bug or a flipped rect.
        // In that case we bail out early to avoid producing invalid glyph placements
        // or performing wasted rasterization work.
        if clip_w <= 0.0 || clip_h <= 0.0 {
            if should_log {
                info!(
                    "CosmicTextBackend: empty_or_invalid_clip: x={}, y={}, w={}, h={}",
                    clip_x, clip_y, clip_w, clip_h
                );
            }
            // Return empty placement list (nothing to draw).
            return Ok(Vec::new());
        }

        // Output glyphs and counters
        let mut out: Vec<PlacedGlyph> = Vec::new();
        let mut pen_x = x;
        let mut rasterized_count: usize = 0usize;
        let mut layout_glyphs: usize = 0usize;
        let mut missing_glyphs: usize = 0usize;

        // Small clip tolerance to avoid accidental rejection due to 1px rounding/hinting.
        // This is a renderer-level, conservative tolerance to make placement robust across
        // minor rounding differences between layout and atlas placement.
        let clip_epsilon: f32 = 1.0;
        let clip_x_e = clip_x - clip_epsilon;
        let clip_y_e = clip_y - clip_epsilon;
        let clip_w_e = clip_w + clip_epsilon * 2.0;
        let clip_h_e = clip_h + clip_epsilon * 2.0;

        // Active cosmic-text layout/rasterization path:
        // - Use Buffer to shape/layout the string into layout runs
        // - Iterate runs/glyphs, request swash raster images
        // - Insert raster into backend atlas (upload) and produce placed glyphs
        //
        // This path is gated by the diagnostic counter to avoid spam but will run
        // for all calls — the logging itself is gated.
        let mut swash_images_obtained = 0usize;
        let mut atlas_insert_attempts = 0usize;
        let mut atlas_insert_succeeded = 0usize;
        let mut produced_placed = 0usize;
        let mut layout_runs_count = 0usize;
        let mut cache_hits_used = 0usize;
        let mut fresh_insertions_used = 0usize;
        let mut skipped_by_reason: HashMap<String, usize> = HashMap::new();

        // Acquire a mutable lock on the FontSystem for shaping/rasterization.
        let mut fs_guard = self.font_system.lock().unwrap();

        // Create Metrics for the buffer (font size belongs in Metrics in cosmic-text 0.19).
        // Provide an explicit line_height (1.2x font size) to satisfy Metrics::new(font_size, line_height)
        let metrics = cosmic_text::Metrics::new(
            self.atlas.font_size as f32,
            self.atlas.font_size as f32 * 1.2,
        );

        // Create a new buffer using the FontSystem & Metrics.
        let mut buf = Buffer::new(&mut *fs_guard, metrics);

        // Build attributes: prefer resolved_family if present, else fall back to default.
        // Use Family::Name to pass an owned name into Attrs.
        let mut attrs = cosmic_text::Attrs::new();
        if let Some(ref fam) = self.resolved_family {
            attrs = attrs.family(cosmic_text::Family::Name(fam.as_str()));
        }

        // Apply text using the real cosmic-text 0.19 API: provide &Attrs, a Shaping strategy,
        // and an optional alignment. Use Advanced shaping (full fallback + shaping).
        buf.set_text(text, &attrs, cosmic_text::Shaping::Advanced, None);

        // Borrow the buffer together with the FontSystem to run layout/shape helpers.
        // This mirrors the intended 0.19 flow: use BorrowedWithFontSystem to obtain layout runs.
        let mut jobs: Vec<(cosmic_text::CacheKey, u64, i32, i32, f32, [f32; 4])> = Vec::new();
        {
            let mut borrowed = buf.borrow_with(&mut *fs_guard);

            // Iterate layout runs produced by the borrowed buffer (this triggers shaping).
            let runs: Vec<_> = borrowed.layout_runs().collect();
            layout_runs_count = runs.len();
            if should_log {
                info!("CosmicTextBackend: layout_runs={}", layout_runs_count);
            }

            // Phase 1: build a list of rasterization jobs while the buffer borrow is active.
            for run in runs.iter() {
                let glyphs = &run.glyphs;
                layout_glyphs += glyphs.len();
                for g in glyphs.iter() {
                    // Compute physical glyph coordinates in absolute pixel space.
                    // The LayoutGlyph::physical(...) expects an (x,y) offset that is
                    // added to the glyph-local coordinates. We must include the caller-
                    // supplied `y` origin (the block/text origin) in the Y offset so
                    // the resulting physical.y is in the same pixel space as the clip
                    // rectangle passed to this function. Previously we passed only
                    // run.line_y which produced coordinates relative to the buffer,
                    // causing the clip test to incorrectly reject glyphs.
                    let physical = g.physical((x, y + run.line_y), 1.0);
                    let gid = g.glyph_id;
                    // integer pixel coords
                    let gx_i = physical.x;
                    let gy_i = physical.y;

                    // stable backend u64 key for atlas lookup
                    let key_u64 = Self::glyph_cache_key(gid.into(), self.atlas.font_size, gy_i);

                    // First check if the glyph is already present in the atlas (by key).
                    let existing = {
                        let map = self.atlas.glyph_id_map.lock().unwrap();
                        map.get(&key_u64).cloned()
                    };

                    // Resolve glyph color (respect possible per-glyph override)
                    let glyph_color = g.color_opt.map_or(color, cosmic_color_to_rgba);

                    if let Some(existing_ginfo) = existing {
                        // Use existing atlas entry to produce placed glyph immediately.
                        if existing_ginfo.width == 0 || existing_ginfo.height == 0 {
                            // advance-only glyph; still count as layout glyph
                            if should_log {
                                debug!(
                                    "CosmicTextBackend: skip glyph reason=advance_only text=\"{}\" glyph_id={} range={}..{} phys=({}, {}) key={} width={} height={}",
                                    text,
                                    gid,
                                    g.start,
                                    g.end,
                                    gx_i,
                                    gy_i,
                                    key_u64,
                                    existing_ginfo.width,
                                    existing_ginfo.height
                                );
                            }
                            *skipped_by_reason.entry("advance_only".to_string()).or_insert(0) += 1;
                            continue;
                        }
                        let x0_px = gx_i as f32 + existing_ginfo.xoffset as f32;
                        let y0_px = gy_i as f32 + existing_ginfo.yoffset as f32;
                        let x1_px = x0_px + existing_ginfo.width as f32;
                        let y1_px = y0_px + existing_ginfo.height as f32;

                        if x1_px <= clip_x_e
                            || x0_px >= (clip_x_e + clip_w_e)
                            || y1_px <= clip_y_e
                            || y0_px >= (clip_y_e + clip_h_e)
                        {
                            if should_log {
                                debug!(
                                    "CosmicTextBackend: skip glyph reason=clip_reject text=\"{}\" glyph_id={} range={}..{} phys=({}, {}) rect=({:.1},{:.1})-({:.1},{:.1}) clip=({:.1},{:.1})-({:.1},{:.1}) key={:?}",
                                    text,
                                    gid,
                                    g.start,
                                    g.end,
                                    gx_i,
                                    gy_i,
                                    x0_px,
                                    y0_px,
                                    x1_px,
                                    y1_px,
                                    clip_x_e,
                                    clip_y_e,
                                    clip_x_e + clip_w_e,
                                    clip_y_e + clip_h_e,
                                    key_u64
                                );
                            }
                            *skipped_by_reason.entry("clip_reject".to_string()).or_insert(0) += 1;
                            continue;
                        }

                        out.push(PlacedGlyph {
                            x0_px,
                            y0_px,
                            x1_px,
                            y1_px,
                            u0: existing_ginfo.u0,
                            v0: existing_ginfo.v0,
                            u1: existing_ginfo.u1,
                            v1: existing_ginfo.v1,
                            color: glyph_color,
                        });

                        rasterized_count += 1;
                        produced_placed += 1;
                        cache_hits_used += 1;
                        if should_log {
                            info!(
                                "CosmicTextBackend: cache_hit placed text=\"{}\" glyph_id={} range={}..{} phys=({}, {}) key={} uv=({:.4},{:.4})-({:.4},{:.4})",
                                text,
                                gid,
                                g.start,
                                g.end,
                                gx_i,
                                gy_i,
                                key_u64,
                                existing_ginfo.u0,
                                existing_ginfo.v0,
                                existing_ginfo.u1,
                                existing_ginfo.v1
                            );
                        }
                        continue;
                    }

                    // Missing in atlas -> enqueue a rasterization/upload job
                    // Capture necessary owned data to perform rasterization/upload later
                    if should_log {
                        debug!(
                            "CosmicTextBackend: enqueue_raster text=\"{}\" glyph_id={} range={}..{} phys=({}, {}) key={:?}",
                            text, gid, g.start, g.end, gx_i, gy_i, key_u64
                        );
                    }
                    jobs.push((physical.cache_key, key_u64, gx_i, gy_i, g.w, glyph_color));
                }
            }
            // `borrowed` goes out of scope here, ending the mutable borrow of FontSystem.
        }

        // Phase 2: perform rasterization/upload using the FontSystem again.
        // At this point the earlier buffer borrow has ended so we can mutably borrow fs_guard again.
        let mut swash = self.swash_cache.lock().unwrap();
        for (cache_key, key_u64, gx_i, gy_i, advance, glyph_color) in jobs.into_iter() {
            // Request raster image from SwashCache (may be cached).
            match swash.get_image(&mut *fs_guard, cache_key) {
                Some(img) => {
                    swash_images_obtained += 1;
                    atlas_insert_attempts += 1;

                    // Inspect swash image to determine format and bytes-per-pixel.
                    // We log a concise diagnostic payload to help confirm the image type.
                    let data_len = img.data.len();
                    let img_w = img.placement.width;
                    let img_h = img.placement.height;
                    let computed_bpp =
                        if img_w > 0 && img_h > 0 && (data_len as u32) == img_w * img_h {
                            1u32
                        } else if img_w > 0 && img_h > 0 && (data_len as u32) == img_w * img_h * 4 {
                            4u32
                        } else {
                            // Unknown packing: fall back to 1 and log for investigation.
                            1u32
                        };

                    if should_log {
                        info!(
                            "CosmicTextBackend: swash_image: cache_key={:?} key_u64={} placement=({}x{}) data_len={} inferred_bpp={}",
                            cache_key, key_u64, img_w, img_h, data_len, computed_bpp
                        );
                        // Log first few bytes so we can tell whether data looks like mask vs RGBA.
                        let sample_bytes = &img.data[..std::cmp::min(16, img.data.len())];
                        debug!("CosmicTextBackend: swash_image sample_bytes={:?}", sample_bytes);
                    }

                    // Attempt to insert/upload the glyph bitmap into the atlas.
                    match self.atlas.insert_glyph_from_bitmap(
                        _queue,
                        key_u64,
                        &img.data,
                        img.placement.width,
                        img.placement.height,
                        advance,
                        img.placement.left,
                        -img.placement.top,
                        computed_bpp,
                    ) {
                        Ok((u0, v0, u1, v1)) => {
                            atlas_insert_succeeded += 1;
                            fresh_insertions_used += 1;

                            let x0_px = gx_i as f32 + img.placement.left as f32;
                            let y0_px = gy_i as f32 - img.placement.top as f32;
                            let x1_px = x0_px + img.placement.width as f32;
                            let y1_px = y0_px + img.placement.height as f32;

                            // Clip-test (use tolerant clip)
                            if x1_px <= clip_x_e
                                || x0_px >= (clip_x_e + clip_w_e)
                                || y1_px <= clip_y_e
                                || y0_px >= (clip_y_e + clip_h_e)
                            {
                                if should_log {
                                    debug!(
                                        "CosmicTextBackend: skip after_insert reason=clip_reject text=\"{}\" cache_key={:?} key_u64={} phys=({}, {}) rect=({:.1},{:.1})-({:.1},{:.1}) clip=({:.1},{:.1})-({:.1},{:.1})",
                                        text,
                                        cache_key,
                                        key_u64,
                                        gx_i,
                                        gy_i,
                                        x0_px,
                                        y0_px,
                                        x1_px,
                                        y1_px,
                                        clip_x_e,
                                        clip_y_e,
                                        clip_x_e + clip_w_e,
                                        clip_y_e + clip_h_e
                                    );
                                }
                                *skipped_by_reason
                                    .entry("clip_reject_after_insert".to_string())
                                    .or_insert(0) += 1;
                                continue;
                            }

                            out.push(PlacedGlyph {
                                x0_px,
                                y0_px,
                                x1_px,
                                y1_px,
                                u0,
                                v0,
                                u1,
                                v1,
                                color: glyph_color,
                            });

                            if should_log {
                                debug!(
                                    "CosmicTextBackend: inserted_and_placed text=\"{}\" cache_key={:?} key_u64={} phys=({}, {}) uv=({:.4},{:.4})-({:.4},{:.4})",
                                    text, cache_key, key_u64, gx_i, gy_i, u0, v0, u1, v1
                                );
                            }

                            rasterized_count += 1;
                            produced_placed += 1;
                        }
                        Err(e) => {
                            // Insertion failed; count as missing and continue.
                            debug!(
                                "CosmicTextBackend: atlas insertion failed for glyph key={} err={:?}",
                                key_u64, e
                            );
                            *skipped_by_reason
                                .entry("atlas_insert_failed".to_string())
                                .or_insert(0) += 1;
                            missing_glyphs += 1;
                        }
                    }
                }
                None => {
                    if should_log {
                        info!(
                            "CosmicTextBackend: no_swash_image text=\"{}\" cache_key={:?} key_u64={} phys=({}, {})",
                            text, cache_key, key_u64, gx_i, gy_i
                        );
                    }
                    *skipped_by_reason.entry("no_swash_image".to_string()).or_insert(0) += 1;
                    missing_glyphs += 1;
                }
            }
        }

        // Summary diagnostics (gated)
        if should_log {
            info!(
                "CosmicTextBackend: layout_runs={} layout_glyphs={}",
                layout_runs_count, layout_glyphs
            );
            info!(
                "CosmicTextBackend: swash_images_obtained={} atlas_insert_attempts={} atlas_insert_succeeded={}",
                swash_images_obtained, atlas_insert_attempts, atlas_insert_succeeded
            );
            info!("CosmicTextBackend: produced_placed_glyphs={}", produced_placed);
        }

        debug!(
            "CosmicTextBackend: layout glyphs={} rasterized_glyphs={} missing_glyphs={}",
            layout_glyphs, rasterized_count, missing_glyphs
        );

        Ok(out)
    }

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        Some(&self.atlas.bind_group)
    }
}

/*
Glyphon integration has been moved to `crates/zaroxi-engine-render/src/renderer/text/mod.rs`
which implements the new `TextRenderer` trait and provides the default Glyphon-backed
text renderer. The legacy `CosmicTextBackend` remains available behind the
`legacy_cosmic` Cargo feature and is not used by default.

The legacy `GlyphonTextBackend` stub that previously lived here has been intentionally
removed to keep glyphon-specific code inside `renderer::text` and to avoid scattering
glyphon APIs across unrelated modules.
*/
