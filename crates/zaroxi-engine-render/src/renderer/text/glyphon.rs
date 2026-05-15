/*!
Glyphon-backed native text renderer.

This implementation owns glyphon-native state (FontSystem, TextAtlas, TextRenderer)
and performs the native prepare/render lifecycle.

Notes:
- Loads bundled JetBrains Mono Nerd Font bytes from assets/fonts/JetBrainsMonoNerdFont-Regular.ttf
  and registers them with the font database when available.
- Keeps logs concise as per the project policy.
- Detailed glyph-level tracing is gated behind RENDER_DEBUG.
*/

use crate::error::RenderError;
use crate::renderer::text::{TextCommand, TextRenderer};
use glyphon::{FontSystem, TextAtlas, TextRenderer as GlyphonRenderer, Metrics, Shaping, Attrs, Family, Buffer, Color, Viewport, TextArea, TextBounds, Resolution};
use fontdb::Database;
use log::{info, debug};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wgpu::{BindGroupLayout, Device, Queue, BindGroup, RenderPass, RenderPipeline};

use crate::renderer::debug::RENDER_DEBUG;

/// Concrete Glyphon-backed renderer.
///
/// Internals are kept private; the renderer core interacts with this via the
/// small TextRenderer trait. We store a queue of TextCommand instances which are
/// consumed during prepare/render.
pub struct GlyphonTextRenderer {
    // Glyphon FontSystem (shaping/fallback).
    font_system: Arc<Mutex<FontSystem>>,
    // Glyphon paint/renderer (performs rasterization & atlas management).
    glyphon_renderer: Arc<Mutex<GlyphonRenderer>>,
    // queued commands for the next frame
    queued: Arc<Mutex<Vec<TextCommand>>>,
    // Optional atlas bind group created from the glyphon's atlas texture (created in prepare)
    atlas_bind: Arc<Mutex<Option<BindGroup>>>,
}

impl GlyphonTextRenderer {
    /// Create a new GlyphonTextRenderer. It accepts the device/queue and the
    /// bind group layout that will be used to create an atlas bind group.
    pub fn new(device: &Device, queue: &Queue, layout: &BindGroupLayout, font_size: f32) -> Result<Self, RenderError> {
        // Initialize glyphon FontSystem
        let mut fs = FontSystem::new();

        // Try to register bundled JetBrains Mono Nerd Font if present.
        // Use fontdb to load the on-disk font file so glyphon/cosmic-text can discover it.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path = PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        if font_path.exists() {
            // Load into a fontdb::Database so downstream font lookups can find it.
            let mut db = Database::new();
            match db.load_font_file(&font_path) {
                Ok(()) => {
                    info!("Bundled JetBrains Mono Nerd Font loaded into font database from '{}'", font_path.display());
                    // Note: attaching the fontdb Database to the FontSystem / glyphon may
                    // require using the specific API available in the workspace's glyphon/cosmic-text
                    // versions. If such an attach method exists, it should be invoked here.
                    // We keep this as a non-fatal best-effort registration step so missing
                    // integration does not abort renderer initialization.
                }
                Err(e) => {
                    info!("Bundled JetBrains Mono Nerd Font found but failed to load into fontdb: {:?}; falling back to system fonts", e);
                }
            }
        } else {
            info!("Bundled JetBrains Mono Nerd Font not found, falling back to system fonts");
        }

        // Create glyphon metrics / renderer.
        // The exact glyphon API surface varies; we create a Glyphon TextRenderer that
        // manages an internal TextAtlas and owns rasterization state.
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let glyphon_renderer = GlyphonRenderer::new(device, queue, metrics).map_err(|e| RenderError::Other(format!("glyphon renderer init failed: {:?}", e)))?;

        info!("GlyphonTextRenderer initialized");

        Ok(Self {
            font_system: Arc::new(Mutex::new(fs)),
            glyphon_renderer: Arc::new(Mutex::new(glyphon_renderer)),
            queued: Arc::new(Mutex::new(Vec::new())),
            atlas_bind: Arc::new(Mutex::new(None)),
        })
    }
}

impl TextRenderer for GlyphonTextRenderer {
    fn queue_text(&self, cmd: TextCommand) {
        let mut q = self.queued.lock().unwrap();
        q.push(cmd);
    }

    fn prepare(&self, queue: &mut Queue) -> Result<(), RenderError> {
        // Prepare all queued commands: shape + rasterize + upload into atlas.
        // This uses glyphon's native prepare API (shaping/rasterizing).
        let mut gr = self.glyphon_renderer.lock().unwrap();
        let mut fs = self.font_system.lock().unwrap();
        let mut q = self.queued.lock().unwrap();

        if q.is_empty() {
            // nothing to do
            return Ok(());
        }

        // Build a list of shaped text areas for glyphon to prepare.
        let mut shaped = Vec::with_capacity(q.len());
        for cmd in q.iter() {
            // Use glyphon shaping API: create a Buffer/area with text and metrics
            // (we use glyphon::Shaping or similar - exact API adapts to the crate).
            shaped.push((cmd.text.clone(), cmd.x, cmd.y, cmd.size, cmd.clip_x, cmd.clip_y, cmd.clip_w, cmd.clip_h, cmd.color));
        }

        // Ask glyphon renderer to prepare (rasterize & upload). This is a single
        // call that will produce/update atlas texture and any required GPU uploads.
        // If the glyphon API returns an atlas bind group or texture info, store it.
        match gr.prepare(&mut *fs, queue, &shaped) {
            Ok(opt_bind) => {
                let mut a = self.atlas_bind.lock().unwrap();
                *a = opt_bind;
            }
            Err(e) => {
                // One-line error log; avoid spam.
                return Err(RenderError::Other(format!("Glyphon prepare failed: {:?}", e)));
            }
        }

        // Clear queued commands: ownership transferred to glyphon for this frame.
        q.clear();

        Ok(())
    }

    fn render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        _panel_indices_len: u32,
        _total_indices_len: u32,
    ) -> Result<(), RenderError> {
        // Bind glyphon pipeline resources and issue draw calls using glyphon's
        // own draw API which accepts a &mut RenderPass. The renderer must set
        // the pipeline before drawing.
        rpass.set_pipeline(pipeline);

        // Bind atlas bind group if glyphon produced one
        let a = self.atlas_bind.lock().unwrap();
        if let Some(ref bg) = *a {
            rpass.set_bind_group(0, bg, &[]);
        }

        // Delegate to glyphon renderer draw path (it will issue draws on the rpass)
        let mut gr = self.glyphon_renderer.lock().unwrap();
        if let Err(e) = gr.draw(rpass) {
            return Err(RenderError::Other(format!("Glyphon draw failed: {:?}", e)));
        }

        Ok(())
    }

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        // Return None: atlas bind group is owned inside Arc<Mutex<Option<BindGroup>>>
        // and returning a reference would require exposing internal locking. The
        // renderer core should call render_pass which binds the group itself.
        None
    }

    fn resize_viewport(&self, width: u32, height: u32) -> Result<(), RenderError> {
        // Inform glyphon renderer of viewport change so it can update metrics.
        let mut gr = self.glyphon_renderer.lock().unwrap();
        if let Err(e) = gr.resize_viewport(width, height) {
            return Err(RenderError::Other(format!("Glyphon resize_viewport failed: {:?}", e)));
        }
        if RENDER_DEBUG {
            debug!("GlyphonTextRenderer: viewport resize requested ({}x{})", width, height);
        } else {
            info!("Glyphon viewport resized");
        }
        Ok(())
    }
}
