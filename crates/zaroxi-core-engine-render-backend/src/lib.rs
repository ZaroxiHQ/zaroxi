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

use wgpu::{CommandEncoderDescriptor, PresentMode, TextureUsages, util::DeviceExt};
use zaroxi_core_engine_window::ZaroxiWindow;
use zaroxi_core_engine_layout::layout::ShellLayout;
use bytemuck;
use zaroxi_core_engine_font::load_bundled_monospace;

/// Simple render backend that drives a wgpu surface and presents frames.
///
/// The backend stores a surface tied to the window lifetime; the `'a` lifetime
/// parameter represents the borrow of the native window used to create the
/// surface.
pub struct RenderBackend<'a> {
    /// The GPU device used to create GPU resources and encode work.
    pub device: wgpu::Device,
    /// The submission queue associated with `device` used for command submission.
    pub queue: wgpu::Queue,
    /// The presentation surface obtained from the window.
    pub surface: wgpu::Surface<'a>,
    /// Current configuration for the surface (format, size, present mode, etc.).
    pub surface_config: wgpu::SurfaceConfiguration,
    /// Chosen texture format for surface presentation.
    pub surface_format: wgpu::TextureFormat,
    /// Simple pipeline used to render solid rectangles for the shell regions.
    pub pipeline: wgpu::RenderPipeline,
    _marker: std::marker::PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl<'a> RenderBackend<'a> {
    /// Create a new RenderBackend for the supplied window.
    ///
    /// This is async because wgpu adapter / device requests are async.
    pub async fn new(window: &'a ZaroxiWindow) -> Self {
        // Create instance using the wgpu default constructor for the resolved local API.
        // This avoids constructing InstanceDescriptor by hand and matches the local wgpu.
        let instance = wgpu::Instance::default();

        // create_surface returns a Result in this wgpu version; unwrap to get the Surface.
        let surface = instance
            .create_surface(window.window())
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
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("zaroxi-render-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
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
            // Use 0 as unspecified / default latency target for this backend.
            desired_maximum_frame_latency: 0,
        };

        surface.configure(&device, &surface_config);

        // Create a minimal shader/pipeline used to render solid-colored rectangles
        // that visualize the ShellLayout regions. This keeps Phase 4 rendering local
        // to the backend and avoids touching broader presenter APIs.
        let shader_src = r#"
struct VertexInput {
    @location(0) position: vec2<f32>;
    @location(1) color: vec4<f32>;
};
struct VertexOutput {
    @builtin(position) pos: vec4<f32>;
    @location(0) color: vec4<f32>;
};
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    return out;
}
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("solid-rect-shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        // Use implicit pipeline layout (None) so we avoid tying into pipeline-layout
        // descriptor fields that vary across wgpu versions. The render pipeline below
        // will be created with layout: None.

        let vertex_size = std::mem::size_of::<Vertex>() as wgpu::BufferAddress;
        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: vertex_size,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("solid-rect-pipeline"),
            // Use implicit layout to maintain compatibility across wgpu versions.
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            device,
            queue,
            surface,
            surface_config,
            surface_format,
            pipeline: render_pipeline,
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

        // Acquire next surface texture. The local wgpu API returns a `CurrentSurfaceTexture`
        // enum. Handle Success and Suboptimal as valid textures; treat other variants as errors.
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                eprintln!("wgpu surface acquired suboptimal texture; proceeding but consider reconfigure");
                tex
            }
            other => {
                eprintln!("wgpu surface acquisition returned {:?}; reconfiguring/skip frame", other);
                // Reconfigure the surface for the next frame. Do not use catch_unwind here;
                // wgpu internals are not guaranteed UnwindSafe and calling catch_unwind
                // causes hard-to-resolve trait errors. If configure panics it will propagate.
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build UI via the engine UI composer and convert to vertices.
        let width = self.surface_config.width;
        let height = self.surface_config.height;
        let ui_rects = zaroxi_core_engine_ui::composer::build_shell_ui(width, height);

        // Helper to convert rect coordinates -> two triangles (6 vertices)
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut add_rect_from = |x: f32, y: f32, w: f32, h: f32, color: [f32; 4]| {
            let left = x;
            let top = y;
            let right = x + w;
            let bottom = y + h;

            let to_ndc = |px: f32, py: f32| -> [f32; 2] {
                let nx = (px / (width as f32)) * 2.0 - 1.0;
                let ny = 1.0 - (py / (height as f32)) * 2.0;
                [nx, ny]
            };

            let tl = to_ndc(left, top);
            let tr = to_ndc(right, top);
            let br = to_ndc(right, bottom);
            let bl = to_ndc(left, bottom);

            vertices.push(Vertex { position: tl, color });
            vertices.push(Vertex { position: tr, color });
            vertices.push(Vertex { position: br, color });

            vertices.push(Vertex { position: tl, color });
            vertices.push(Vertex { position: br, color });
            vertices.push(Vertex { position: bl, color });
        };

        // Panel rectangles from the UI composer (stable deterministic order).
        for r in ui_rects {
            add_rect_from(r.x, r.y, r.width, r.height, r.color);
        }

        // Create a transient vertex buffer for this frame (small, recreated each frame).
        let vertex_buffer = if !vertices.is_empty() {
            Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("zaroxi-rect-verts"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }))
        } else {
            None
        };

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("zaroxi-draw-encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("zaroxi-root-pass"),
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

            if let Some(vb) = &vertex_buffer {
                rpass.set_pipeline(&self.pipeline);
                rpass.set_vertex_buffer(0, vb.slice(..));
                // draw all vertices
                let vert_count = vertices.len() as u32;
                if vert_count > 0 {
                    rpass.draw(0..vert_count, 0..1);
                }
            }
            // rpass dropped here
        }

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }

    /// Render editor primitives (text glyph boxes, caret, selections) as simple rectangles.
    ///
    /// This method provides a minimal, deterministic editor overlay rendering
    /// using the existing rectangle pipeline. It intentionally renders glyph
    /// runs as monospace boxes (no shaping) using the bundled monospace metrics.
    pub fn render_editor_primitives(&mut self, primitives: &zaroxi_core_engine_scene::EditorPrimitiveSet) {
        // Acquire next surface texture.
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                eprintln!("wgpu surface acquired suboptimal texture; proceeding but consider reconfigure");
                tex
            }
            other => {
                eprintln!("wgpu surface acquisition returned {:?}; skip editor primitives", other);
                // Reconfigure for safety and return.
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let width = self.surface_config.width as f32;
        let height = self.surface_config.height as f32;

        // Helper to convert rect coordinates -> two triangles (6 vertices)
        let mut vertices: Vec<Vertex> = Vec::new();
        let to_ndc = |px: f32, py: f32| -> [f32; 2] {
            let nx = (px / width) * 2.0 - 1.0;
            let ny = 1.0 - (py / height) * 2.0;
            [nx, ny]
        };

        let mut add_rect = |x: f32, y: f32, w: f32, h: f32, color: [f32; 4]| {
            let left = x;
            let top = y;
            let right = x + w;
            let bottom = y + h;

            let tl = to_ndc(left, top);
            let tr = to_ndc(right, top);
            let br = to_ndc(right, bottom);
            let bl = to_ndc(left, bottom);

            vertices.push(Vertex { position: tl, color });
            vertices.push(Vertex { position: tr, color });
            vertices.push(Vertex { position: br, color });

            vertices.push(Vertex { position: tl, color });
            vertices.push(Vertex { position: br, color });
            vertices.push(Vertex { position: bl, color });
        };

        // Measure text glyph width using bundled monospace metrics.
        let font = load_bundled_monospace();
        let char_w = font.char_width as f32;
        let line_h = font.line_height as f32;

        // Selections (semi-transparent overlay)
        for s in &primitives.selections {
            add_rect(s.x as f32, s.y as f32, s.width as f32, s.height as f32, [0.2, 0.4, 0.8, 0.4]);
        }

        // Carets (thin opaque rectangle)
        for c in &primitives.carets {
            add_rect(c.x as f32, c.y as f32, 2.0, c.height as f32, [1.0, 0.5, 0.0, 1.0]);
        }

        // Text runs (monospace glyph boxes as deterministic stand-ins)
        for t in &primitives.texts {
            let w = (t.text.chars().count() as f32) * char_w;
            add_rect(t.x as f32, t.y as f32, w.max(1.0), line_h, [1.0, 1.0, 1.0, 1.0]);
        }

        // Gutter labels (smaller, muted boxes)
        for g in &primitives.gutter_labels {
            let w = (g.text.chars().count() as f32) * (char_w * 0.8);
            add_rect(g.x as f32, g.y as f32, w.max(1.0), line_h, [0.8, 0.8, 0.8, 1.0]);
        }

        if vertices.is_empty() {
            surface_texture.present();
            return;
        }

        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("zaroxi-editor-verts"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("zaroxi-editor-encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("zaroxi-editor-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        // Load so we preserve the background/panels already drawn.
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            rpass.set_pipeline(&self.pipeline);
            rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
            let vert_count = vertices.len() as u32;
            if vert_count > 0 {
                rpass.draw(0..vert_count, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}
