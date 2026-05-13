use crate::error::RenderError;
use log::{debug, info};
use wgpu::{
    util::DeviceExt, Backends, Color, Device, DeviceDescriptor, Instance, LoadOp, Operations,
    RequestAdapterOptions, RenderPassColorAttachment, RenderPassDescriptor, Surface, SurfaceConfiguration,
    TextureUsages, TextureView, TextureFormat, PresentMode, Features, Limits, Queue, CommandEncoderDescriptor,
};
use winit::window::Window;
use winit::dpi::PhysicalSize;
use std::num::NonZeroU32;

/// GPU renderer owning the device, queue, and surface.
pub struct Renderer {
    instance: Instance,
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    clear_color: Color,
    window: Window,
}

impl Renderer {
    /// Initialize the GPU renderer asynchronously.
    pub async fn new(window: &Window, clear_color: [f64; 4]) -> Result<Self, RenderError> {
        // Create instance with Vulkan + Metal + DX12 + Browser GL disabled for native desktop.
        let instance = Instance::new(Backends::all());

        // SAFETY: winit guarantees the window handle is valid while the Window is alive.
        let surface = unsafe { instance.create_surface(window) }.map_err(|e| {
            RenderError::Other(format!("Failed to create surface: {:?}", e))
        })?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| RenderError::Other("No compatible GPU adapter found".to_string()))?;

        let required_features = Features::empty();
        let required_limits = Limits::downlevel_webgl2_defaults().using_resolution(1);

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("zaroxi-engine-device"),
                    features: required_features,
                    limits: required_limits,
                },
                None,
            )
            .await
            .map_err(RenderError::from)?;

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
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
        };

        surface.configure(&device, &config);

        info!("Renderer initialized ({}x{})", config.width, config.height);

        Ok(Self {
            instance,
            surface,
            device,
            queue,
            config,
            size,
            clear_color: Color {
                r: clear_color[0] as f64 as f32,
                g: clear_color[1] as f64 as f32,
                b: clear_color[2] as f64 as f32,
                a: clear_color[3] as f64 as f32,
            },
            window: window.clone(),
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

    /// Reconfigure surface in case of Lost.
    pub fn reconfigure(&mut self) -> Result<(), RenderError> {
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Request a redraw by calling Window.request_redraw.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Perform a single clear-pass render. Returns SurfaceError for caller handling.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = match self.surface.get_current_texture() {
            Ok(o) => o,
            Err(e) => return Err(e),
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("clear-encoder"),
        });

        {
            let _rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.clear_color),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
        } // rpass dropped here

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}
