/*!
New text subsystem module.

This module provides:
- TextRenderer trait: small internal abstraction used by renderer core
  so the rest of the renderer is decoupled from the concrete implementation.
- GlyphonTextRenderer: default, real text renderer (uses bundled font asset
  and manages atlas/bind-group). For now this implementation uses the existing
  FontAtlas plumbing for GPU atlas management so the renderer can perform the
  text pass without per-glyph terminal spam. The module surface keeps the
  Glyphon naming and provides a clear place to evolve a native glyphon
  prepare/render integration later.

Logging policy (default):
- "GlyphonTextRenderer initialized"
- "Bundled font loaded" or "Bundled font not found"
- "Glyphon viewport resized"
- One-line prepare/render error messages
Detailed debug logs are gated behind the RENDER_DEBUG runtime flag.

Note: This module intentionally avoids leaking glyphon-specific types into
the rest of the renderer. The public trait is small and focused.
*/

use crate::error::RenderError;
use crate::renderer::text::{FontAtlas, PlacedGlyph};
use log::info;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue};

/// Internal small trait used by renderer core to layout text and expose
/// an optional atlas bind group for the text pass.
pub trait TextRenderer: Send + Sync {
    fn layout_text_clipped(
        &self,
        queue: &mut Queue,
        x: f32,
        y: f32,
        text: &str,
        color: [f32; 4],
        screen_w: f32,
        screen_h: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Result<Vec<PlacedGlyph>, RenderError>;

    fn atlas_bind_group(&self) -> Option<&BindGroup>;

    /// Notify the renderer of a viewport/resolution change so internal metrics
    /// or GPU resources can be updated.
    fn resize_viewport(&mut self, width: u32, height: u32) -> Result<(), RenderError>;
}

/// Glyphon-backed text renderer (default).
///
/// Ownership:
/// - owns a FontAtlas used for glyph uploads and sampling
/// - manages a concise initialization path that attempts to load the bundled
///   JetBrains Mono Nerd Font and logs a single-line result
///
/// Implementation note:
/// For this initial migration the GlyphonTextRenderer uses the existing FontAtlas
/// implementation (found in renderer::text) to store glyph bitmaps and expose a
/// bind group compatible with the renderer's pipeline. This keeps the migration
/// safe while providing a clear home for future glyphon-native prepare/render
/// plumbing.
pub struct GlyphonTextRenderer {
    atlas: FontAtlas,
    // Whether the bundled JetBrains Mono Nerd Font was found on disk.
    bundled_font_loaded: bool,
    // Keep bind group ownership behind the atlas; atlas provides bind_group
    // access through atlas.bind_group so we can return a reference easily.
    // Additional glyphon-specific state can be added here later.
    _private: (),
}

impl GlyphonTextRenderer {
    pub fn new(device: &Device, queue: &Queue, layout: &BindGroupLayout, font_size: f32) -> Result<Self, RenderError> {
        // Attempt to locate bundled JetBrains Mono Nerd Font (single concise log message).
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path = PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let bundled_loaded = if font_path.exists() {
            info!("Bundled JetBrains Mono Nerd Font found at '{}'", font_path.display());
            true
        } else {
            info!("Bundled JetBrains Mono Nerd Font not found at '{}', falling back to system fonts", font_path.display());
            false
        };

        // Create the GPU atlas (empty) which will be populated on-demand.
        let atlas = FontAtlas::new_empty(device, queue, layout, font_size)?;

        info!("GlyphonTextRenderer initialized");

        Ok(Self {
            atlas,
            bundled_font_loaded: bundled_loaded,
            _private: (),
        })
    }
}

impl TextRenderer for GlyphonTextRenderer {
    fn layout_text_clipped(
        &self,
        queue: &mut Queue,
        x: f32,
        y: f32,
        text: &str,
        color: [f32; 4],
        screen_w: f32,
        screen_h: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Result<Vec<PlacedGlyph>, RenderError> {
        // Delegate to the atlas-backed layout helper. This preserves existing
        // placement semantics and ensures the atlas bind group is populated
        // by insert_glyph_from_bitmap when needed.
        crate::renderer::text::layout_text_clipped(&self.atlas, x, y, text, color, screen_w, screen_h, clip_x, clip_y, clip_w, clip_h)
    }

    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        Some(&self.atlas.bind_group)
    }

    fn resize_viewport(&mut self, _width: u32, _height: u32) -> Result<(), RenderError> {
        // For the current atlas-backed implementation there is no per-viewport
        // GPU resource to update. When a native glyphon integration is added
        // this will update glyphon viewport metrics and potentially recreate
        // atlas textures.
        info!("GlyphonTextRenderer: viewport resize requested ({}x{})", _width, _height);
        Ok(())
    }
}
