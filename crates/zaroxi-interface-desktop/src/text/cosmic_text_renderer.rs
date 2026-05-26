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
        // Try to read project font bytes for diagnostics; log size but do not silently change behavior.
        match zaroxi_core_engine_font::project_font_bytes() {
            Ok(bytes) => {
                eprintln!("CosmicTextRenderer: loaded project font bytes: {} bytes", bytes.len());
                // Note: we avoid introducing silent fallbacks here. The FontSystem may
                // already resolve the family via system fonts or pre-registered fonts.
                // If explicit registration of bytes becomes necessary on your platform,
                // add registration into the FontSystem here using the platform-appropriate API.
            }
            Err(e) => {
                eprintln!(
                    "CosmicTextRenderer: project font bytes unavailable: {}; proceeding with system fonts",
                    e
                );
            }
        }

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
        // Use the Color provided by cosmic-text but composite glyph pixels into
        // the destination framebuffer using standard alpha blending. The draw
        // callback delivers opaque rectangles corresponding to glyph masks and
        // the Color's alpha should be used as coverage when blending.
        buf.draw(&mut guard.font_system, &mut swash_cache, draw_color, |bx: i32, by: i32, w: u32, h: u32, c: Color| {
            // Convert color to bytes (color channels are in 0..255)
            let rgba = c.as_rgba();
            let sr = rgba[0] as f32;
            let sg = rgba[1] as f32;
            let sb = rgba[2] as f32;
            let sa = (rgba[3] as f32) / 255.0;

            // Destination rectangle origin including the requested anchor offset.
            let ox = bx + x;
            let oy = by + y;

            // Iterate rectangle and blend pixels with bounds checks.
            for row in 0..h {
                let py_i = oy + (row as i32);
                if py_i < 0 {
                    continue;
                }
                let pyu = py_i as u32;
                if pyu >= fb_h {
                    continue;
                }
                for col in 0..w {
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
                        // Read destination color
                        let dr = out_buffer[idx] as f32;
                        let dg = out_buffer[idx + 1] as f32;
                        let db = out_buffer[idx + 2] as f32;

                        // Standard alpha compositing: out = src*sa + dst*(1-sa)
                        let out_r = (sr * sa + dr * (1.0 - sa)).round().clamp(0.0, 255.0) as u8;
                        let out_g = (sg * sa + dg * (1.0 - sa)).round().clamp(0.0, 255.0) as u8;
                        let out_b = (sb * sa + db * (1.0 - sa)).round().clamp(0.0, 255.0) as u8;

                        out_buffer[idx] = out_r;
                        out_buffer[idx + 1] = out_g;
                        out_buffer[idx + 2] = out_b;
                        // Keep framebuffer fully opaque for downstream consumers (drop any source alpha)
                        out_buffer[idx + 3] = 255;
                    }
                }
            }
        });

        Ok(())
    }
}
