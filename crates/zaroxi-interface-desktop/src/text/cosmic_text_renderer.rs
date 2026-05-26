/*!
Cosmic Text integration shim.

Responsibilities:
- Obtain project font bytes from the canonical font loader (zaroxi-core-engine-font).
- Construct and own a cosmic_text::FontSystem instance and register the project font.
- Provide a small, deterministic API to render UTF-8 strings into an RGBA8 framebuffer
  by shaping with Cosmic Text and rasterizing the shaped glyph runs into the buffer.
- Keep ownership of font asset discovery inside `zaroxi-core-engine-font` and keep
  all cosmic-text types inside this crate (zaroxi-interface-desktop).
*/

use std::sync::{Arc, Mutex};

use zaroxi_core_engine_font;

/// Use the workspace-provided cosmic-text dependency for shaping and layout.
use cosmic_text::FontSystem;
use cosmic_text::Buffer as CosmicBuffer;
use cosmic_text::Attrs;

/// Thin wrapper around a shared renderer-like instance.
///
/// The GPU binary will create one renderer and reuse it for all frames.
/// We keep a Mutex to allow the synchronous paint executor to borrow it.
pub struct CosmicTextRenderer {
    inner: Mutex<Inner>,
}

struct Inner {
    /// Whether the project font bytes were successfully loaded and registered.
    font_loaded: bool,
    /// Owned cosmic FontSystem used for shaping/rasterization.
    font_system: FontSystem,
    /// A shared empty buffer used as a scratch for shaping each label.
    /// We allocate per-draw to avoid cross-frame state coupling; keeping a
    /// single buffer here allows reuse if desired.
    // Note: CosmicBuffer is intentionally a light-weight object that holds layout.
    buffer: CosmicBuffer,
}

impl CosmicTextRenderer {
    /// Initialize the renderer by ensuring the project's font bytes are loadable
    /// and registering them into a FontSystem.
    ///
    /// Rationale:
    /// - `zaroxi-core-engine-font` owns discovery of the project font bytes.
    /// - `zaroxi-interface-desktop` constructs FontSystem and Buffer to perform
    ///   shaping and rasterization at runtime (keeps cosmic-text types local to this crate).
    pub fn new() -> Result<Arc<Self>, String> {
        // Obtain bytes from the canonical loader provided by the core font crate.
        let bytes = zaroxi_core_engine_font::project_font_bytes().map_err(|e| {
            format!("CosmicTextRenderer: failed to obtain project font bytes: {}", e)
        })?;

        if bytes.is_empty() {
            return Err("CosmicTextRenderer: project font bytes are empty".to_string());
        }

        // Construct a FontSystem and register the project font bytes.
        // The cosmic-text API provides an owned FontSystem which we keep in this crate.
        let mut fs = FontSystem::new();

        // Register project font bytes into the font system. The exact API below
        // mirrors the common cosmic-text insertion method: `add_font_bytes`.
        // If the specific method name changes in upstream cosmic-text, adapt here.
        // We intentionally do not make core-engine-font depend on cosmic-text.
        fs.add_font_bytes("ZaroxiProjectFont", &bytes);

        // Create an empty buffer associated with this font system for shaping calls.
        let buffer = CosmicBuffer::new(&fs);

        let inner = Inner {
            font_loaded: true,
            font_system: fs,
            buffer,
        };

        Ok(Arc::new(CosmicTextRenderer {
            inner: Mutex::new(inner),
        }))
    }

    /// Draw `text` into `buffer` as RGBA8, anchored at (x, y). `max_w` is an
    /// optional maximum width for shaping; when provided the buffer will wrap.
    ///
    /// Implementation notes:
    /// - We shape using Cosmic Text Buffer APIs and then rasterize glyph runs into
    ///   the provided RGBA8 framebuffer. For now we perform a conservative raster
    ///   of the glyphs by filling glyph boxes using Cosmic Text metrics to keep
    ///   a small, dependency-contained raster path in this crate.
    /// - This function keeps the actual font bytes & discovery in core-engine-font.
    pub fn draw_text(
        renderer: &Arc<Self>,
        out_buffer: &mut [u8],
        fb_w: u32,
        fb_h: u32,
        x: i32,
        y: i32,
        text: &str,
        color: [u8; 4],
        max_w: Option<u32>,
    ) -> Result<(), String> {
        let mut guard = renderer.inner.lock().unwrap();

        if !guard.font_loaded {
            return Err("CosmicTextRenderer: project font not loaded".to_string());
        }

        // Prepare attrs and set text into the cosmic buffer for shaping.
        let mut attrs = Attrs::new();
        // Use a conservative font size (in pixels). Consumers may later provide this.
        attrs.set_font_size(14.0);

        // Reset and set text in the buffer.
        guard.buffer.set_text(text, &guard.font_system);
        guard.buffer.set_width(max_w.unwrap_or((fb_w as u32) - (x as u32)) as f32, &guard.font_system);

        // Shape the buffer (layout)
        guard.buffer.shape_until_scroll(&guard.font_system);

        // Obtain the positioned glyph runs. The cosmic buffer exposes a sequence of glyph clusters.
        // We iterate glyph runs and rasterize a conservative glyph rectangle for each glyph.
        // This uses the glyph metrics as returned by the font system / buffer.
        let lines = guard.buffer.lines_count();
        let mut pen_y = y as i32;

        for line_idx in 0..lines {
            let line = guard.buffer.line(line_idx);
            // line.origin.x/y are f32 positions; convert to i32
            let mut pen_x = x as i32 + line.origin_x().round() as i32;
            // Use line height from font system metrics.
            let lh = guard.font_system.line_height();
            // Iterate glyphs in the line
            for run in line.runs() {
                for glyph in run.glyphs() {
                    // glyph has .px and .py positions and width/height metrics
                    let gx = pen_x + glyph.x.round() as i32;
                    let gy = pen_y + glyph.y.round() as i32;
                    let gw = glyph.w as i32;
                    let gh = glyph.h as i32;

                    // Rasterize a conservative filled rectangle per glyph into out_buffer.
                    for ry in 0..gh {
                        let py = gy + ry;
                        if py < 0 || py as u32 >= fb_h {
                            continue;
                        }
                        for rx in 0..gw {
                            let px = gx + rx;
                            if px < 0 || px as u32 >= fb_w {
                                continue;
                            }
                            let idx = ((py as u32 * fb_w + px as u32) * 4) as usize;
                            if idx + 4 <= out_buffer.len() {
                                out_buffer[idx..idx + 4].copy_from_slice(&color);
                            }
                        }
                    }
                }
            }
            pen_y += lh as i32;
        }

        Ok(())
    }
}
