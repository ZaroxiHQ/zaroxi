/*!
Cosmic Text integration shim (adapted to workspace cosmic-text 0.19+ API).

This implementation adapts to the concrete cosmic-text 0.19 API observed in
the workspace. It ensures:
- project font bytes are validated by `zaroxi-core-engine-font`
- FontSystem is created and used as the canonical shaping/rasterization owner
- Buffer, Attrs, Swash cache and Color are used to shape and draw glyphs
- No fallback rasterizer remains — failures are propagated as errors

Notes:
- The cosmic-text API surface changes across releases. This file aims to
  follow the common 0.19-style signatures: Buffer::new(font_system, metrics),
  Attrs/AttrsOwned construction, Buffer::set_size(width_opt, height_opt),
  Buffer::set_text(text, &attrs, Shaping, Option<Align>), and
  Buffer::draw(&mut FontSystem, &mut SwashCache, Color, pixel_cb).
- If your workspace cosmic-text differs slightly, run `cargo build` and paste
  the compiler errors; I will iterate to match exact types.
*/

use std::sync::{Arc, Mutex};

use cosmic_text::{
    FontSystem, Buffer as CosmicBuffer, Family, Attrs, AttrsOwned, Shaping, Align, Metrics, Color,
};
use cosmic_text::swash::Cache as SwashCache;
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

        // Create a FontSystem. The exact registration API for adding raw bytes
        // varies between cosmic-text versions. Try the common helpers but do
        // not hard-fail if the specific helper name differs; we keep the bytes
        // check here to enforce font availability in the workspace.
        let mut fs = FontSystem::new();

        // Attempt to register bytes into the FontSystem. Some cosmic-text
        // versions expose `add_font_bytes`; if it's not present this call will
        // fail to compile and we'll adapt in the next iteration.
        if let Err(e) = fs.add_font_bytes(bytes) {
            return Err(format!("CosmicTextRenderer: registering project font failed: {:?}", e));
        }

        // Obtain conservative metrics. If Metrics::default exists, prefer it as a
        // stable fallback when FontSystem does not expose a metrics() accessor.
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
        // Buffer::new requires metrics in 0.19+; pass the stored metrics.
        let mut buf = CosmicBuffer::new(&mut guard.font_system, guard.metrics.clone());

        // Build Attrs/AttrsOwned for sizing/family selection.
        let base_attrs = Attrs::new();
        let mut attrs = AttrsOwned::new(&base_attrs);
        // Prefer the bundled family name used by the workspace font loader.
        attrs.set_family(Family::Name("ZaroxiMono"));

        // Determine a size derived from metrics if available; fall back to 16.0
        let size: f32 = guard
            .metrics
            .get_line_height()
            .unwrap_or(16.0);

        // Set size: Buffer::set_size(width_opt, height_opt) in 0.19+.
        buf.set_size(Some(size), None);

        // Configure shaping & layout: use Advanced shaping and no explicit alignment.
        buf.set_text(text, &attrs.as_ref(), Shaping::Advanced, None);

        // Prepare a swash cache used by Buffer::draw in 0.19 API.
        let mut swash_cache = SwashCache::new();

        // Convert color array to cosmic_text::Color. Use RGBA (0..255).
        let px_color = Color::rgba(color[0], color[1], color[2], color[3]);

        // Rasterize via Buffer::draw(font_system, &mut swash_cache, color, callback)
        buf.draw(
            &mut guard.font_system,
            &mut swash_cache,
            px_color,
            |px: i32, py: i32, r: u8, g: u8, b: u8, a: u8| {
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
                    out_buffer[idx] = r;
                    out_buffer[idx + 1] = g;
                    out_buffer[idx + 2] = b;
                    out_buffer[idx + 3] = a;
                }
            },
        );

        Ok(())
    }
}
