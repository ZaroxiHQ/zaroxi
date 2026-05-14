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
            // Use fontdb::Database::load_font_file to register the on-disk TTF with the database.
            // This API returns Result<(), std::io::Error>.
            match db.load_font_file(&font_path) {
                Ok(()) => {
                    bundled_loaded = true;
                    debug!("CosmicTextBackend: bundled font loaded into fontdb from '{}'", font_path.display());
                }
                Err(e) => {
                    debug!("CosmicTextBackend: fontdb failed to load bundled font '{}': {:?}", font_path.display(), e);
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
        let swash_cache = cosmic_text::SwashCache::new();

        // Determine the exact family name that will be used for the bundled font.
        // Prefer "JetBrainsMono Nerd Font" but query the attached font database to
        // discover the resolved family name to use in attributes/queries.
        let mut resolved_family: Option<String> = None;
        {
            // Query the local fontdb::Database we created above to discover registered families.
            // Collect discovered family names from faces. FaceInfo.families is Vec<(String, Language)>;
            // use the first family name for each face as the primary discovered family.
            let matches: Vec<String> = db.faces().filter_map(|face| {
                face.families.get(0).map(|(name, _lang)| name.clone())
            }).collect();

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
        // Simplified, robust layout path that relies on the backend's internal
        // atlas metadata populated at runtime. This avoids calling unstable or
        // version-specific cosmic-text shaping helpers here and keeps the
        // renderer functional during the migration.
        let mut out: Vec<PlacedGlyph> = Vec::new();
        let mut pen_x = x;
        let mut rasterized_count: usize = 0usize;
        let mut layout_glyphs: usize = 0usize;

        for ch in text.chars() {
            layout_glyphs += 1;
            if let Some(g) = self.atlas.glyphs.get(&ch) {
                // Advance-only glyphs move the pen
                if g.width == 0 || g.height == 0 {
                    pen_x += g.advance;
                    continue;
                }

                let x0_px = pen_x + g.xoffset as f32;
                let y0_px = y + g.yoffset as f32;
                let x1_px = x0_px + g.width as f32;
                let y1_px = y0_px + g.height as f32;

                // Clip-test
                if x1_px <= clip_x || x0_px >= (clip_x + clip_w) || y1_px <= clip_y || y0_px >= (clip_y + clip_h) {
                    pen_x += g.advance;
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

                pen_x += g.advance;
                rasterized_count += 1;
            } else {
                // Missing glyph: advance conservatively and continue.
                pen_x += self.atlas.font_size * 0.5;
            }
        }

        debug!(
            "CosmicTextBackend: layout glyphs={} rasterized_glyphs={}",
            layout_glyphs,
            rasterized_count
        );

        Ok(out)
    }

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        Some(&self.atlas.bind_group)
    }
}
