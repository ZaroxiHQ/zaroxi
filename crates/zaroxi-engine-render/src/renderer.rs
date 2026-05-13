use crate::error::RenderError;
use log::{debug, info};
use std::marker::PhantomData;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use wgpu::{
    Backends, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, PresentMode, RequestAdapterOptions, Surface, SurfaceConfiguration,
    TextureFormat, TextureUsages, TextureViewDescriptor, Color, Queue, LoadOp, Operations, StoreOp,
    SurfaceTexture,
};

/// GPU renderer owning the device, queue, and surface.
///
/// This implementation targets wgpu 29.0.3 and uses an explicit lifetime so
/// the Surface can reference the window provided by the runtime. The runtime
/// keeps the Arc<Window> and passes a borrow into Renderer::new; the Surface
/// lifetime is therefore tied to the runtime-held window.
pub struct Renderer<'a> {
    instance: Instance,
    surface: Surface<'a>,
    adapter: wgpu::Adapter,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    clear_color: Color,
    _window_lifetime: PhantomData<&'a Window>,
}

impl<'a> Renderer<'a> {
    /// Create a new renderer. `window` must outlive the returned Renderer.
    pub async fn new(window: &'a Window, clear_color: [f64; 4]) -> Result<Self, RenderError> {
        // Build Instance
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            flags: wgpu::InstanceFlags::empty(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        // Create surface bound to the provided window reference.
        let surface = instance
            .create_surface(window)
            .map_err(|e| RenderError::Other(format!("create_surface failed: {:?}", e)))?;

        // request_adapter returns a Result in this workspace; map errors explicitly.
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| RenderError::Other(format!("request_adapter failed: {:?}", e)))?;

        // Minimal device requirements.
        let required_features = Features::empty();
        let required_limits = Limits::default();

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
        let caps = surface.get_capabilities(&adapter);

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
            _window_lifetime: PhantomData,
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

    /// Reconfigure surface after Lost/Outdated.
    pub fn reconfigure(&mut self) -> Result<(), RenderError> {
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Request redraw. The runtime owns the Window and should call this method
    /// with a Window reference (keeps renderer free of window ownership).
    pub fn request_redraw(&self, window: &Window) {
        window.request_redraw();
    }

    /// Render a single clear-pass frame.
    /// This uses the exact API shape present in this workspace: `get_current_texture()`
    /// returns a CurrentSurfaceTexture directly; we create a view, encode a clear pass,
    /// submit the queue and present the frame.
    pub fn render(&mut self) -> Result<(), RenderError> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }

        // Acquire the current surface texture (CurrentSurfaceTexture, not Result).
        let frame: SurfaceTexture = self.surface.get_current_texture();

        // Create a texture view for the render pass.
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        // Create encoder and record clear pass.
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
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
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
        }

        self.queue.submit(Some(encoder.finish()));
        // Present the frame (CurrentSurfaceTexture exposes present()).
        frame.present();

        Ok(())
    }
}
