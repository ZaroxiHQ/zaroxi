#![deny(missing_docs)]
/*!
Minimal wgpu-based render backend that presents a solid background color
to a winit window. The API accepts a vello::Scene (phase 2 will wire vello
rendering); for now the backend clears the surface color each frame which
keeps visible progress fast and avoids broad refactors.

Responsibilities:
- Initialize wgpu Device / Queue / Surface
- Configure the surface for the window size
- Provide resize handling and a render_frame(scene) entry that presents a frame
*/

use wgpu::{CommandEncoderDescriptor, PresentMode, TextureUsages};
use zaroxi_core_engine_window::ZaroxiWindow;

/// Simple render backend that drives a wgpu surface and presents frames.
///
/// The backend stores a surface tied to the window lifetime; the `'a` lifetime
/// parameter represents the borrow of the native window used to create the
/// surface.
pub struct RenderBackend<'a> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'a>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> RenderBackend<'a> {
    /// Create a new RenderBackend for the supplied window.
    ///
    /// This is async because wgpu adapter / device requests are async.
    pub async fn new(window: &'a ZaroxiWindow) -> Self {
        // Create instance and surface using the v29 InstanceDescriptor API.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // create_surface returns a Result in this wgpu version; unwrap to get the Surface.
        let surface = unsafe { instance.create_surface(window.window()) }
            .expect("failed to create wgpu surface");

        // Choose a high-performance adapter when available and prefer a surface-compatible adapter.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("failed to request wgpu adapter");

        // Request device with conservative, sane limits. Adapter::request_device returns a Result<(Device, Queue), _>.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("zaroxi-render-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("failed to request wgpu device");

        // Choose surface format: prefer Bgra8UnormSrgb if available.
        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8UnormSrgb)
            .unwrap_or(caps.formats[0]);

        let (width, height) = window.size();
        let width = width.max(1);
        let height = height.max(1);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: PresentMode::Fifo, // V-sync; stable and widely supported
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            // optional latency target — keep None for now
            desired_maximum_frame_latency: None,
        };

        surface.configure(&device, &surface_config);

        Self {
            device,
            queue,
            surface,
            surface_config,
            surface_format,
            _marker: std::marker::PhantomData,
        }
    }

    /// Resize the underlying surface configuration to the new size.
    /// Zero sizes are ignored (do not attempt to reconfigure to zero).
    pub fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);

        if self.surface_config.width == w && self.surface_config.height == h {
            return;
        }

        self.surface_config.width = w;
        self.surface_config.height = h;
        self.surface
            .configure(&self.device, &self.surface_config);
    }

    /// Render a single frame. The provided vello::Scene is currently unused
    /// by this phase; the backend performs a full-surface clear to the chosen
    /// background color and presents the frame.
    pub fn render_frame(&mut self, _scene: &vello::Scene) {
        // Background color: rgba(13,14,17,255)
        let bg_color = wgpu::Color {
            r: 13.0 / 255.0,
            g: 14.0 / 255.0,
            b: 17.0 / 255.0,
            a: 1.0,
        };

        // Acquire next surface texture.
        match self.surface.get_current_texture() {
            Ok(surface_texture) => {
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = self
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("zaroxi-clear-encoder"),
                    });

                {
                    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("zaroxi-clear-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(bg_color),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        multiview_mask: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                    // drop _rpass to finish the pass
                }

                self.queue.submit(Some(encoder.finish()));
                surface_texture.present();
            }
            Err(err) => {
                // Handle surface errors gracefully. Do not panic on transient issues.
                match err {
                    wgpu::SurfaceError::Lost => {
                        // Recreate swap chain
                        eprintln!("wgpu surface lost; reconfiguring");
                        self.surface
                            .configure(&self.device, &self.surface_config);
                    }
                    wgpu::SurfaceError::OutOfMemory => {
                        // OutOfMemory is fatal for the application.
                        eprintln!("wgpu surface out of memory; exiting: {:?}", err);
                        // Let caller decide; for now we attempt to continue.
                    }
                    wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Outdated => {
                        // Transient; skip this frame.
                        eprintln!("wgpu surface transient error, skipping frame: {:?}", err);
                    }
                }
            }
        }
    }
}
