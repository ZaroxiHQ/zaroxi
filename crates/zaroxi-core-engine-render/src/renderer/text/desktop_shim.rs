/*
Desktop shim for cosmic-text-based software rendering.

This file contains the former desktop-side `cosmic_text_renderer.rs` implementation
moved into the canonical core renderer crate so there is only one place that owns
Cosmic integration. The shim exposes the same public API used by the desktop
crate: `CosmicTextRenderer` and `init_cosmic_renderer()`.

The implementation is intentionally minimal and preserves the original behavior
so the `gpu_shell` binary (minifb path) continues to work while removing the
duplicate implementation from the desktop crate.
*/

use std::sync::{Arc, Mutex};

use cosmic_text::{Attrs, Buffer as CosmicBuffer, Color, FontSystem, Metrics, Shaping, SwashCache};
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
    pub fn new() -> Result<Arc<Self>, String> {
        // Try to read project font bytes for diagnostics; log size but do not silently change behavior.
        match zaroxi_core_engine_font::project_font_bytes() {
            Ok(bytes) => {
                eprintln!("CosmicTextRenderer: loaded project font bytes: {} bytes", bytes.len());
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

        let inner = Inner { font_system: fs, metrics };

        Ok(Arc::new(CosmicTextRenderer { inner: Mutex::new(inner) }))
    }

    /// Draw `text` into `out_buffer` as RGBA8 anchored at (x, y).
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

        let req_r = color[0] as f32;
        let req_g = color[1] as f32;
        let req_b = color[2] as f32;

        buf.draw(
            &mut guard.font_system,
            &mut swash_cache,
            draw_color,
            |bx: i32, by: i32, w: u32, h: u32, c: Color| {
                // Per-rectangle coverage/color (alpha carries coverage).
                let rgba = c.as_rgba();
                let coverage_a = (rgba[3] as f32) / 255.0;

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
                            if coverage_a <= 0.0 {
                                continue;
                            }

                            // Read destination color
                            let dr = out_buffer[idx] as f32;
                            let dg = out_buffer[idx + 1] as f32;
                            let db = out_buffer[idx + 2] as f32;

                            // Blend requested color using coverage alpha (non-premultiplied src).
                            let out_r = (req_r * coverage_a + dr * (1.0 - coverage_a))
                                .round()
                                .clamp(0.0, 255.0) as u8;
                            let out_g = (req_g * coverage_a + dg * (1.0 - coverage_a))
                                .round()
                                .clamp(0.0, 255.0) as u8;
                            let out_b = (req_b * coverage_a + db * (1.0 - coverage_a))
                                .round()
                                .clamp(0.0, 255.0) as u8;

                            out_buffer[idx] = out_r;
                            out_buffer[idx + 1] = out_g;
                            out_buffer[idx + 2] = out_b;
                            out_buffer[idx + 3] = 255;
                        }
                    }
                }
            },
        );

        Ok(())
    }
}

/// Initialize a global renderer instance (compat shim).
pub fn init_cosmic_renderer() -> Result<Arc<CosmicTextRenderer>, String> {
    CosmicTextRenderer::new()
}
