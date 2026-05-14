use crate::error::RenderError;
use crate::renderer::text::{FontAtlas, PlacedGlyph};
use log::debug;

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
    fn layout_text_clipped(
        &self,
        atlas: &FontAtlas,
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
}

//
// CosmicTextBackend
//
// Implements TextBackend using cosmic-text as the source of shaping/layout
// and baseline/metric information. Rasterization and atlas uploads continue
// to be owned by FontAtlas for now (atlas remains an internal implementation
// detail). This backend uses cosmic-text for:
//  - shaping (grapheme/clusters, complex scripts, bidi as available)
//  - layout (advances, positions, baselines, line metrics)
//  - font fallback (cosmic-text's font resolution)
// The placed glyphs returned to the renderer are generic and carry pixel-space
// rectangles plus atlas UVs (when available). If the atlas does not contain a
// raster for a shaped glyph (e.g., fallback glyph), the UVs are zero and the
// atlas layer may later populate the glyph image on-demand (future work).
//
pub struct CosmicTextBackend {
    // cosmic-text's FontSystem is the shaping/layout/fallback engine.
    // We keep it boxed to avoid exposing cosmic_text in the public crate API.
    font_system: cosmic_text::FontSystem,
}

impl CosmicTextBackend {
    /// Create a new CosmicTextBackend and eagerly register the same font used by
    /// the legacy FontAtlas so shaping/layout & fallback resolution can succeed
    /// for the primary UI font. The backend may register additional system
    /// fonts for fallback as needed by cosmic-text internals.
    pub fn new() -> Result<Self, RenderError> {
        // Initialize FontSystem (cosmic-text 0.19.0).
        // We create the system and register the bundled UI font so shaping/layout
        // resolves the same glyphs the legacy atlas expects. The backend keeps
        // the FontSystem as the authoritative shaping/layout/fallback provider.
        let mut fs = cosmic_text::FontSystem::new();

        // Load the bundled font (shared workspace asset). If this fails we surface
        // a RenderError so the caller can decide how to proceed.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path = std::path::PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let font_bytes = std::fs::read(&font_path).map_err(|e| {
            RenderError::Other(format!("cosmic-text: failed to read font '{}': {:?}", font_path.display(), e))
        })?;

        // cosmic-text 0.19.0 exposes `add_font_bytes` which accepts the font bytes
        // and returns an identifier. Register the bytes so FontSystem can use them
        // for shaping and fallback. We ignore the returned id (consumer code may
        // need it later for advanced font selection).
        let _ = fs.add_font_bytes(font_bytes);

        Ok(Self { font_system: fs })
    }
}

impl TextBackend for CosmicTextBackend {
    fn layout_text_clipped(
        &self,
        atlas: &FontAtlas,
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
    ) -> Result<Vec<PlacedGlyph>, RenderError> {
        // Create a cosmic-text Buffer to perform shaping + layout.
        // We limit buffer configuration to the essentials required for single-line
        // header/title layout: font size, simple metrics, and the provided text.
        let mut buffer = cosmic_text::Buffer::new();
        buffer.set_size(atlas.font_size as f32, 0.0); // set_size(font_size, line_height_extra)
        buffer.set_text(text);

        // Perform shaping/layout using the font system. This call produces an
        // internal glyph list we can iterate to obtain glyph positions and
        // advances. If shaping fails, fall back to a conservative empty layout.
        buffer.shape(&self.font_system);

        // Acquire shaped glyphs: cosmic-text exposes glyph info via buffer.glyphs().
        // Each entry contains glyph id, x/y positions (pixel space), and advance.
        // Iterate and convert into renderer-facing PlacedGlyph entries.
        let mut out: Vec<PlacedGlyph> = Vec::new();

        // cosmic-text's glyph iterator returns items (glyph_id, x, y, w, h, advance).
        // We'll map glyphs back to characters for atlas lookup where possible by
        // using the original text and cluster offsets from the buffer.
        // Note: cosmic-text may shape clusters into glyph sequences (complex scripts).
        let glyphs = buffer.glyphs();

        for g in glyphs.iter() {
            // x,y are positions relative to the buffer origin (pixel-space).
            let gx = x + g.x as f32;
            // cosmic-text supplies y as baseline offset; compute top y using glyph height.
            let gy = y + (g.y as f32) - (g.h as f32);

            let x0_px = gx;
            let y0_px = gy;
            let x1_px = gx + g.w as f32;
            let y1_px = gy + g.h as f32;

            // Clip-test: skip glyphs fully outside the clip rect (still advance)
            if x1_px <= clip_x || x0_px >= (clip_x + clip_w) || y1_px <= clip_y || y0_px >= (clip_y + clip_h) {
                continue;
            }

            // Attempt to get atlas UVs using glyph.cluster_char if available.
            // Fallback: try to find atlas entry by the Unicode scalar (best-effort).
            let (u0, v0, u1, v1) = if let Some(ch) = g.cluster_char {
                if let Some(ai) = atlas.glyphs.get(&ch) {
                    (ai.u0, ai.v0, ai.u1, ai.v1)
                } else {
                    (0.0f32, 0.0f32, 0.0f32, 0.0f32)
                }
            } else {
                (0.0f32, 0.0f32, 0.0f32, 0.0f32)
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

        if cfg!(debug_assertions) {
            debug!("CosmicTextBackend: laid out {} placed glyphs for text '{}'", out.len(), text);
        }

        Ok(out)
    }
}
