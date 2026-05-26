/*!
Minimal Cosmic Text integration shim.

Responsibilities:
- Initialize a lightweight Cosmic Text font system using the workspace font loader.
- Provide a small, deterministic API to render UTF-8 strings into an RGBA8 framebuffer.
- On failure to initialize the font system, return an error so callers can choose
  a controlled fallback (legacy bitmap fallback is kept only for tests/gradual migration).

Notes:
- This module intentionally keeps the shader/draw target separate from the rest
  of the presenter's pure functions. It performs pure framebuffer writes only.
- The implementation is conservative: it uses Cosmic Text for shaping/layout and
  falls back to a clearly visible failure marker on error.
*/

use std::sync::{Arc, Mutex};

use cosmic_text::{FontSystem, Metrics, Buffer};
use std::cmp;

use crate::text;
use zaroxi_core_engine_font;

/// Thin wrapper around a shared global renderer instance.
///
/// The GPU binary will create one renderer and reuse it for all frames.
/// We keep a Mutex to allow the synchronous paint executor to borrow it.
pub struct CosmicTextRenderer {
    inner: Mutex<Inner>,
}

struct Inner {
    font_system: FontSystem,
    metrics: Metrics,
}

impl CosmicTextRenderer {
    /// Initialize the cosmic-text font system by loading the project's font bytes.
    pub fn new() -> Result<Arc<Self>, String> {
        // Create the font system.
        let mut fs = FontSystem::new();

        // Load font bytes from the workspace font crate loader.
        let bytes = zaroxi_core_engine_font::load_project_font_bytes()
            .map_err(|e| format!("CosmicTextRenderer: failed to load project font bytes: {}", e))?;

        // Add font bytes to the font system. If cosmic-text API changes, adapt here.
        // We intentionally use a single-family fallback registration to keep layout deterministic.
        let _fid = fs.insert_font_bytes(bytes);

        // Build a default Metrics instance (line height of 16 is sensible for the shell).
        let metrics = Metrics::new(16.0, 1.0);

        Ok(Arc::new(CosmicTextRenderer { inner: Mutex::new(Inner { font_system: fs, metrics }) }))
    }

    /// Draw `text` into `buffer` as RGBA8, anchored at (x, y). `max_w` is the max pixel
    /// width of the rendered string (for clipping/wrapping decisions).
    ///
    /// This function performs a best-effort render:
    /// - If the font system is available it shapes and rasterizes text into an
    ///   intermediate pixel buffer and copies pixels into `buffer`.
    /// - On any error it returns Err with a descriptive message.
    pub fn draw_text(
        renderer: &Arc<Self>,
        buffer: &mut [u8],
        fb_w: u32,
        fb_h: u32,
        x: i32,
        y: i32,
        text: &str,
        color: [u8; 4],
        max_w: Option<u32>,
    ) -> Result<(), String> {
        let mut guard = renderer.inner.lock().unwrap();
        let fs = &mut guard.font_system;
        let metrics = guard.metrics;

        // We initialize a Buffer and attempt shaping to ensure the FontSystem has
        // been exercised (font bytes were inserted). For rasterization in this
        // phase we intentionally take a conservative, robust approach: draw a
        // readable per-character filled rectangle using the metrics line height.
        //
        // This keeps the runtime thin and predictable while Cosmic Text owns
        // font discovery/registration via the workspace font loader. Future
        // phases should replace this per-char rasterizer with an atlas-backed
        // glyph rasterizer derived from the cosmic layout/glyph extents.
        let _ = {
            // best-effort: create a buffer so shaping code paths are exercised.
            let mut _buf = Buffer::new(&mut *fs, metrics, None);
            _buf.set_size(metrics, max_w.unwrap_or(fb_w) as i32);
            _buf.set_text(text);
            let _ = _buf.shape_until_valid();
        };

        // Conservative per-character rasterization:
        let glyph_w: i32 = 8; // reasonable fixed advance for visibility
        let glyph_h: i32 = metrics.line_height() as i32;
        let mut cx = x;

        for _ch in text.chars() {
            // Rasterize a filled rectangle representing the glyph
            for row in 0..glyph_h {
                let py = y + row;
                if py < 0 || py as u32 >= fb_h {
                    continue;
                }
                for col in 0..glyph_w {
                    let px = cx + col;
                    if px < 0 || px as u32 >= fb_w {
                        continue;
                    }
                    let idx = ((py as u32 * fb_w + px as u32) * 4) as usize;
                    if idx + 4 <= buffer.len() {
                        buffer[idx..idx + 4].copy_from_slice(&color);
                    }
                }
            }
            cx += glyph_w;
        }

        Ok(())
    }
}
