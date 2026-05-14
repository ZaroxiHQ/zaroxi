use crate::error::RenderError;
use crate::renderer::text::{FontAtlas, PlacedGlyph};
use crate::renderer::text;
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

/// DefaultTextBackend
///
/// Incremental migration backend that adapts the existing font atlas approach
/// into the new TextBackend boundary. It performs basic cluster-aware layout
/// (using grapheme clusters) and delegates glyph metrics/uv lookup to the
/// supplied FontAtlas. This backend is intentionally conservative: it keeps
/// the existing raster/atlas behavior while providing a clean integration
/// point for future replacers (e.g., CosmicText + swash).
pub struct DefaultTextBackend {}

impl DefaultTextBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl TextBackend for DefaultTextBackend {
    fn layout_text_clipped(
        &self,
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
        // Use a lightweight grapheme-cluster-aware iterator to avoid splitting
        // visually atomic clusters. This provides a small but meaningful step
        // toward real shaping without pulling in a heavy shaping engine here.
        //
        // The implementation reuses the atlas glyph metadata (u0..v1, xoffset/yoffset,
        // advance) to compute glyph pixel rects and performs the clip test.
        use unicode_segmentation::UnicodeSegmentation;

        let mut out: Vec<PlacedGlyph> = Vec::new();

        for g in text.graphemes(true) {
            // For each grapheme cluster, iterate its chars to emit glyphs in sequence.
            for ch in g.chars() {
                if let Some(glyph) = atlas.glyphs.get(&ch) {
                    // Advance-only glyphs: still advance pen but do not emit geometry.
                    if glyph.width == 0 || glyph.height == 0 {
                        x += glyph.advance;
                        continue;
                    }

                    let x0_px = x + glyph.xoffset as f32;
                    let y0_px = y + glyph.yoffset as f32;
                    let x1_px = x0_px + glyph.width as f32;
                    let y1_px = y0_px + glyph.height as f32;

                    // Clip-test: skip glyphs fully outside the clip rect (still advance)
                    if !(x1_px <= clip_x || x0_px >= (clip_x + clip_w) || y1_px <= clip_y || y0_px >= (clip_y + clip_h)) {
                        out.push(PlacedGlyph {
                            x0_px,
                            y0_px,
                            x1_px,
                            y1_px,
                            u0: glyph.u0,
                            v0: glyph.v0,
                            u1: glyph.u1,
                            v1: glyph.v1,
                            color,
                        });
                    }

                    x += glyph.advance;
                } else {
                    // Unknown glyph: attempt to skip gracefully by advancing a fallback amount.
                    // Use a small nominal advance to avoid collapse; real fallback will be
                    // implemented by a future backend (font fallback).
                    x += atlas.font_size * 0.5;
                }
            }
        }

        if cfg!(debug_assertions) {
            debug!("DefaultTextBackend: laid out {} placed glyphs for text '{}'", out.len(), text);
        }

        Ok(out)
    }
}
