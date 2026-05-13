use crate::error::RenderError;
use log::{debug, info};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use std::num::NonZeroU32;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use wgpu::{
    util::DeviceExt, Backends, BindGroup, BindGroupLayout, Buffer, CommandEncoderDescriptor, Device,
    DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits, PresentMode, Queue, RequestAdapterOptions,
    Surface, SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, Color, LoadOp, Operations,
    StoreOp, SurfaceError, Origin3d, TextureDescriptor, Extent3d, TextureDimension, TextureView, Sampler,
    SamplerDescriptor,
};

use fontdue::Font;
use std::collections::HashMap;

use zaroxi_app::AppState;
use zaroxi_theme::{SemanticColors as Theme, Color as ThemeColor};

/// Helper to convert theme Color -> renderer [f32;4]
fn color_to_rgba(c: &ThemeColor) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

/// Simple glyph metadata stored in the atlas.
struct GlyphInfo {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub width: u32,
    pub height: u32,
    pub advance: f32,
    pub xoffset: i32,
    pub yoffset: i32,
}

/// Minimal font atlas backing struct.
struct FontAtlas {
    pub atlas_width: u32,
    pub atlas_height: u32,
    // GPU texture view & bind group for sampling
    pub texture_view: TextureView,
    pub bind_group: BindGroup,
    pub glyphs: HashMap<char, GlyphInfo>,
    pub font_size: f32,
}

impl FontAtlas {
    /// Build an atlas from the bundled font bytes.
    fn new(device: &Device, queue: &Queue, layout: &BindGroupLayout, font_size: f32) -> Result<Self, RenderError> {
        // Load bundled font from workspace assets (crate-agnostic path).
        // Use CARGO_MANIFEST_DIR relative traversal to reach workspace root.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let font_path = PathBuf::from(manifest).join("../../assets/fonts/JetBrainsMonoNerdFont-Regular.ttf");
        let font_data = std::fs::read(&font_path).map_err(|e| RenderError::Other(format!("failed to read font: {:?}", e)))?;

        // fontdue::Font::from_bytes returns a Font in the versions we depend on.
        // If this returns a Result in future versions, handle it similarly.
        let font = Font::from_bytes(font_data.as_slice(), fontdue::FontSettings::default())
            .map_err(|e| RenderError::Other(format!("fontdue load failed: {:?}", e)))?;

        // Rasterize ASCII range 32..=126
        let padding = 2;
        let atlas_w = 2048u32;
        let mut atlas_h = 256u32;
        let mut x = padding;
        let mut y = padding;
        let mut row_h = 0u32;

        // store bitmaps temporarily
        let mut placements: Vec<(char, Vec<u8>, u32, u32, i32, i32, f32)> = Vec::new();

        for c in 32u8..=126u8 {
            let ch = c as char;
            let (metrics, bitmap) = font.rasterize(ch, font_size);
            let w = metrics.width as u32;
            let h = metrics.height as u32;
            if w == 0 || h == 0 {
                // Still need to store advance and offsets
                placements.push((ch, Vec::new(), w, h, metrics.xmin, metrics.ymin, metrics.advance_width));
                continue;
            }
            if x + w + padding > atlas_w {
                // new row
                x = padding;
                y += row_h + padding;
                row_h = 0;
            }
            placements.push((ch, bitmap, w, h, metrics.xmin, metrics.ymin, metrics.advance_width));
            x += w + padding;
            row_h = row_h.max(h);
            atlas_h = atlas_h.max(y + row_h + padding);
        }

        // Create atlas R8 texture
        let atlas_size = Extent3d {
            width: atlas_w,
            height: atlas_h,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: Some("font-atlas"),
            size: atlas_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // create a cpu buffer for atlas (R8)
        let mut atlas_buf = vec![0u8; (atlas_w * atlas_h) as usize];

        // place glyphs
        x = padding;
        y = padding;
        row_h = 0;
        let mut glyphs = HashMap::new();

        for (ch, bitmap, w, h, xmin, ymin, advance) in placements {
            if w == 0 || h == 0 {
                // empty glyph -> store advance only
                glyphs.insert(ch, GlyphInfo {
                    u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0,
                    width: 0, height: 0,
                    advance,
                    xoffset: xmin, yoffset: ymin,
                });
                continue;
            }
            if x + w + padding > atlas_w {
                x = padding;
                y += row_h + padding;
                row_h = 0;
            }
            for row in 0..h {
                let dst_off = ((y + row) * atlas_w + x) as usize;
                let src_off = (row * w) as usize;
                atlas_buf[dst_off..dst_off + w as usize].copy_from_slice(&bitmap[src_off..src_off + w as usize]);
            }
            let u0 = x as f32 / atlas_w as f32;
            let v0 = y as f32 / atlas_h as f32;
            let u1 = (x + w) as f32 / atlas_w as f32;
            let v1 = (y + h) as f32 / atlas_h as f32;

            glyphs.insert(ch, GlyphInfo {
                u0, v0, u1, v1,
                width: w, height: h,
                advance,
                xoffset: xmin, yoffset: ymin,
            });

            x += w + padding;
            row_h = row_h.max(h);
        }

        // Upload atlas to GPU using write_texture
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_buf,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(atlas_w),
                rows_per_image: None,
            },
            atlas_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create a simple sampler for the atlas
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("font-atlas-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("font-atlas-bind-group"),
        });

