/*!
Text subsystem

This module exposes a small internal TextRenderer trait used by renderer core
and provides the native Glyphon-backed implementation under `glyphon.rs`.

Design summary:
- TextCommand: small command model emitted by the renderer core for each text
  item (title/body). Commands are queued and consumed by the native Glyphon
  prepare/render flow.
- TextRenderer trait: minimal interface used by core:
    - queue_text(cmd)
    - prepare(queue) -> perform glyph rasterization / GPU uploads
    - render_pass(rpass, pipeline, panel_indices_len, total_indices_len)
    - resize_viewport(w,h)
- glyphon::GlyphonTextRenderer: concrete implementation that owns glyphon-native
  state (FontSystem, TextAtlas, TextRenderer) and registers the bundled JetBrains
  Mono Nerd Font bytes as the preferred family if available.

The default implementation is fully native Glyphon; legacy FontAtlas-based
code is gated behind the `legacy_cosmic` Cargo feature and is not used by default.
*/

use crate::error::RenderError;
use log::info;
use std::sync::Mutex;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline};

pub mod glyphon;
pub use glyphon::GlyphonTextRenderer;

/// Small in-process command representing text to be rendered.
///
/// The renderer core emits these commands per panel title/content. The native
/// Glyphon renderer consumes them, performs shaping/rasterization in `prepare`
/// and draws them in `render_pass`.
#[derive(Debug, Clone)]
pub struct TextCommand {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub color: [f32; 4],
    pub size: f32,
    pub clip_x: f32,
    pub clip_y: f32,
    pub clip_w: f32,
    pub clip_h: f32,
    pub is_title: bool,
}

impl TextCommand {
    pub fn new(text: impl Into<String>, x: f32, y: f32, color: [f32;4], size: f32, clip_x: f32, clip_y: f32, clip_w: f32, clip_h: f32, is_title: bool) -> Self {
        Self {
            text: text.into(),
            x,
            y,
            color,
            size,
            clip_x,
            clip_y,
            clip_w,
            clip_h,
            is_title,
        }
    }

    pub fn new_title(text: &str, x: f32, y: f32, color: [f32;4], size: f32, clip_x: f32, clip_y: f32, clip_w: f32, clip_h: f32) -> Self {
        Self::new(text, x, y, color, size, clip_x, clip_y, clip_w, clip_h, true)
    }

    pub fn new_body(text: &str, x: f32, y: f32, color: [f32;4], size: f32, clip_x: f32, clip_y: f32, clip_w: f32, clip_h: f32) -> Self {
        Self::new(text, x, y, color, size, clip_x, clip_y, clip_w, clip_h, false)
    }
}

/// Minimal internal trait used by renderer core to plan/prepare/render text.
///
/// The goal is to keep the rest of the renderer glyphon-agnostic while giving
/// the Glyphon backend ownership of the native prepare/render lifecycle.
pub trait TextRenderer: Send + Sync {
    /// Queue a text command for the upcoming frame.
    fn queue_text(&self, cmd: TextCommand);

    /// Prepare glyphs for queued commands: shape, rasterize and upload any GPU resources.
    fn prepare(&self, queue: &mut Queue) -> Result<(), RenderError>;

    /// Render queued/ prepared text into the provided render pass. This method
    /// must bind any atlas bind groups and issue draw calls. It is called after
    /// shape/background drawing to preserve draw ordering.
    fn render_pass<'a>(
        &self,
        rpass: &mut RenderPass<'a>,
        pipeline: &RenderPipeline,
        panel_indices_len: u32,
        total_indices_len: u32,
    ) -> Result<(), RenderError>;

    /// Return an optional atlas bind group to be used by the renderer if it
    /// needs access to it for compatibility with existing submit paths.
    fn atlas_bind_group(&self) -> Option<&BindGroup> { None }

    /// Update viewport/resolution information.
    fn resize_viewport(&self, width: u32, height: u32) -> Result<(), RenderError> {
        info!("TextRenderer: viewport resize requested ({}x{})", width, height);
        Ok(())
    }
}
