/*!
Minimal Cosmic Text integration shim.

Responsibilities:
- Initialize a lightweight check that the project's font bytes are available
  via the workspace font loader.
- Provide a small, deterministic API to render UTF-8 strings into an RGBA8 framebuffer.
- Keep the implementation robust and compileable across cosmic-text API drift
  by avoiding direct, fragile calls into cosmic-text's unstable Buffer API here.
  The presence of the font bytes is still validated so future phases can safely
  enable a richer cosmic-backed rasterizer.

Notes:
- This conservative implementation exercises the font loader (zaroxi-core-engine-font)
  and then renders readable per-character filled rectangles as a stable interim
  rasterization strategy. This keeps the presenter visible and deterministic
  while allowing a follow-up phase to plug a full glyph-atlas rasterizer.
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
    /// Conservative fixed metrics used for the interim rasterizer.
    line_height: u32,
    char_width: u32,
}

impl CosmicTextRenderer {
    /// Initialize the renderer by ensuring the project's font bytes are loadable.
    /// This avoids coupling to cosmic-text's unstable Buffer API while keeping
    /// a clear, canonical loader seam in the core font crate.
    pub fn new() -> Result<Arc<Self>, String> {
        // Try to load project font bytes via the canonical loader.
        match zaroxi_core_engine_font::load_project_font_bytes() {
            Ok(bytes) if !bytes.is_empty() => {
                // We successfully located the project's TTF. Use conservative metrics
                // for the interim rasterizer; future phases should derive metrics
                // from the actual font metrics via cosmic-text.
                let inner = Inner {
                    font_loaded: true,
                    line_height: 16,
                    char_width: 8,
                };
                Ok(Arc::new(CosmicTextRenderer { inner: Mutex::new(inner) }))
            }
            Ok(_) => Err("project font loader returned empty bytes".to_string()),
            Err(e) => Err(format!("CosmicTextRenderer: failed to load project font bytes: {}", e)),
        }
    }

    /// Draw `text` into `buffer` as RGBA8, anchored at (x, y). `max_w` is ignored
    /// by this conservative rasterizer but kept in the signature for future use.
    ///
    /// This implementation renders a filled rectangle per character using the
    /// conservative metrics. It returns Err when the font bytes were not available
    /// during initialization so callers can choose a controlled fallback.
    pub fn draw_text(
        renderer: &Arc<Self>,
        buffer: &mut [u8],
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
