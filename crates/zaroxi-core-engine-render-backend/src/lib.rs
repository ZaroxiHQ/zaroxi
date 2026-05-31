#![deny(missing_docs)]
#![allow(non_local_definitions)]
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
use std::sync::atomic::{AtomicUsize, Ordering};
use wgpu::{CommandEncoderDescriptor, PresentMode, TextureUsages, util::DeviceExt};
use zaroxi_core_engine_window::ZaroxiWindow;

static GUI_TEXT_FALLBACK_COUNTER: AtomicUsize = AtomicUsize::new(0);

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

/// Small, low-level rectangle spec consumed by one-shot draw helpers.
/// This type is intentionally minimal and public so callers can construct
/// resolved, theme-fed rects to pass into the backend.
#[derive(Clone, Copy)]
pub struct DrawRect {
    /// X origin in window coordinates (pixels).
    pub x: u32,
    /// Y origin in window coordinates (pixels).
    pub y: u32,
    /// Rectangle width in pixels.
    pub width: u32,
    /// Rectangle height in pixels.
    pub height: u32,
    /// Fill color as an sRGBA color (wgpu::Color).
    pub color: wgpu::Color,
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

        // For GUI-4 we create a tiny, safe colored-rect pipeline during init.
        // The pipeline is minimal: a WGSL shader that accepts per-vertex position
        // and color attributes. This lets the one-shot helper draw a small set of
        // resolved rectangles (background overlay bands) without pulling heavyweight
        // rendering logic into the interface layer.
        //
        // Pipeline creation is conservative: if any step fails we log and continue
        // with `pipeline: None` so the historic clear-only path remains available.
        //
        // --- One-shot clear+present helper (async) ---
        // This helper lets interface-desktop request a minimal clear+present for a newly
        // created window without permanently owning a RenderBackend instance.
        //
        // Usage:
        // pollster::block_on(RenderBackend::clear_present_once(&zaroxi_window, wgpu::Color { r,g,b,a }, None));
        impl<'a> RenderBackend<'a> {
            /// Clear and present a single frame to the supplied window using the
            /// backend's initialization path. Accepts an optional slice of `DrawRect`
            /// describing rectangles (absolute window coords + color) to draw on top
            /// of the cleared background. This keeps the backend API low-level: the
            /// interface layer resolves regions and theme colors and passes concrete
            /// draw inputs here.
            pub async fn clear_present_once(
                window: &'a zaroxi_core_engine_window::ZaroxiWindow,
                color: wgpu::Color,
                overlay_rects: Option<&[DrawRect]>,
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
                        eprintln!(
                            "wgpu surface acquisition returned {:?}; aborting clear_present_once",
                            other
                        );
                        backend.surface.configure(&backend.device, &backend.surface_config);
                        return Err("surface acquisition failed".into());
                    }
                };