        Ok(Self {
            atlas_width: atlas_w,
            atlas_height: atlas_h,
            texture_view,
            bind_group,
            glyphs,
            font_size,
        })
    }
}

/// Vertex for textured quad.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // pos
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // uv
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// GPU renderer owning the device, queue, surface and text pipelines.
pub struct Renderer<'a> {
    // Keep the Instance alive to preserve ownership relationships required by wgpu
    _instance: Instance,
    surface: Surface<'a>,
    _adapter: wgpu::Adapter,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    clear_color: Color,
    _window_lifetime: PhantomData<&'a Window>,
    /// Resolved theme tokens supplied by the theme crate (semantic colors).
    theme: Theme,

    // pipelines / bind groups
    text_pipeline: wgpu::RenderPipeline,
    text_bind_layout: BindGroupLayout,
    // font atlas
    font_atlas: FontAtlas,

    // vertex/index buffers reused each frame
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

impl<'a> Renderer<'a> {
    /// Create a new renderer. `window` must outlive the returned Renderer.
    ///
    /// Additionally accepts the shared AppState so the renderer can prepare
    /// state dependent resources (if needed).
    pub async fn new(window: &'a Window, clear_color: [f64; 4], app_state: Arc<std::sync::Mutex<AppState>>) -> Result<Self, RenderError> {
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

        // NOTE: in this workspace adapter.request_device takes a single argument.
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

        // Create bind group layout for font atlas (texture + sampler)
        let text_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // sampled texture (R8)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("text_bind_layout"),
        });

        // Build font atlas now
        let font_size = 14.0;
        let font_atlas = FontAtlas::new(&device, &queue, &text_bind_layout, font_size)?;

        // Create a simple shader for textured text (WGSL).
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text_shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text-pipeline-layout"),
            bind_group_layouts: &[&text_bind_layout],
            push_constant_ranges: &[],
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // create empty vertex/index buffers sized for moderate content; we'll recreate if needed
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vb"),
            size: 65536,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ib"),
            size: 65536,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        info!("Renderer initialized ({}x{})", config.width, config.height);

        Ok(Self {
            _instance: instance,
            surface,
            _adapter: adapter,
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
            text_pipeline,
            text_bind_layout,
            font_atlas,
            vertex_buffer,
            index_buffer,
            theme: Theme::default(),
            index_count: 0,
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

    /// Render a single frame using the provided AppState as the source of truth.
    pub fn render(&mut self, app_state: &AppState) -> Result<(), RenderError> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }

        // Build draw lists from app_state into vertex/index buffers.
        // For simplicity we only render textual labels and simple colored quads
        // representing panels. Text is rendered via the glyph atlas.

        // Example layout metrics
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        let theme = &self.theme;

        // Build a simple vertex list
        let mut verts: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();

        // Helper to push a colored quad (background) - here we use white texture uv = 0
        let mut push_colored_quad = |x: f32, y: f32, w: f32, h: f32, color: [f32;4]| {
            let base = verts.len() as u16;
            let v0 = Vertex { pos: [x, y], uv: [0.0, 0.0], color };
            let v1 = Vertex { pos: [x+w, y], uv: [0.0, 0.0], color };
            let v2 = Vertex { pos: [x+w, y+h], uv: [0.0, 0.0], color };
            let v3 = Vertex { pos: [x, y+h], uv: [0.0, 0.0], color };
            verts.push(v0); verts.push(v1); verts.push(v2); verts.push(v3);
            indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        };

        // Top bar: 48 px height
        push_colored_quad(0.0, 0.0, width, 48.0, color_to_rgba(&theme.title_bar_background));

        // Left sidebar: 260px width
        push_colored_quad(0.0, 48.0, 260.0, height - 48.0 - 24.0, color_to_rgba(&theme.sidebar_background));

        // Right assistant panel: 320px width
        push_colored_quad(width - 320.0, 48.0, 320.0, height - 48.0 - 24.0, color_to_rgba(&theme.assistant_panel_background));

        // Bottom panel: 200px height anchored above status bar
        // Use elevated panel background for bottom area to give subtle separation.
        push_colored_quad(260.0, height - 24.0 - 200.0, width - 260.0 - 320.0, 200.0, color_to_rgba(&theme.elevated_panel_background));

        // Editor area: center
        push_colored_quad(260.0, 48.0, width - 260.0 - 320.0, height - 48.0 - 24.0 - 200.0, color_to_rgba(&theme.editor_background));

        // Bottom status bar: 24 px height
        push_colored_quad(0.0, height - 24.0, width, 24.0, color_to_rgba(&theme.status_bar_background));

        // Text: render a few labels from app_state

        // Helper to emit glyphs for a given string at (x,y) in pixels (top-left origin).
        let mut cursor_y = 12.0;
        let origin_y = 0.0;

        // Title in top bar
        let title = &app_state.config.title;
        self.emit_text(&mut verts, &mut indices, 12.0, 12.0, title, color_to_rgba(&theme.text_primary), width, height)?;

        // Tabs header - simple
        let tabs = app_state.tabs.tabs.iter().map(|t| t.title.clone()).collect::<Vec<_>>().join("  ");
        self.emit_text(&mut verts, &mut indices, 200.0, 12.0, &tabs, color_to_rgba(&theme.text_muted), width, height)?;

        // Sidebar title
        self.emit_text(&mut verts, &mut indices, 12.0, 64.0, "Workspace", color_to_rgba(&theme.text_primary), width, height)?;

        // List some workspace items
        for (i, item) in app_state.workspace.items.iter().enumerate() {
            let y = 96.0 + i as f32 * 20.0;
            self.emit_text(&mut verts, &mut indices, 12.0, y, &item.name, color_to_rgba(&theme.text_muted), width, height)?;
        }

        // Editor sample: render first few lines of active document
        if let Some(doc) = app_state.editor.active_document().cloned() {
            // render document title in editor header
            self.emit_text(&mut verts, &mut indices, 280.0, 56.0, &doc.display_name, color_to_rgba(&theme.text_primary), width, height)?;

            // split lines and render first 20 lines
            for (i, line) in doc.text.lines().take(20).enumerate() {
                let y = 86.0 + i as f32 * 18.0;
                // line numbers
                let ln = format!("{:>3} ", i+1);
                self.emit_text(&mut verts, &mut indices, 268.0, y, &ln, color_to_rgba(&theme.text_muted), width, height)?;
                self.emit_text(&mut verts, &mut indices, 300.0, y, line, color_to_rgba(&theme.text_primary), width, height)?;
            }
        }

        // Assistant header
        self.emit_text(&mut verts, &mut indices, width - 300.0, 64.0, "AI Assistant", color_to_rgba(&theme.text_primary), width, height)?;
        // Assistant messages
        for (i, m) in app_state.assistant.messages.iter().enumerate().take(6) {
            let y = 96.0 + i as f32 * 18.0;
            self.emit_text(&mut verts, &mut indices, width - 300.0, y, m, color_to_rgba(&theme.text_muted), width, height)?;
        }

        // Status bar text
        let status = &app_state.status.message;
        self.emit_text(&mut verts, &mut indices, 8.0, height - 18.0, status, color_to_rgba(&theme.text_muted), width, height)?;

        // Upload vertex/index data
        let vb_bytes = bytemuck::cast_slice(&verts);
        self.queue.write_buffer(&self.vertex_buffer, 0, vb_bytes);

        let ib_bytes = bytemuck::cast_slice(&indices);
        self.queue.write_buffer(&self.index_buffer, 0, ib_bytes);

        // Acquire frame and render
        let surface_texture = match self.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(e) => {
                // Map wgpu surface errors to renderer surface errors
                return match e {
                    wgpu::SurfaceError::Lost => Err(RenderError::SurfaceLost),
                    wgpu::SurfaceError::OutOfMemory => Err(RenderError::Other("surface out of memory".to_string())),
                    wgpu::SurfaceError::Timeout => Err(RenderError::SurfaceTimeout),
                    wgpu::SurfaceError::Outdated => Err(RenderError::SurfaceOutdated),
                    // Fallback for unexpected cases
                    other => Err(RenderError::Other(format!("surface error: {:?}", other))),
                };
            }
        };

        let view = surface_texture.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("zaroxi-render-encoder"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            rpass.set_pipeline(&self.text_pipeline);
            rpass.set_bind_group(0, &self.font_atlas.bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        Ok(())
    }

    /// Emit text into the provided vertex/index arrays using the font atlas.
    fn emit_text(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u16>, mut x: f32, y: f32, text: &str, color: [f32;4], _screen_w: f32, _screen_h: f32) -> Result<(), RenderError> {
        let base_index = verts.len() as u16;
        for ch in text.chars() {
            let glyph = self.font_atlas.glyphs.get(&ch);
            if glyph.is_none() {
                // skip unknown glyphs
                continue;
            }
            let g = glyph.unwrap();
            if g.width == 0 || g.height == 0 {
                x += g.advance;
                continue;
            }
            // positions: top-left origin; atlas uv coordinates map into glyph
            let x0 = x as f32 + g.xoffset as f32;
            let y0 = y as f32 + g.yoffset as f32;
            let x1 = x0 + g.width as f32;
            let y1 = y0 + g.height as f32;
            // UVs
            let u0 = g.u0;
            let v0 = g.v0;
            let u1 = g.u1;
            let v1 = g.v1;

            let a = Vertex { pos: [x0, y0], uv: [u0, v0], color };
            let b = Vertex { pos: [x1, y0], uv: [u1, v0], color };
            let c = Vertex { pos: [x1, y1], uv: [u1, v1], color };
            let d = Vertex { pos: [x0, y1], uv: [u0, v1], color };

            verts.push(a); verts.push(b); verts.push(c); verts.push(d);
            let i0 = base_index + (verts.len() as u16 - 4);
            indices.extend_from_slice(&[i0, i0+1, i0+2, i0, i0+2, i0+3]);

            x += g.advance;
        }
        Ok(())
    }
}
