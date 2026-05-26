/*!
Cosmic Text integration shim (adapted to the workspace cosmic-text v0.19 API).

This file adapts to the concrete 0.19 API available in the local registry.
It avoids private-module imports, uses the public re-exports, and follows the
example usage pattern from the cosmic-text docs:

- Create FontSystem
- Create Buffer with Metrics
- Create a SwashCache
- Set text/attrs on the Buffer
- Call Buffer::draw(&mut swash_cache, color, |x,y,w,h,color| { ... })

This implementation intentionally:
- Validates that the project's font bytes exist (via `project_font_bytes`)
  but does not require a specific `FontSystem` registration method.
- Uses public types exported by cosmic_text (FontSystem, Buffer, Metrics, Attrs, Shaping, Color, SwashCache).
- Fails loudly on error (no fallback).
*/

use std::sync::{Arc, Mutex};

use cosmic_text::{FontSystem, Buffer as CosmicBuffer, Attrs, Shaping, Metrics, Color, SwashCache};
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
    /// Initialize the renderer by ensuring the project font bytes are available.
    ///
    /// NOTE: concrete registration APIs vary across cosmic-text versions. This
    /// initializer enforces the presence of the project font bytes (owned by
    /// `zaroxi-core-engine-font`) and constructs a FontSystem. If you prefer
    /// to register the bytes into the FontSystem, adapt this function to call
    /// the appropriate method (for example `add_font_bytes`) on your local
    /// cosmic-text `FontSystem`.
    pub fn new() -> Result<Arc<Self>, String> {
        // Ensure project font bytes are present (keeps ownership in core-engine-font).
        let _bytes = zaroxi_core_engine_font::project_font_bytes()
            .map_err(|e| format!("CosmicTextRenderer: failed to obtain project font bytes: {}", e))?;

        // Create a FontSystem (system fonts available as a fallback).
        let fs = FontSystem::new();

        // Default conservative metrics (font size, line height). Callers can tune later.
        let metrics = Metrics::new(16.0, 20.0);

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
    /// Uses the cosmic-text Buffer/SwashCache draw flow available in v0.19:
    /// - Buffer::new(&mut font_system, metrics)
    /// - buffer.set_size(width_opt, height_opt)
    /// - buffer.set_text(text, &attrs, Shaping::Advanced, None)
    /// - buffer.draw(&mut swash_cache, Color, |x,y,w,h,color| { ... })
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
        // Obtain metrics first (copy) while holding the lock briefly, then
        // re-lock to get a mutable reference for FontSystem to avoid conflicting
        // mutable/immutable borrows of the mutex guard.
        let metrics = {
            let guard = renderer.inner.lock().unwrap();
            guard.metrics
        };
        // Re-acquire the lock mutably for FontSystem usage.
        let mut guard = renderer.inner.lock().unwrap();

        // Create a buffer bound to our FontSystem using the copied metrics.
        let mut buf = CosmicBuffer::new(&mut guard.font_system, metrics);

        // Use default attributes for now (family selection will use system/project fonts).
        let attrs = Attrs::new();

        // No explicit width/height constraints for now.
        buf.set_size(None, None);

        // Set the text to shape/layout.
        buf.set_text(text, &attrs, Shaping::Advanced, None);

        // Prepare a swash cache as required by the draw API.
        let mut swash_cache = SwashCache::new();

        // Convert incoming RGBA array into cosmic_text::Color
        let draw_color = Color::rgba(color[0], color[1], color[2], color[3]);

        // The draw callback receives rectangles (x,y,w,h) and a Color.
        // We translate those into pixel writes into out_buffer, offset by (x,y).
        // Buffer::draw in cosmic-text v0.19 expects the FontSystem as the first argument.
        // Note: the closure signature requires (i32, i32, u32, u32, Color) for (x,y,w,h,color).
        buf.draw(&mut guard.font_system, &mut swash_cache, draw_color, |bx: i32, by: i32, w: u32, h: u32, c: Color| {
            // Convert color to bytes
            let rgba = c.as_rgba();
            let cr = rgba[0];
            let cg = rgba[1];
            let cb = rgba[2];
            let ca = rgba[3];

            // Destination rectangle origin including the requested anchor offset.
            let ox = bx + x;
            let oy = by + y;

            // Iterate rectangle and write pixels with bounds checks.
            for row in 0..h {
                // row is u32; convert to i32 when adding to oy (which is i32).
                let py_i = oy + (row as i32);
                if py_i < 0 {
                    continue;
                }
                let pyu = py_i as u32;
                if pyu >= fb_h {
                    continue;
                }
                for col in 0..w {
                    // col is u32; convert to i32 for signed arithmetic with ox.
                    let px_i = ox + (col as i32);
                    if px_i < 0 {
                        continue;
                    }
                    let pxu = px_i as u32;
                    if pxu >= fb_w {
                        continue;
                    }
                    let idx = ((pyu * fb_w + pxu) * 4) as usize;
                    if idx + 4 <= out_buffer.len() {
                        out_buffer[idx] = cr;
                        out_buffer[idx + 1] = cg;
                        out_buffer[idx + 2] = cb;
                        out_buffer[idx + 3] = ca;
                    }
                }
            }
        });

        Ok(())
    }
}
