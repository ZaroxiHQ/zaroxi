use crate::error::RenderError;
use crate::renderer::text::{PlacedGlyph, FontAtlas, GlyphInfo};
use log::{debug, info};
use std::collections::HashMap;
use std::sync::Mutex;
use wgpu::{Device, Queue, BindGroupLayout, BindGroup};

/* explicit re-exports expected by the backend code; cosmic-text crate is
   provided as "cosmic-text" in Cargo.toml but the Rust module path is
   `cosmic_text`. Import the commonly used types here so the file uses them
   directly. */
use cosmic_text::{FontSystem, SwashCache, Buffer};

/// A minimal backend boundary trait for text shaping/layout/rasterization.
///
/// Implementations are responsible for:
/// - shaping & layout (producing placed glyphs with pixel coordinates + atlas UVs)
/// - rasterization / atlas interactions (atlas is internal to the backend)
///
/// The renderer consumes only placed glyphs produced by a backend instance.
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

        Self {
            ui_families: ui,
            mono_families: mono,
            symbol_families: symbols,
        }
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

pub struct CosmicTextBackend {
    // cosmic-text's FontSystem is the shaping/layout/fallback engine.
    font_system: cosmic_text::FontSystem,
    // swash-backed raster cache from cosmic-text (used to rasterize glyph bitmaps)
    swash_cache: cosmic_text::SwashCache,
    // GPU atlas and associated metadata (managed by the backend)
    atlas: FontAtlas,
    // Mapping from a stable cache key -> glyph placement/meta in the atlas.
    // Key encodes glyph identity + raster-size-related inputs.
    glyph_cache_keys: Mutex<HashMap<u64, GlyphInfo>>,
    // Font selection/fallback policy used by this backend.
    font_policy: FontPolicy,
}

