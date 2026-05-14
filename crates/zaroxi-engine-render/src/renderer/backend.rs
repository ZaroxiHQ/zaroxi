use crate::error::RenderError;
use crate::renderer::text::{PlacedGlyph, FontAtlas, GlyphInfo};
use log::{debug, info};
use std::collections::HashMap;
use std::sync::Mutex;
use wgpu::{Device, Queue, BindGroupLayout, BindGroup};

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
pub struct CosmicTextBackend {
    // cosmic-text's FontSystem is the shaping/layout/fallback engine.
    font_system: cosmic_text::FontSystem,
    // swash-backed raster cache from cosmic-text (used to rasterize glyph bitmaps)
    swash_cache: cosmic_text::SwashCache,
    // GPU atlas and associated metadata (managed by the backend)
    atlas: FontAtlas,
    // Mapping from a stable cache key -> glyph placement/meta in the atlas.
    // Key encodes glyph identity + raster-size related inputs.
    glyph_cache_keys: Mutex<HashMap<u64, GlyphInfo>>,
}

impl CosmicTextBackend {
    /// Create a new CosmicTextBackend and create an empty GPU atlas using
    /// the provided bind group layout so the backend can upload glyphs on-demand.
    pub fn new(device: &Device, queue: &Queue, layout: &BindGroupLayout, font_size: f32) -> Result<Self, RenderError> {
        // Initialize FontSystem.
        let mut fs = cosmic_text::FontSystem::new();

        // Load the bundled font (shared workspace asset).
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path = std::path::PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let font_bytes = std::fs::read(&font_path).map_err(|e| {
            RenderError::Other(format!("cosmic-text: failed to read font '{}': {:?}", font_path.display(), e))
        })?;
        let _ = fs.add_font_bytes(font_bytes);

        // Initialize swash cache (cosmic-text wrapper that exposes swash rasterization).
        let swash_cache = cosmic_text::SwashCache::new();

        // Create an empty GPU atlas that the backend will populate on demand.
        let atlas = FontAtlas::new_empty(device, queue, layout, font_size)?;

        Ok(Self {
            font_system: fs,
            swash_cache,
            atlas,
            glyph_cache_keys: Mutex::new(HashMap::new()),
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
        screen_w: f32,
        screen_h: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Result<Vec<PlacedGlyph>, RenderError> {
        // Create a cosmic-text Buffer to perform shaping + layout.
        let mut buffer = cosmic_text::Buffer::new();
        buffer.set_size(self.atlas.font_size as f32, 0.0);
        buffer.set_text(text);

        buffer.shape(&self.font_system);

        let glyphs = buffer.glyphs();
        let mut out: Vec<PlacedGlyph> = Vec::new();

        for g in glyphs.iter() {
            // Compute pixel-space positions.
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

            // Determine a glyph identity to rasterize/cached against.
            // Prefer cluster_char when available (stable scalar), otherwise fall back
            // to cosmic-text-provided glyph id if present.
            let glyph_identity = match g.cluster_char {
                Some(ch) => ch as u32,
                None => {
                    // best-effort: cosmic-text glyph id (if present)
                    g.glyph_id.unwrap_or(0)
                }
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
                // Rasterize glyph via swash_cache at target font size and subpixel alignment.
                // The swash cache produces a single-channel coverage bitmap we can upload.
                // The exact swash API used here is the SwashCache convenience provided by cosmic-text.
                let font_px = self.atlas.font_size as f32;
                let raster = self.swash_cache.rasterize_glyph(glyph_identity, font_px, subpixel_y)?;

                // raster: (bytes, w, h, xoffset, yoffset, advance)
                let (bmp, w, h, xmin, ymin, advance) = raster;

                // Insert into GPU atlas (pack + upload)
                let gi = GlyphInfo {
                    u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0,
                    width: w, height: h,
                    advance,
                    xoffset: xmin, yoffset: ymin,
                };

                // atlas.insert_glyph_from_bitmap will write into the texture and update UVs.
                let (u0_n, v0_n, u1_n, v1_n) = self.atlas.insert_glyph_from_bitmap(queue, key, &bmp, w, h, gi.advance, gi.xoffset, gi.yoffset)?;

                // Store metadata in glyph_cache_keys
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
                (stored.u0, stored.v0, stored.u1, stored.v1)
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

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        Some(&self.atlas.bind_group)
    }
}
