/*!
Native Glyphon-backed text renderer (real integration with glyphon 0.11.0).

This implementation uses the exact glyphon 0.11.0 API:
- Create a Cache, TextAtlas and Viewport first.
- Construct glyphon's TextRenderer via:
    TextRenderer::new(&mut atlas, device, MultisampleState, Option<DepthStencilState>)
- Prepare using:
    TextRenderer::prepare(device, queue, font_system, atlas, viewport, text_areas_iter, cache)
- Render using:
    TextRenderer::render(atlas, viewport, render_pass)

Logging policy: concise single-line logs for init, bundled font found/missing,
viewport resize and one-line prepare/render errors. Detailed tracing remains gated
behind RENDER_DEBUG.
*/

use crate::error::RenderError;
use crate::renderer::text::{TextCommand, TextRenderer};
use glyphon::{Cache, TextAtlas, TextRenderer as GlyphonRenderer, Viewport, SwashCache};
use cosmic_text::{Buffer as CosmicBuffer, Metrics as CosmicMetrics, Attrs, Shaping, Color as CosmicColor, FontSystem};
use log::{info, debug};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wgpu::{BindGroupLayout, Device, Queue, BindGroup, RenderPass, RenderPipeline, MultisampleState, DepthStencilState, TextureFormat};

use crate::renderer::debug::RENDER_DEBUG;

/// Concrete Glyphon-backed renderer.
///
/// Owns glyphon-native state (Cache, TextAtlas, Viewport, FontSystem, SwashCache,
/// and glyphon's TextRenderer). The renderer accepts high-level TextCommand
/// instances from the core renderer, constructs temporary cosmic_text Buffers
/// during prepare, and feeds glyphon-native prepare/render APIs directly.
pub struct GlyphonTextRenderer {
    cache: Cache,
    atlas: Arc<Mutex<TextAtlas>>,
    viewport: Arc<Mutex<Viewport>>,
    font_system: Arc<Mutex<FontSystem>>,
    swash_cache: Arc<Mutex<SwashCache>>,
    glyphon_renderer: Arc<Mutex<GlyphonRenderer>>,
    queued: Arc<Mutex<Vec<TextCommand>>>,
}

impl GlyphonTextRenderer {
    /// Create a new GlyphonTextRenderer.
    ///
    /// Note: requires the color format so the TextAtlas can be created with the
    /// same format used by the text pipeline.
    pub fn new(device: &Device, queue: &Queue, color_format: TextureFormat, _font_size: f32) -> Result<Self, RenderError> {
        // Create glyphon cache and atlas first (exact glyphon 0.11.0 flow).
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, color_format);
        let viewport = Viewport::new(device, &cache);

        // Initialize cosmic-text FontSystem
        let fs = FontSystem::new();

        // Try to register bundled JetBrains Mono Nerd Font (best-effort).
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path = PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        if font_path.exists() {
            match std::fs::read(&font_path) {
                Ok(_data) => {
                    // We load the file into fontdb in earlier iterations; glyphon/cosmic-text
                    // will consult system fontdb. This is a best-effort info log.
                    info!("Bundled JetBrains Mono Nerd Font found at '{}'", font_path.display());
                }
                Err(e) => {
                    info!("Bundled JetBrains Mono Nerd Font found but failed to read: {:?}; falling back to system fonts", e);
                }
            }
        } else {
            info!("Bundled JetBrains Mono Nerd Font not found, falling back to system fonts");
        }

        // Create swash cache required by glyphon prepare path.
        let swash = SwashCache::new();

        // Construct the glyphon TextRenderer using the exact glyphon 0.11.0 signature.
        // Pass a mutable reference to the atlas we just created.
        let multisample = MultisampleState::default();
        let depth_stencil: Option<DepthStencilState> = None;
        let glyphon_renderer = GlyphonRenderer::new(&mut atlas, device, multisample, depth_stencil);

        info!("GlyphonTextRenderer initialized");

