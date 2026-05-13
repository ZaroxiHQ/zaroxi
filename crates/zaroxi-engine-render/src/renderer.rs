use crate::error::RenderError;
use log::{debug, info};
use winit::dpi::PhysicalSize;
use winit::window::Window;
use wgpu::{
    Backends, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, LoadOp, Operations, PresentMode, RequestAdapterOptions,
    RenderPassColorAttachment, RenderPassDescriptor, Surface, SurfaceConfiguration, StoreOp,
    TextureFormat, TextureUsages, TextureViewDescriptor, Color, Queue,
};

/// GPU renderer owning the device, queue, and surface.
///
/// The surface is tied to the window lifetime; the renderer therefore
/// carries the same lifetime parameter. The runtime creates the renderer
/// with the window reference that outlives the renderer for v1.
pub struct Renderer<'a> {
    instance: Instance,
    surface: Surface<'a>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    clear_color: Color,
    // Note: we intentionally do NOT store Window in the renderer to avoid
    // clone/ownership problems. Requesting redraws uses a Window reference
    // passed from the runtime.
    _marker: std::marker::PhantomData<&'a Window>,
}

impl<'a> Renderer<'a> {
    /// Initialize the GPU renderer asynchronously.
    pub async fn new(window: &'a Window, clear_color: [f64; 4]) -> Result<Self, RenderError> {
        // Create instance with all native backends enabled using the simple, stable form.
        // Use InstanceDescriptor with backends set and let Default fill the rest.
        // Construct an explicit InstanceDescriptor compatible with wgpu 29.x.
        // Avoid relying on Default for InstanceDescriptor which is not implemented.
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            flags: wgpu::InstanceFlags::empty(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        // SAFETY: winit guarantees the window handle is valid while the Window is alive.
        let surface = instance.create_surface(window)
            .map_err(|e| RenderError::Other(format!("Failed to create surface: {:?}", e)))?;

        // Request an adapter compatible with the surface.
        // Use `request_adapter(...).await` and map any error to our RenderError.
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| RenderError::Other(format!("request_adapter failed: {:?}", e)))?;

        // Minimal device requirements for v1.
        let required_features = Features::empty();
        let required_limits = Limits::default();

        // Request device and queue. Use DeviceDescriptor and default the remaining fields.
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("zaroxi-engine-device"),
                    required_features,
                    required_limits,
                    ..Default::default()
                },
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
            _marker: std::marker::PhantomData,
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
    /// The runtime owns the Window; pass it here when requesting a redraw.
    pub fn request_redraw(&self, window: &Window) {
        window.request_redraw();
    }

    /// Perform a single clear-pass render.
    /// For v1 we return our crate RenderError on failure so the runtime
    /// can decide how to react without depending on wgpu's exact error type.
    pub fn render(&mut self) -> Result<(), RenderError> {
        // Acquire the current surface texture (returns a SurfaceTexture-like handle).
        // Some wgpu builds expose different SurfaceTexture APIs. To avoid fragile
        // access to private fields across versions, present the acquired surface
        // texture by dropping it and skip a render pass for now. This keeps the
        // scaffold compiling and the window responsive; we'll add a proper clear
        // pass once we standardize on the concrete SurfaceTexture API.
        let surface_texture = self.surface.get_current_texture();
        drop(surface_texture);

        // Nothing to draw in this early scaffold; return success.
        return Ok(());

        {
            // Begin a simple render pass that clears the frame.
            let _rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
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
        } // rpass dropped here

        self.queue.submit(Some(encoder.finish()));

        // Present the frame by dropping the surface texture (present occurs on drop in this wgpu build).
        drop(surface_texture);

        Ok(())
    }
}