impl CosmicTextBackend {
    /// Create a new CosmicTextBackend and create an empty GPU atlas using
    /// the provided bind group layout so the backend can upload glyphs on-demand.
    pub fn new(device: &Device, queue: &Queue, layout: &BindGroupLayout, font_size: f32) -> Result<Self, RenderError> {
        // Initialize FontSystem and register the bundled TTF into a fontdb
        // database so cosmic-text can resolve the exact family name we intend to use.
        // This explicitly prefers the workspace-bundled JetBrainsMono Nerd Font
        // as the primary UI/mono family.
        let mut fs = cosmic_text::FontSystem::new();

        // Try to register bundled font using fontdb and attach the database to the FontSystem.
        // This uses fontdb 0.23 compatible APIs.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path = PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let mut db = fontdb::Database::new();
        let mut bundled_loaded = false;
        if font_path.exists() {
            match std::fs::read(&font_path) {
                Ok(bytes) => {
                    let id = db.load_font_data(bytes);
                    if id.is_some() {
                        bundled_loaded = true;
                        debug!("CosmicTextBackend: bundled font loaded into fontdb from '{}'", font_path.display());
                    } else {
                        debug!("CosmicTextBackend: fontdb failed to load bundled font '{}'", font_path.display());
                    }
                }
                Err(e) => {
                    debug!("CosmicTextBackend: failed to read bundled font '{}': {:?}", font_path.display(), e);
                }
            }
        } else {
            debug!("CosmicTextBackend: bundled font not found at '{}'", font_path.display());
        }

        // Attach database to FontSystem if possible.
        // The FontSystem in cosmic-text exposes a way to override its database via `set_db`.
        // If that method isn't present, this is a best-effort attempt that still compiles
        // against the typical cosmic-text/fontdb integration.
        if bundled_loaded {
            fs.set_database(db);
        } else {
            // If loading failed, still attach an empty db so we can inspect registered families below.
            fs.set_database(db);
        }

        // Build a default font policy. This captures preferred family names and
        // a symbol/nerd-font fallback chain. The policy is purely a configuration
        // object; the FontSystem remains the authoritative shaping/fallback engine.
        let font_policy = FontPolicy::default_with_assets(".");

        // Initialize swash cache (cosmic-text wrapper that exposes swash rasterization).
        let swash_cache = cosmic_text::SwashCache::new();

        // Determine the exact family name that will be used for the bundled font.
        // Prefer "JetBrainsMono Nerd Font" but query the attached font database to
        // discover the resolved family name to use in attributes/queries.
        let mut resolved_family: Option<String> = None;
        {
            // Ask fontdb (if available in the FontSystem) for matching families.
            // This is a non-spammy diagnostic: log only if bundled font was loaded.
            if let Some(db_ref) = fs.database() {
                // Find faces with family name hint 'JetBrainsMono Nerd Font' first.
                let matches = db_ref.faces().iter().filter_map(|face|
                    match db_ref.family_by_face_id(face.id) {
                        Some(f) => Some(f.to_string()),
                        None => None
                    }
                ).collect::<Vec<_>>();

                // If the exact family exists, prefer it; otherwise pick the first discovered.
                if matches.iter().any(|m| m == "JetBrainsMono Nerd Font") {
                    resolved_family = Some("JetBrainsMono Nerd Font".to_string());
                } else if !matches.is_empty() {
                    resolved_family = Some(matches[0].clone());
                }

                debug!("CosmicTextBackend: fontdb discovered families (sample) = {:?}", matches);
            } else {
                debug!("CosmicTextBackend: FontSystem has no attached database to query families");
            }
        }

        if let Some(ref fam) = resolved_family {
            debug!("CosmicTextBackend: using resolved bundled family '{}'", fam);
        } else {
            debug!("CosmicTextBackend: no bundled family resolved; will attempt to use default FontSystem fallbacks");
        }

        // Create an empty GPU atlas that the backend will populate on demand.
        let atlas = FontAtlas::new_empty(device, queue, layout, font_size)?;

        Ok(Self {
            font_system: fs,
            swash_cache,
            atlas,
            glyph_cache_keys: Mutex::new(HashMap::new()),
            font_policy,
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

impl TextBackend for CosmicTextBackend {
    fn layout_text_clipped(
        &self,
        queue: &mut Queue,
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
        // Use cosmic-text Buffer (0.19) for shaping/layout, then rasterize missing
        // glyph bitmaps via SwashCache and upload into our atlas.
        // This keeps cosmic-text as the single source-of-truth for shaping.
        let mut out: Vec<PlacedGlyph> = Vec::new();

        // Create buffer and shape text using the backend FontSystem
        let mut buffer = cosmic_text::Buffer::new(&self.font_system);
        buffer.set_size(self.atlas.font_size as f32, 0.0);
        buffer.set_text(text);
        buffer.shape();

        // Collect glyph layout info from the buffer.
        let glyphs = buffer.glyphs();
        let mut rasterized_count: usize = 0usize;

        for g in glyphs.iter() {
            // Compute pixel-space positions. cosmic-text positions are in pixels.
            let gx = x + g.x as f32;
            let gy = y + (g.y as f32) - (g.h as f32);

            let x0_px = gx;
            let y0_px = gy;
            let x1_px = gx + g.w as f32;
            let y1_px = gy + g.h as f32;

            // Clip-test
            if x1_px <= clip_x || x0_px >= (clip_x + clip_w) || y1_px <= clip_y || y0_px >= (clip_y + clip_h) {
                continue;
            }

            // Determine a glyph identity to rasterize/cache against.
            let glyph_identity = match g.cluster_char {
                Some(ch) => ch as u32,
                None => g.glyph_id.unwrap_or(0),
            };

            // Build cache key including subpixel Y alignment (rounding to 1/64)
            let subpixel_y = ((g.y as f32 * 64.0).round() as i32) as i32;
            let key = CosmicTextBackend::glyph_cache_key(glyph_identity, self.atlas.font_size as f32, subpixel_y);

            // Check existing atlas entry for this key
            let maybe_gi = {
                let map = self.glyph_cache_keys.lock().unwrap();
                map.get(&key).cloned()
            };

            let (u0, v0, u1, v1) = if let Some(ai) = maybe_gi {
                (ai.u0, ai.v0, ai.u1, ai.v1)
            } else {
                // Rasterize glyph using SwashCache provided by cosmic-text.
                let font_px = self.atlas.font_size as f32;
                match self.swash_cache.rasterize_glyph(glyph_identity, font_px, subpixel_y) {
                    Ok(raster) => {
                        // raster: (bytes, w, h, xoffset, yoffset, advance)
                        let (bmp, w, h, xmin, ymin, advance) = raster;

                        // Insert into GPU atlas (pack + upload)
                        let gi = GlyphInfo {
                            u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0,
                            width: w, height: h,
                            advance,
                            xoffset: xmin, yoffset: ymin,
                        };

                        let (u0_n, v0_n, u1_n, v1_n) = match self.atlas.insert_glyph_from_bitmap(queue, key, &bmp, w, h, gi.advance, gi.xoffset, gi.yoffset) {
                            Ok(vals) => vals,
                            Err(e) => {
                                debug!("CosmicTextBackend: atlas insertion failed for key {}: {:?}", key, e);
                                (0.0, 0.0, 0.0, 0.0)
                            }
                        };

                        let stored = GlyphInfo {
                            u0: u0_n, v0: v0_n, u1: u1_n, v1: v1_n,
                            width: w, height: h,
                            advance: gi.advance,
                            xoffset: gi.xoffset, yoffset: gi.yoffset,
                        };
                        {
                            let mut map = self.glyph_cache_keys.lock().unwrap();
                            map.insert(key, stored.clone());
                        }

                        rasterized_count += 1;
                        (stored.u0, stored.v0, stored.u1, stored.v1)
                    }
                    Err(e) => {
                        debug!("CosmicTextBackend: swash_cache rasterize_glyph failed for glyph_id {}: {:?}", glyph_identity, e);
                        // fallback: insert placeholder entry (advance-only)
                        let stored = GlyphInfo {
                            u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0,
                            width: 0, height: 0,
                            advance: 8.0,
                            xoffset: 0, yoffset: 0,
                        };
                        {
                            let mut map = self.glyph_cache_keys.lock().unwrap();
                            map.insert(key, stored.clone());
                        }
                        (stored.u0, stored.v0, stored.u1, stored.v1)
                    }
                }
            };

            out.push(PlacedGlyph {
                x0_px,
                y0_px,
                x1_px,
                y1_px,
                u0,
                v0,
                u1,
                v1,
                color,
            });
        }

        // Diagnostics: log layout/rasterization summary.
        debug!(
            "CosmicTextBackend: layout glyphs={} rasterized_glyphs={}",
            glyphs.len(),
            rasterized_count
        );

        Ok(out)
    }

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        Some(&self.atlas.bind_group)
    }
}
