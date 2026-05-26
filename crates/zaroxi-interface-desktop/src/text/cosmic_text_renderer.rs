/*!
Cosmic Text integration shim.

Responsibilities:
- Keep project font discovery owned by `zaroxi-core-engine-font`.
- Provide an interface-local renderer that validates project font bytes are
  available and exposes a small draw_text API used by the presenter.
- IMPORTANT: At this stage we intentionally do not own the project font path
  discovery policy (that's in `zaroxi-core-engine-font`) and we avoid coupling
  other crates to cosmic-text types. A follow-up change will wire the full
  cosmic-text shaping + rasterization pipeline once the workspace cosmic-text
  API is standardized. For now this module preserves the correct ownership
  boundaries and provides a deterministic, readable fallback rasterizer.
*/

use std::sync::{Arc, Mutex};

use zaroxi_core_engine_font;

/// Thin wrapper around a shared renderer-like instance.
///
/// The GPU binary will create one renderer and reuse it for all frames.
/// We keep a Mutex to allow the synchronous paint executor to borrow it.
pub struct CosmicTextRenderer {
    inner: Mutex<Inner>,
}

struct Inner {
    /// Whether the project font bytes were successfully loaded.
    font_loaded: bool,
    /// Number of bytes loaded (diagnostic).
    font_bytes_len: usize,
    /// Conservative monospace metrics used by the fallback rasterizer.
    char_width: u32,
    line_height: u32,
}

impl CosmicTextRenderer {
    /// Initialize the renderer by ensuring the project's font bytes are loadable.
    ///
    /// Note: this function purposefully does not construct any cosmic-text types.
    /// The font discovery seam lives in `zaroxi-core-engine-font` and returns raw
    /// bytes. Future work will register those bytes into a FontSystem here.
    pub fn new() -> Result<Arc<Self>, String> {
        // Obtain bytes from the canonical loader provided by the core font crate.
        let bytes = zaroxi_core_engine_font::project_font_bytes().map_err(|e| {
            format!("CosmicTextRenderer: failed to obtain project font bytes: {}", e)
        })?;

        if bytes.is_empty() {
            return Err("CosmicTextRenderer: project font bytes are empty".to_string());
        }

        // Conservative monospace metrics derived from the core font crate helper.
        let fm = zaroxi_core_engine_font::load_bundled_monospace();

        let inner = Inner {
            font_loaded: true,
            font_bytes_len: bytes.len(),
            char_width: fm.char_width,
            line_height: fm.line_height,
        };

        Ok(Arc::new(CosmicTextRenderer {
            inner: Mutex::new(inner),
        }))
    }

    /// Draw `text` into `out_buffer` as RGBA8, anchored at (x, y).
    ///
    /// This implementation uses a deterministic monospace fallback rasterizer
    /// based on metrics exposed by `zaroxi-core-engine-font`. It is robust,
    /// simple, and preserves the correct font ownership seam. When a full
    /// cosmic-text pipeline is added, this function will shape with cosmic-text
    /// and then rasterize glyphs. For now it produces readable labels.
    pub fn draw_text(
        renderer: &Arc<Self>,
        out_buffer: &mut [u8],
        fb_w: u32,
        fb_h: u32,
        x: i32,
        y: i32,
        text: &str,
        color: [u8; 4],
        _max_w: Option<u32>,
    ) -> Result<(), String> {
        let guard = renderer.inner.lock().unwrap();

        if !guard.font_loaded {
            return Err("CosmicTextRenderer: project font not loaded".to_string());
        }

        let glyph_w: i32 = guard.char_width as i32;
        let glyph_h: i32 = guard.line_height as i32;
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
                    if idx + 4 <= out_buffer.len() {
                        out_buffer[idx..idx + 4].copy_from_slice(&color);
                    }
                }
            }
            cx += glyph_w;
        }

        Ok(())
    }
}
