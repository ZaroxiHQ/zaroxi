use crate::error::RenderError;
use log::{debug, info};
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use wgpu::{
    Backends, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, PresentMode, RequestAdapterOptions, Surface, SurfaceConfiguration,
    TextureFormat, TextureUsages, TextureViewDescriptor, Color, Queue, LoadOp, Operations, StoreOp,
    TextureView,
};

/// GPU renderer owning the device, queue, and surface.
pub struct Renderer {
    instance: Instance,
    surface: Surface,
    adapter: wgpu::Adapter,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    clear_color: Color,
    window: Arc<Window>,
}

impl Renderer {
    /// Initialize the GPU renderer asynchronously.
    pub async fn new(window: Arc<Window>, clear_color: [f64; 4]) -> Result<Self, RenderError> {
        // Construct an InstanceDescriptor compatible with wgpu 29.x.
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            flags: wgpu::InstanceFlags::empty(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        let surface = instance
            .create_surface(&*window)
            .map_err(|e| RenderError::Other(format!("Failed to create surface: {:?}", e)))?;

        // Request an adapter compatible with the surface.
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| RenderError::Other("No compatible GPU adapter found".to_string()))?;

        // Minimal device requirements for v1.
        let required_features = Features::empty();
        let required_limits = Limits::default();

        // Request device and queue.
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("zaroxi-engine-device"),
                    required_features,
                    required_limits,
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| RenderError::Other(format!("request_device failed: {:?}", e)))?;

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);

        // Prefer an sRGB format when available.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| matches!(f, TextureFormat::Bgra8UnormSrgb | TextureFormat::Rgba8UnormSrgb))
            .or_else(|| surface_caps.formats.get(0).copied())
            .unwrap_or(TextureFormat::Bgra8UnormSrgb);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 0u32,
        };

        surface.configure(&device, &config);

        info!("Renderer initialized ({}x{})", config.width, config.height);

        Ok(Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
            size,
            clear_color: Color {
                r: clear_color[0] as f32,
                g: clear_color[1] as f32,
                b: clear_color[2] as f32,
                a: clear_color[3] as f32,
            },
            window,
        })
    }

    /// Resize and reconfigure the surface.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) -> Result<(), RenderError> {
        if new_size.width == 0 || new_size.height == 0 {
            return Ok(());
        }

        self.size = new_size;
        self.config.width = new_size.width.max(1);
        self.config.height = new_size.height.max(1);
        self.surface.configure(&self.device, &self.config);
        debug!("Surface reconfigured to {}x{}", self.config.width, self.config.height);
        Ok(())
    }

    /// Reconfigure surface in case of Lost or Outdated.
    pub fn reconfigure(&mut self) -> Result<(), RenderError> {
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Request a redraw via the stored Arc<Window>.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Perform a single clear-pass render.
    /// Returns wgpu::SurfaceError so the runtime can handle surface-specific cases.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.config.width == 0 || self.config.height == 0 {
            // Skip rendering when surface has zero area.
            return Ok(());
        }

        // Acquire next texture; propagate SurfaceError to the caller for handling.
        let surface_texture = self.surface.get_current_texture()?;
        // Create a texture view
        let view = surface_texture
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("clear-encoder"),
            });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.clear_color),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
        }

        self.queue.submit(Some(encoder.finish()));
        // Present the frame.
        surface_texture.present();

        Ok(())
    }
}