        Ok(Self {
            cache,
            atlas: Arc::new(Mutex::new(atlas)),
            viewport: Arc::new(Mutex::new(viewport)),
            font_system: Arc::new(Mutex::new(fs)),
            swash_cache: Arc::new(Mutex::new(swash)),
            glyphon_renderer: Arc::new(Mutex::new(glyphon_renderer)),
            queued: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

impl TextRenderer for GlyphonTextRenderer {
    fn queue_text(&self, cmd: TextCommand) {
        let mut q = self.queued.lock().unwrap();
        q.push(cmd);
    }

    fn prepare(&self, device: &Device, queue: &mut Queue) -> Result<(), RenderError> {
        // Lock mutable glyphon state
        let mut gr = self.glyphon_renderer.lock().unwrap();
        let mut fs = self.font_system.lock().unwrap();
        let mut atlas = self.atlas.lock().unwrap();
        let viewport = self.viewport.lock().unwrap();
        let mut swash = self.swash_cache.lock().unwrap();

        let mut q = self.queued.lock().unwrap();
        if q.is_empty() {
            return Ok(());
        }

        // Convert queued TextCommand into glyphon-compatible TextArea instances.
        // We must create cosmic_text::Buffer instances that live for the duration
        // of the prepare call. Build them into a local Vec so their lifetimes
        // outlive the iterator passed to glyphon.
        let mut buffers: Vec<CosmicBuffer> = Vec::with_capacity(q.len());
        let mut areas: Vec<glyphon::TextArea> = Vec::with_capacity(q.len());

        for cmd in q.iter() {
            // Create metrics for this buffer (use font size from command).
            let metrics = CosmicMetrics::new(cmd.size, cmd.size * 1.2);

            let mut buf = CosmicBuffer::new(&mut *fs, metrics);
            // Use a simple Attrs; prefer bundled family later when available.
            let mut attrs = Attrs::new();
            // Set text and shaping
            buf.set_text(&cmd.text, &attrs, Shaping::Advanced, None);

            // Build TextBounds from clip rectangle (convert f32 -> i32)
            let bounds = glyphon::TextBounds {
                left: cmd.clip_x.max(0.0) as i32,
                top: cmd.clip_y.max(0.0) as i32,
                right: (cmd.clip_x + cmd.clip_w).max(0.0) as i32,
                bottom: (cmd.clip_y + cmd.clip_h).max(0.0) as i32,
            };

            // Default color: convert RGBA float to cosmic_text::Color (u32 packed).
            let color = {
                let r = (cmd.color[0] * 255.0) as u32;
                let g = (cmd.color[1] * 255.0) as u32;
                let b = (cmd.color[2] * 255.0) as u32;
                let a = (cmd.color[3] * 255.0) as u32;
                // Pack as 0xRRGGBBAA in u32 (cosmic_text::Color is a newtype over u32)
                CosmicColor(((r << 24) | (g << 16) | (b << 8) | a) as u32)
            };

            // Push buffer into vector so it lives
            buffers.push(buf);
        }

        // Build TextArea refs referencing buffers
        for (i, cmd) in q.iter().enumerate() {
            // SAFETY: buffers[i] exists and will live until the end of this function
            let buf_ref: &CosmicBuffer = &buffers[i];
            let area = glyphon::TextArea {
                buffer: buf_ref,
                left: cmd.x,
                top: cmd.y,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: cmd.clip_x.max(0.0) as i32,
                    top: cmd.clip_y.max(0.0) as i32,
                    right: (cmd.clip_x + cmd.clip_w).max(0.0) as i32,
                    bottom: (cmd.clip_y + cmd.clip_h).max(0.0) as i32,
                },
                default_color: color,
                custom_glyphs: &[],
            };
            areas.push(area);
        }

        // Call the glyphon prepare API with the exact signature required by 0.11.0.
        match gr.prepare(device, queue, &mut *fs, &mut *atlas, &*viewport, areas.into_iter(), &mut *swash) {
            Ok(()) => {
                // prepared successfully
            }
            Err(e) => {
                return Err(RenderError::Other(format!("Glyphon prepare failed: {:?}", e)));
            }
        }

        // Clear queued commands
        q.clear();

        Ok(())
    }

    fn render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        _pipeline: &RenderPipeline,
        _panel_indices_len: u32,
        _total_indices_len: u32,
    ) -> Result<(), RenderError> {
        // Acquire locks for atlas and viewport then delegate to glyphon::TextRenderer::render
        let atlas = self.atlas.lock().unwrap();
        let viewport = self.viewport.lock().unwrap();
        let mut gr = self.glyphon_renderer.lock().unwrap();

        if let Err(e) = gr.render(&*atlas, &*viewport, rpass) {
            return Err(RenderError::Other(format!("Glyphon render failed: {:?}", e)));
        }

        Ok(())
    }
}