                let view =
                    surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Primary encoder for the one-shot pass.
                let mut encoder =
                    backend.device.create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("zaroxi-clear-encoder"),
                    });

                // Initial full-surface clear to the requested background color.
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
                    // initial clear; we'll draw overlay rects below if requested
                }

                // If overlay rects requested and we have a pipeline, draw them as colored quads.
                if let Some(rects) = overlay_rects {
                    if !rects.is_empty() {
                        // Runtime trace: detect whether the interface layer recorded a text layout
                        // marker for the canonical label ("Zaroxi") and whether the Cosmic prepare
                        // marker exists. We use temp-file markers created by the interface and
                        // renderer crates to avoid introducing cross-crate type dependencies.
                        let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
                        let tmp_cosmic =
                            std::env::temp_dir().join("zaroxi_gui_trace_cosmic_prepare");
                        let layout_present = tmp_layout.exists();
                        let cosmic_present = tmp_cosmic.exists();
                        eprintln!(
                            "GUI_SHELL_TRACE: clear_present_once overlay_rects_count={} layout_present={} cosmic_prepare_present={}",
                            rects.len(),
                            layout_present,
                            cosmic_present
                        );

                        // Additional backend-level adapter tracing: try to extract adapter-side label count
                        // from the adapter marker file written by the interface (if present).
                        let mut adapter_text_ops: usize = 0;
                        if layout_present {
                            if let Ok(s) = std::fs::read_to_string(&tmp_layout) {
                                if let Some(rest) = s.strip_prefix("lines=") {
                                    adapter_text_ops =
                                        rest.split(" | ").filter(|p| !p.is_empty()).count();
                                }
                            }
                        }
                        let overlay_rects_count = rects.len();
                        let forwarded = layout_present && overlay_rects_count > 0;
                        eprintln!(
                            "GUI_TEXT_STAGE_2_BACKEND: adapter_text_ops={} overlay_rects={} forwarded={}",
                            adapter_text_ops, overlay_rects_count, forwarded
                        );

                        // If the interface produced layout for "Zaroxi" but the Cosmic prepare
                        // marker has not been observed, we are likely still on the overlay
                        // rectangle fallback path.
                        //
                        // DO NOT abort startup: instead emit a loud diagnostic and a per-frame
                        // fallback counter so developers can observe the mismatch without
                        // killing the process. Also write a small temp marker so other tools
                        // can correlate the fallback activity.
                        if layout_present && !cosmic_present {
                            let cnt = GUI_TEXT_FALLBACK_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
                            eprintln!(
                                "GUI_TEXT_STAGE_2_BACKEND: detected layout for 'Zaroxi' but no cosmic prepare; fallback_count={} overlay_rects_count={}",
                                cnt,
                                rects.len()
                            );
                            // write a small marker to temp so other processes/crates can observe fallback counts.
                            let _ = std::fs::write(
                                std::env::temp_dir().join("zaroxi_gui_trace_fallback"),
                                format!("count={}\nrects={}\n", cnt, rects.len()),
                            );
                            // Continue with overlay rects; do not return/abort startup so we can observe multiple frames.
                        }

                        // Build vertex list for rects (two triangles per rect).
                        let mut vertices: Vec<Vertex> = Vec::new();
                        for r in rects.iter() {
                            let left = r.x as f32;
                            let top = r.y as f32;
                            let right = left + r.width as f32;
                            let bottom = top + r.height as f32;

                            let to_ndc = |px: f32, py: f32| -> [f32; 2] {
                                let nx = (px / (backend.surface_config.width as f32)) * 2.0 - 1.0;
                                let ny = 1.0 - (py / (backend.surface_config.height as f32)) * 2.0;
                                [nx, ny]
                            };

                            let tl = to_ndc(left, top);
                            let tr = to_ndc(right, top);
                            let br = to_ndc(right, bottom);
                            let bl = to_ndc(left, bottom);

                            let clr = [
                                r.color.r as f32,
                                r.color.g as f32,
                                r.color.b as f32,
                                r.color.a as f32,
                            ];

                            vertices.push(Vertex { position: tl, color: clr });
                            vertices.push(Vertex { position: tr, color: clr });
                            vertices.push(Vertex { position: br, color: clr });

                            vertices.push(Vertex { position: tl, color: clr });
                            vertices.push(Vertex { position: br, color: clr });
                            vertices.push(Vertex { position: bl, color: clr });
                        }

                        if let Some(pipeline_ref) = &backend.pipeline {
                            // Create vertex buffer and draw using the pipeline.
                            let vertex_buffer = backend.device.create_buffer_init(
                                &wgpu::util::BufferInitDescriptor {
                                    label: Some("zaroxi-clear-rect-verts"),
                                    contents: bytemuck::cast_slice(&vertices),
                                    usage: wgpu::BufferUsages::VERTEX,
                                },
                            );

                            {
                                let mut rpass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("zaroxi-clear-overlay-pass"),
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                depth_slice: None,
                                                ops: wgpu::Operations {
                                                    // Load preserves the background clear we just did.
                                                    load: wgpu::LoadOp::Load,
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        depth_stencil_attachment: None,
                                        multiview_mask: None,
                                        occlusion_query_set: None,
                                        timestamp_writes: None,
                                    });

                                rpass.set_pipeline(pipeline_ref);
                                rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                                let vert_count = vertices.len() as u32;
                                if vert_count > 0 {
                                    rpass.draw(0..vert_count, 0..1);
                                }
                            }
                        } else {
                            eprintln!(
                                "clear_present_once: pipeline missing; skipping overlay rect draws"
                            );
                        }
                    }
                }

                backend.queue.submit(Some(encoder.finish()));
                surface_texture.present();

                Ok(())
            }
        }

        // Attempt to create a minimal colored-rect pipeline compatible with the
        // workspace wgpu version. This is intentionally small: a single WGSL shader
        // with a vertex stage that accepts position/color and a fragment stage that
        // outputs the interpolated color.
        //
        // Creation is wrapped so any failure yields `None` and preserves the
        // historic clear-only fallback.
        let pipeline = (|| {
            // WGSL: per-vertex position in NDC and color
            // Load WGSL shader from a dedicated file to keep the Rust source concise.
            // The shader lives at `src/shaders/rect.wgsl` relative to this file.
            let shader_src = include_str!("shaders/rect.wgsl");

            // Create shader module
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("zaroxi-rect-shader"),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });

            // Pipeline layout: no bind groups for this minimal pipeline
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("zaroxi-rect-pipeline-layout"),
                bind_group_layouts: &[],
                ..Default::default()
            });

            // Vertex buffer layout: [f32;2] position, [f32;4] color
            let vertex_buffer_layout = wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
            };

            // Vertex / fragment state (match local wgpu API expectations)
            let vertex_state = wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[vertex_buffer_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            };

            let fragment_state = Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            });

            // Render pipeline descriptor
            let pipeline_desc = wgpu::RenderPipelineDescriptor {
                label: Some("zaroxi-rect-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: vertex_state,
                fragment: fragment_state,
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: Default::default(),
            };

            // Try to create the pipeline; if it errors (panic) we catch it using
            // AssertUnwindSafe and fall back to clear-only mode. `wgpu::Device`
            // contains interior mutability and is not `UnwindSafe`, so wrap the
            // closure in `AssertUnwindSafe` before calling `catch_unwind`.
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                device.create_render_pipeline(&pipeline_desc)
            })) {
                Ok(p) => Some(p),
                Err(_) => {
                    eprintln!(
                        "RenderBackend: pipeline creation panicked; falling back to clear-only mode"
                    );
                    None
                }
            }
        })();

        Self {
            device,
            queue,
            surface,
            surface_config,
            surface_format,
            pipeline,
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

        // Trace what the backend observes on each frame (adapter marker => adapter_text_ops).
        let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
        let mut adapter_text_ops: usize = 0;
        if tmp_layout.exists() {
            if let Ok(s) = std::fs::read_to_string(&tmp_layout) {
                if let Some(rest) = s.strip_prefix("lines=") {
                    adapter_text_ops = rest.split(" | ").filter(|p| !p.is_empty()).count();
                }
            }
        }
        let backend_text_ops = ui_rects.len();
        let forwarded = adapter_text_ops > 0 && backend_text_ops > 0;
        eprintln!(
            "GUI_TEXT_STAGE_2_BACKEND: adapter_text_ops={} backend_text_ops={} forwarded={}",
            adapter_text_ops, backend_text_ops, forwarded
        );

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
                if let Some(pipeline_ref) = &self.pipeline {
                    rpass.set_pipeline(pipeline_ref);
                    rpass.set_vertex_buffer(0, vb.slice(..));
                    // draw all vertices
                    let vert_count = vertices.len() as u32;
                    if vert_count > 0 {
                        rpass.draw(0..vert_count, 0..1);
                    }
                } else {
                    // No pipeline available (GUI-3 first-frame mode): skip rect draws.
                    eprintln!("RenderBackend: pipeline missing; skipping rect draws this frame");
                }
            }
            // rpass dropped here
        }

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();

        // Editor overlay intentionally disabled in the minimal backend.
        // The full renderer (zaroxi-core-engine-render with the text pipeline)
        // is responsible for shaped glyph rendering. Drawing opaque monospace
        // glyph boxes here previously obscured actual glyph output produced by
        // the real text pipeline. Keep this area empty so the canonical pipeline
        // can present readable glyphs.
    }

    /// Render editor primitives.
    ///
    /// The minimal backend intentionally no-ops here. Shaped glyph rendering is
    /// performed by the full renderer (zaroxi-core-engine-render) via the
    /// canonical text pipeline. Keeping this method as a documented no-op
    /// satisfies the crate-level `deny(missing_docs)` lint while preserving the
    /// intended architecture where the full renderer owns glyph rasterization.
    pub fn render_editor_primitives(
        &mut self,
        _primitives: &zaroxi_core_engine_scene::EditorPrimitiveSet,
    ) {
        // No-op: editor primitives rendering disabled in the minimal backend.
        // The full renderer (zaroxi-core-engine-render) is responsible for shaped
        // glyph rendering via the canonical text pipeline (Cosmic Text integration).
    }
}
