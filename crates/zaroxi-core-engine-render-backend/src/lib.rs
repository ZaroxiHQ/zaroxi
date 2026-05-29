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

use bytemuck;
use wgpu::{CommandEncoderDescriptor, PresentMode, TextureUsages, util::DeviceExt};
use zaroxi_core_engine_font::load_bundled_monospace;
use zaroxi_core_engine_window::ZaroxiWindow;

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
    pub pipeline: Option<wgpu::RenderPipeline>,
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
        let surface =
            instance.create_surface(window.window()).expect("failed to create wgpu surface");

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

        // --- One-shot clear+present helper (async) ---
        // This helper lets interface-desktop request a minimal clear+present for a newly
        // created window without permanently owning a RenderBackend instance. It's
        // intentionally conservative and used only for GUI-3 first-frame proof.
        //
        // Usage:
        // pollster::block_on(RenderBackend::clear_present_once(&zaroxi_window, wgpu::Color { r,g,b,a }));
        impl<'a> RenderBackend<'a> {
            /// Clear and present a single frame to the supplied window using the
            /// backend's initialization path. Returns Ok(()) on success or an Err
            /// with a boxed error on failure.
            pub async fn clear_present_once(
                window: &'a zaroxi_core_engine_window::ZaroxiWindow,
                color: wgpu::Color,
            ) -> Result<(), Box<dyn std::error::Error>> {
                // Create a temporary backend (async init).
                let backend = Self::new(window).await;

                // Acquire next surface texture.
                let surface_texture = match backend.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(tex) => tex,
                    wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                        eprintln!("wgpu surface acquired suboptimal texture; proceeding");
                        tex
                    }
                    other => {
                        eprintln!("wgpu surface acquisition returned {:?}; aborting clear_present_once", other);
                        backend.surface.configure(&backend.device, &backend.surface_config);
                        return Err("surface acquisition failed".into());
                    }
                };

                let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = backend.device.create_command_encoder(&CommandEncoderDescriptor {
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
                                load: wgpu::LoadOp::Clear(color),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        multiview_mask: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                    // no draw calls for one-shot clear
                }

                backend.queue.submit(Some(encoder.finish()));
                surface_texture.present();

                Ok(())
            }
        }

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
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Render a single frame. The provided vello::Scene is currently unused
    /// by this phase; the backend performs a full-surface clear to the chosen
    /// background color and presents the frame.
    pub fn render_frame(&mut self, _scene: &vello::Scene) {
        // Background color: rgba(13,14,17,255)
        let bg_color = wgpu::Color { r: 13.0 / 255.0, g: 14.0 / 255.0, b: 17.0 / 255.0, a: 1.0 };

        // Acquire next surface texture. The local wgpu API returns a `CurrentSurfaceTexture`
        // enum. Handle Success and Suboptimal as valid textures; treat other variants as errors.
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                eprintln!(
                    "wgpu surface acquired suboptimal texture; proceeding but consider reconfigure"
                );
                tex
            }
            other => {
                eprintln!(
                    "wgpu surface acquisition returned {:?}; reconfiguring/skip frame",
                    other
                );
                // Reconfigure the surface for the next frame. Do not use catch_unwind here;
                // wgpu internals are not guaranteed UnwindSafe and calling catch_unwind
                // causes hard-to-resolve trait errors. If configure panics it will propagate.
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
        };

        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

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

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("zaroxi-draw-encoder"),
        });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

        // --- Overlay: simple editor text surface (Phase 4 demo) ---
        // Build a minimal EditorPrimitiveSet using the engine text layout and
        // the shell layout so the backend can render a real text surface inside
        // the editor content area. This is intentionally small and deterministic:
        // - uses the bundled monospace font metrics
        // - layouts a tiny sample buffer (replaceable by real buffer plumbing)
        // - emits text runs, gutter labels, and a demo caret
        //
        // NOTE: This double-presents (background first, then editor overlay).
        // It's a conservative, low-risk integration point for Phase 4 so we can
        // incrementally wire real editor primitives without refactoring the
        // broader render loop. Future phases should integrate these primitives
        // earlier in the single-pass render_frame path.
        {
            // Sample lines (placeholder for real buffer content from editor state).
            let sample_lines = vec![
                "fn main() {".to_string(),
                "    println!(\"Hello, Zaroxi!\");".to_string(),
                "}".to_string(),
            ];

            // Load bundled monospace font metrics used by engine text layout.
            let font = load_bundled_monospace();
            let char_w: u32 = font.char_width;
            let line_h: u32 = font.line_height;

            // Compute editor content origin from the deterministic ShellLayout.
            let layout = zaroxi_core_engine_layout::ShellLayout::from_window_size(width, height);
            // editor_content is an absolute window-space rect
            let editor_x = layout.editor_content.x.max(0.0) as u32;
            let editor_y = layout.editor_content.y.max(0.0) as u32;

            // Use the engine text layout to shape visible lines into TextPrimitive items.
            let line_layout = zaroxi_core_engine_text::plain::layout_plain_lines(
                &sample_lines,
                &font,
                editor_x,
                editor_y,
                None,
            );

            // Convert text primitives into an EditorPrimitiveSet for renderer consumption.
            let mut set = zaroxi_core_engine_scene::EditorPrimitiveSet::new();

            // Text runs
            for tp in line_layout.primitives.into_iter() {
                set.texts.push(zaroxi_core_engine_scene::TextPrimitive {
                    x: tp.x,
                    y: tp.y,
                    text: tp.text,
                    font_name: tp.font_name,
                    max_width: tp.max_width,
                });
            }

            // Gutter labels (right-aligned numeric labels, deterministic width)
            let gutter_width: u32 = 48;
            let gutter_x = if editor_x > gutter_width { editor_x - gutter_width } else { 0 };
            for (i, _) in sample_lines.iter().enumerate() {
                let doc_row = 1u32.saturating_add(i as u32);
                let y = editor_y.saturating_add((i as u32).saturating_mul(line_h));
                set.gutter_labels.push(zaroxi_core_engine_scene::TextPrimitive {
                    x: gutter_x,
                    y,
                    text: format!("{:>4}", doc_row),
                    font_name: font.family.clone(),
                    max_width: None,
                });
            }

            // Demo caret (place at line 2, column 4). In Phase 4 this will be driven
            // by real editor state: cursor_line / cursor_column.
            let caret_line = 2u32;
            let caret_col = 4u32;
            let content_text_x = editor_x.saturating_add(6);
            let caret_x = content_text_x.saturating_add(caret_col.saturating_mul(char_w));
            let caret_y =
                editor_y.saturating_add((caret_line.saturating_sub(1)).saturating_mul(line_h));
            set.carets.push(zaroxi_core_engine_scene::CaretItem {
                x: caret_x,
                y: caret_y,
                height: line_h,
            });

            // Render the editor primitives as an overlay.
            self.render_editor_primitives(&set);
        }
    }

    /// Render editor primitives (text glyph boxes, caret, selections) as simple rectangles.
    ///
    /// This method provides a minimal, deterministic editor overlay rendering
    /// using the existing rectangle pipeline. It intentionally renders glyph
    /// runs as monospace boxes (no shaping) using the bundled monospace metrics.
    pub fn render_editor_primitives(
        &mut self,
        primitives: &zaroxi_core_engine_scene::EditorPrimitiveSet,
    ) {
        // Acquire next surface texture.
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                eprintln!(
                    "wgpu surface acquired suboptimal texture; proceeding but consider reconfigure"
                );
                tex
            }
            other => {
                eprintln!("wgpu surface acquisition returned {:?}; skip editor primitives", other);
                // Reconfigure for safety and return.
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
        };

        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

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

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
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

        // --- Overlay: engine-driven editor text surface (Phase 4) ---
        // Query the engine scene model (published by the harness/interface)
        // and convert it into EditorPrimitiveSet for rendering. This replaces
        // the prior hard-coded sample_lines and makes the backend reflect live
        // editor state when the application publishes it.
        {
            let scene_model = zaroxi_core_engine_scene::get_current_scene();

            // Load bundled monospace font metrics used by engine text layout.
            let font = load_bundled_monospace();
            let char_w: u32 = font.char_width;
            let line_h: u32 = font.line_height;

            // Compute editor content origin from the deterministic ShellLayout.
            let layout = zaroxi_core_engine_layout::ShellLayout::from_window_size(
                width as u32,
                height as u32,
            );
            let editor_x = layout.editor_content.x.max(0.0) as u32;
            let editor_y = layout.editor_content.y.max(0.0) as u32;

            // Determine visible slice based on scene.viewport_top_line and the
            // available editor content height.
            let avail_h = layout.editor_content.height.max(0.0) as u32;
            let visible_rows =
                if line_h > 0 { (avail_h / line_h) as usize } else { scene_model.text_lines.len() };

            let top_line = scene_model.viewport_top_line.max(1) as usize;
            let start_idx = top_line.saturating_sub(1);
            let end_idx = std::cmp::min(start_idx + visible_rows, scene_model.text_lines.len());
            let visible_slice: Vec<String> = scene_model.text_lines[start_idx..end_idx].to_vec();

            // Use the engine text layout to shape visible lines into TextPrimitive items.
            // We pass the content inset (6px) so text runs and caret math align with presenter expectations.
            let content_inset: u32 = 6;
            let content_text_x = editor_x.saturating_add(content_inset);
            let line_layout = zaroxi_core_engine_text::plain::layout_plain_lines(
                &visible_slice,
                &font,
                content_text_x,
                editor_y,
                None,
            );

            // Convert text primitives into an EditorPrimitiveSet for renderer consumption.
            let mut set = zaroxi_core_engine_scene::EditorPrimitiveSet::new();

            // Text runs (already positioned at content_text_x by layout_plain_lines)
            for tp in line_layout.primitives.into_iter() {
                set.texts.push(zaroxi_core_engine_scene::TextPrimitive {
                    x: tp.x,
                    y: tp.y,
                    text: tp.text,
                    font_name: tp.font_name,
                    max_width: tp.max_width,
                });
            }

            // Gutter labels (right-aligned numeric labels, deterministic width)
            let gutter_width: u32 = 48;
            let gutter_x = if editor_x > gutter_width { editor_x - gutter_width } else { 0 };
            for (i, _) in visible_slice.iter().enumerate() {
                let doc_row = (top_line as u32).saturating_add(i as u32);
                let y = editor_y.saturating_add((i as u32).saturating_mul(line_h));
                set.gutter_labels.push(zaroxi_core_engine_scene::TextPrimitive {
                    x: gutter_x,
                    y,
                    text: format!("{:>4}", doc_row),
                    font_name: font.family.clone(),
                    max_width: None,
                });
            }

            // Caret: project from live scene cursor_line / cursor_column into absolute coords
            if let Some(cl) = scene_model.cursor_line {
                let col = scene_model.cursor_column.unwrap_or(0);
                // Check if caret is inside the visible slice
                if cl >= scene_model.viewport_top_line
                    && (cl as usize) < (start_idx + visible_rows + 1)
                {
                    let offset_rows = (cl as usize).saturating_sub(top_line);
                    let caret_x = content_text_x.saturating_add(col.saturating_mul(char_w));
                    let caret_y =
                        editor_y.saturating_add((offset_rows as u32).saturating_mul(line_h));
                    set.carets.push(zaroxi_core_engine_scene::CaretItem {
                        x: caret_x,
                        y: caret_y,
                        height: line_h,
                    });
                }
            }

            // Simple selection rendering is intentionally omitted here unless the scene
            // exposes explicit selection ranges. The scene currently offers only a
            // `selection_present` flag; real selection rects will be produced by the
            // presenter or application in a later phase.

            // Render the editor primitives as an overlay.
            self.render_editor_primitives(&set);
        }
    }
}
