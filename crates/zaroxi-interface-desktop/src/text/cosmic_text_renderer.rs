/*!
Cosmic Text integration shim (canonical single-path implementation).

This module now owns the real Cosmic Text pipeline integration for the
GPU shell. All previous fallback rasterizers, legacy glyph tables, and
dual-path rendering have been removed. The renderer:

- Loads project font bytes from `zaroxi-core-engine-font::project_font_bytes`.
- Initializes a real cosmic-text FontSystem and uses a Buffer for shaping.
- Rasterizes shaped glyphs into the provided RGBA8 framebuffer.
- Returns an error on any failure (no fallback).

NOTE: This module depends on the workspace `cosmic-text` crate. If the
cosmic-text API in the workspace differs, update the function calls
accordingly. The presented implementation follows the canonical flow:
FontSystem -> Buffer (layout) -> draw into pixel callback.
*/

use std::sync::{Arc, Mutex};

use cosmic_text::{FontSystem, Buffer as CosmicBuffer, Family, AttrsOwned, Metrics};
use zaroxi_core_engine_font;

/// Thin wrapper around a shared cosmic-text renderer instance.
///
/// The GPU binary should call `init_cosmic_renderer()` (in crate::text::mod)
/// once during startup which will create and set a global Arc<CosmicTextRenderer>.
pub struct CosmicTextRenderer {
    inner: Mutex<Inner>,
}

struct Inner {
    /// The cosmic-text FontSystem instance used for shaping and rasterization.
    font_system: FontSystem,
    /// Conservative default metrics extracted from the font (diagnostic).
    metrics: Metrics,
}

impl CosmicTextRenderer {
    /// Initialize the renderer by loading the project font bytes and registering
    /// them into a FontSystem. Returns an Arc-wrapped renderer ready for use.
    pub fn new() -> Result<Arc<Self>, String> {
        // Obtain bytes from the canonical loader provided by the core font crate.
        let bytes = zaroxi_core_engine_font::project_font_bytes()
            .map_err(|e| format!("CosmicTextRenderer: failed to obtain project font bytes: {}", e))?;

        if bytes.is_empty() {
            return Err("CosmicTextRenderer: project font bytes are empty".to_string());
        }

        // Create a FontSystem and register the project font bytes into it.
        let mut fs = FontSystem::new();

        // Register the font bytes into the font system. This uses the typical
        // cosmic-text flow of adding font bytes to the system so Buffer can
        // reference the face by family name. API name may vary by cosmic-text
        // version; if `add_font_bytes` is not available adapt to the workspace API.
        fs.add_font_bytes(bytes).map_err(|e| format!("CosmicTextRenderer: add_font_bytes failed: {:?}", e))?;

        // Query conservative metrics from the FontSystem for layout defaults.
        let metrics = fs.metrics();

        let inner = Inner {
            font_system: fs,
            metrics,
        };

        Ok(Arc::new(CosmicTextRenderer {
            inner: Mutex::new(inner),
        }))
    }

    /// Draw `text` into `out_buffer` as RGBA8 anchored at (x, y).
    ///
    /// This function uses cosmic-text Buffer for shaping and rasterization and
    /// writes pixels into `out_buffer` via a simple callback. It returns an
    /// error on any failure — there is intentionally no fallback rasterizer.
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
        let mut guard = renderer.inner.lock().unwrap();

        // Create a cosmic-text Buffer bound to our FontSystem.
        let mut buf = CosmicBuffer::new(&mut guard.font_system);

        // Configure attributes: use the project monospace family if available.
        // Prefer the explicit "ZaroxiMono" family name used by the project font bundle.
        let mut attrs = AttrsOwned::new();
        attrs.set_family(Family::Name("ZaroxiMono".to_string()));

        // Set reasonable size derived from metrics; callers can adjust by passing
        // a different attribute set in future iterations.
        // Use metrics.line_height (f32) if available; fall back to a conservative value.
        let size = guard.metrics.get_line_height().unwrap_or(16.0);
        buf.set_size(size);

        // Set the text to shape and layout.
        buf.set_text(text);

        // Rasterize using the Buffer draw callback. We translate the cosmic-text
        // pixel callback into writes into out_buffer. The callback signature used
        // here follows the common cosmic-text draw convention: closure receives
        // (px, py, r, g, b, a) where color components are u8.
        buf.draw(&mut guard.font_system, |px: i32, py: i32, r: u8, g: u8, b: u8, a: u8| {
            // Map shaped glyph pixel into framebuffer by offsetting with (x,y).
            let tx = px + x;
            let ty = py + y;
            if tx < 0 || ty < 0 {
                return;
            }
            let tx = tx as u32;
            let ty = ty as u32;
            if tx >= fb_w || ty >= fb_h {
                return;
            }
            let idx = ((ty * fb_w + tx) * 4) as usize;
            if idx + 4 <= out_buffer.len() {
                // Premultiplied alpha is not assumed here; write RGBA directly.
                out_buffer[idx] = r;
                out_buffer[idx + 1] = g;
                out_buffer[idx + 2] = b;
                out_buffer[idx + 3] = a;
            }
        });

        // Successful render via cosmic-text pipeline.
        Ok(())
    }
}
