//! Optional vello rendering layer (feature `vello_pipeline`).
//!
//! Wires a [`vello::Renderer`] into the backend: it rasterizes a
//! [`vello::Scene`] into an intermediate GPU **storage** target texture sized to
//! the surface. This is the deferred "phase 2" core — vello has no
//! `render_to_surface`, so the on-screen path is: render → storage target →
//! composite onto the swapchain.
//!
//! NOTE: the final composite/blit of the target onto the presentation surface
//! depends on this workspace's (non-standard) wgpu copy/blit API and must be
//! validated on a real GPU; this module owns the vello renderer + target so that
//! step has a verified, correctly-sized source to read from.

use vello::peniko::Color;
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};
use wgpu::{
    Device, Extent3d, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

/// Owns the vello renderer and a resizable intermediate render target.
pub struct VelloLayer {
    renderer: Renderer,
    target: Option<RenderTarget>,
    format: TextureFormat,
}

struct RenderTarget {
    #[allow(dead_code)]
    texture: Texture,
    view: TextureView,
    width: u32,
    height: u32,
}

impl VelloLayer {
    /// Create a vello layer for `device`, producing targets in `format`.
    ///
    /// `format` should be the swapchain format so the target can be composited
    /// onto the surface without conversion.
    pub fn new(device: &Device, format: TextureFormat) -> Result<Self, vello::Error> {
        let renderer = Renderer::new(device, RendererOptions::default())?;
        Ok(Self { renderer, target: None, format })
    }

    /// (Re)allocate the intermediate target if the size changed.
    fn ensure_target(&mut self, device: &Device, width: u32, height: u32) {
        let stale = match &self.target {
            Some(t) => t.width != width || t.height != height,
            None => true,
        };
        if stale {
            let texture = device.create_texture(&TextureDescriptor {
                label: Some("zaroxi-vello-target"),
                size: Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                // vello fine-rasterization writes via a storage binding; COPY_SRC
                // + TEXTURE_BINDING let a later step blit/sample it to the surface.
                usage: TextureUsages::STORAGE_BINDING
                    | TextureUsages::COPY_SRC
                    | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&TextureViewDescriptor::default());
            self.target = Some(RenderTarget { texture, view, width, height });
        }
    }

    /// Rasterize `scene` into the intermediate target (allocating/resizing it as
    /// needed). The target view is then available via [`VelloLayer::target_view`]
    /// for compositing onto the surface.
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        scene: &Scene,
        base_color: Color,
        width: u32,
        height: u32,
    ) -> Result<(), vello::Error> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.ensure_target(device, width, height);
        // Disjoint field borrows: `target.view` (shared) + `renderer` (mut).
        let view = &self.target.as_ref().expect("target ensured above").view;
        let params =
            RenderParams { base_color, width, height, antialiasing_method: AaConfig::Area };
        self.renderer.render_to_texture(device, queue, scene, view, &params)
    }

    /// The current intermediate target view (the most recent [`render`] output),
    /// for compositing onto the presentation surface.
    ///
    /// [`render`]: VelloLayer::render
    pub fn target_view(&self) -> Option<&TextureView> {
        self.target.as_ref().map(|t| &t.view)
    }
}
