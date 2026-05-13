use crate::error::RenderError;
use log::{debug, info, warn};
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use wgpu::{
    Backends, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, PresentMode, RequestAdapterOptions, Surface, SurfaceConfiguration,
    TextureFormat, TextureUsages, TextureViewDescriptor, Color, Queue, LoadOp, Operations, StoreOp,
};

/// GPU renderer owning the device, queue, and surface.
///
/// This implementation targets wgpu 29.0.3 and uses a simple, robust ownership
/// model: the runtime provides an Arc<Window> which the renderer keeps to be
/// able to request redraws without borrowing issues.
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
    /// Create a new renderer. The window is wrapped in Arc to avoid lifetime
    /// and ownership complications between winit and wgpu.
    pub async fn new(window: Arc<Window>, clear_color: [f64; 4]) -> Result<Self, RenderError> {
        // Construct instance for all native backends.
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            flags: wgpu::InstanceFlags::empty(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        // Create surface for this window.
        let surface = instance
            .create_surface(&*window)
            .map_err(|e| RenderError::Other(format!("create_surface failed: {:?}", e)))?;

        // Request an adapter compatible with the surface.
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| RenderError::Other("No compatible GPU adapter found".to_string()))?;

        // Minimal device requirements.
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

        // Surface / swapchain (configuration)
        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);

        // Choose a format (prefer sRGB when available).
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| matches!(f, TextureFormat::Bgra8UnormSrgb | TextureFormat::Rgba8UnormSrgb))
            .or_else(|| caps.formats.get(0).copied())
            .unwrap_or(TextureFormat::Bgra8UnormSrgb);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: Vec::new(),
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
                r: clear_color[0],
                g: clear_color[1],
                b: clear_color[2],
                a: clear_color[3],
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
        debug!("Reconfigured surface to {}x{}", self.config.width, self.config.height);
        Ok(())
    }

    /// Reconfigure the surface (used after Lost/Outdated).
    pub fn reconfigure(&mut self) -> Result<(), RenderError> {
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Request a redraw using the stored Window handle.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Render a single clear-pass frame.
    ///
    /// Returns wgpu::SurfaceError to allow the runtime to react appropriately.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }

        // Acquire the next surface frame. Propagate SurfaceError to caller.
        let frame = self.surface.get_current_texture()?;

        // Create a view for the texture.
        let view = frame
            .texture
            .create_view(&TextureViewDescriptor::default());

        // Create command encoder and record a clear pass.
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("zaroxi-clear-encoder"),
        });

        {
            let _rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        // Submit and present.
        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}
