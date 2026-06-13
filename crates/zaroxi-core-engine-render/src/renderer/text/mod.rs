/*!
Text subsystem

Provides the Cosmic Text–backed text rendering pipeline used by renderer core.

Design summary:
- TextCommand: small command model emitted by the renderer core for each text
  item (title/body). Commands are queued and consumed by the CosmicText renderer.
- TextRenderer trait: minimal interface used by core:
    - queue_text(cmd)
    - prepare(queue) -> perform glyph rasterization / GPU atlas uploads
    - render_pass(rpass, pipeline, panel_indices_len, total_indices_len)
    - resize_viewport(w,h)
- CosmicTextRenderer: concrete implementation that owns cosmic-text native
  state (FontSystem, SwashCache, SharedAtlas) and the wgpu instance buffer.

CosmicTextRenderer is the single authoritative text renderer for GUI text.
*/

use crate::error::RenderError;
use log::info;
use std::sync::Mutex;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass, RenderPipeline};

pub mod cosmic;
pub mod desktop_shim;
pub use cosmic::CosmicTextRenderer;

/// Small in-process command representing text to be rendered.
///
/// The renderer core emits these commands per panel title/content. The native
/// Cosmic renderer consumes them, performs shaping/rasterization in `prepare`
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
    pub fn new(
        text: impl Into<String>,
        x: f32,
        y: f32,
        color: [f32; 4],
        size: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
        is_title: bool,
    ) -> Self {
        Self { text: text.into(), x, y, color, size, clip_x, clip_y, clip_w, clip_h, is_title }
    }

    pub fn new_title(
        text: &str,
        x: f32,
        y: f32,
        color: [f32; 4],
        size: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Self {
        Self::new(text, x, y, color, size, clip_x, clip_y, clip_w, clip_h, true)
    }

    pub fn new_body(
        text: &str,
        x: f32,
        y: f32,
        color: [f32; 4],
        size: f32,
        clip_x: f32,
        clip_y: f32,
        clip_w: f32,
        clip_h: f32,
    ) -> Self {
        Self::new(text, x, y, color, size, clip_x, clip_y, clip_w, clip_h, false)
    }
}

/// Minimal internal trait used by renderer core to plan/prepare/render text.
///
/// The goal is to keep the rest of the renderer backend-agnostic while giving
/// the CosmicText backend ownership of the native prepare/render lifecycle.
pub trait TextRenderer: Send + Sync {
    /// Queue a text command for the upcoming frame.
    fn queue_text(&self, cmd: TextCommand);

    /// Return number of queued text commands waiting to be prepared.
    ///
    /// This allows renderer core to decide to invoke native prepare/render even
    /// when legacy vertex/index counters are zero.
    fn queued_len(&self) -> usize;

    /// Prepare glyphs for queued commands: shape, rasterize and upload any GPU resources.
    ///
    /// The prepare step needs access to the Device to create or update GPU
    /// resources (textures, buffers) during prepare. Device is
    /// passed in along with the Queue.
    fn prepare(&self, device: &Device, queue: &mut Queue) -> Result<(), RenderError>;

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
    fn atlas_bind_group(&self) -> Option<&BindGroup> {
        None
    }

    /// Update viewport/resolution information.
    fn resize_viewport(&self, width: u32, height: u32) -> Result<(), RenderError> {
        info!("TextRenderer: viewport resize requested ({}x{})", width, height);
        Ok(())
    }

    /// Return the monospace character advance width in logical pixels,
    /// as computed from the actual loaded font metrics. Returns None
    /// if the backend cannot determine a reliable advance value.
    fn monospace_advance_x(&self) -> Option<f32> {
        None
    }
}
