use crate::error::RenderError;
use log::{debug, info};
use std::marker::PhantomData;
use std::path::PathBuf;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use wgpu::{
    Backends, BindGroup, BindGroupLayout, Buffer, CommandEncoderDescriptor, Device, DeviceDescriptor,
    Features, Instance, InstanceDescriptor, Limits, PresentMode, Queue, RequestAdapterOptions, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, Color, LoadOp, Operations,
    StoreOp, Extent3d, TextureDescriptor, TextureDimension, TextureView, SamplerDescriptor,
};

use fontdue::Font;
use std::collections::HashMap;

use zaroxi_app::AppState;
use zaroxi_theme::{SemanticColors, Color as ThemeColor};

/// Helper to convert theme Color -> renderer [f32;4]
fn color_to_rgba(c: &ThemeColor) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

/// Simple rectangle used by the resolved layout.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Resolved layout passed into the renderer. Layout is owned by the app
/// / layout layer; the renderer simply consumes it.
#[derive(Debug, Clone)]
pub struct RenderLayout {
    pub title_bar: Rect,
    pub sidebar: Rect,
    pub editor: Rect,
    pub right_panel: Rect,
    pub bottom_panel: Rect,
    pub status_bar: Rect,
    pub colors: SemanticColors,
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

        // Upload atlas to GPU using queue.write_texture (direct write) with the
        // wgpu 29.0.3 texel-copy types. This keeps the renderer implementation
        // compact and avoids introducing a direct dependency on wgpu_types.
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_buf,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas_w),
                rows_per_image: Some(atlas_h),
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
            // use defaults for mipmap behavior to avoid version mismatches
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
            // wgpu 29 uses Option<&BindGroupLayout> in the slice
            bind_group_layouts: &[Some(&text_bind_layout)],
            ..Default::default()
        });

        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            // wgpu 29 uses multiview_mask & cache fields
            multiview_mask: None,
            cache: None,
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

    /// Render a single frame using the provided resolved layout and AppState.
    ///
    /// Important: layout (panel geometry + resolved colors) is owned by the
    /// application/layout layer. The renderer only draws the provided layout.
    pub fn render_with_layout(&mut self, app_state: &AppState, layout: &RenderLayout, render_panels: &[zaroxi_app::view_model::RenderPanel]) -> Result<(), RenderError> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }
        info!(
            "entering render_with_layout (window {}x{}), render_panels={}",
            self.config.width,
            self.config.height,
            render_panels.len()
        );

        // Log received render panels for traceability.
        for p in render_panels {
            info!("renderer received render_panel id='{}' title='{}' visible={}", p.id, p.title, p.visible);
        }

        // Build draw lists from app_state into vertex/index buffers.
        // The renderer consumes the resolved layout (rects + colors).
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        // Use colors supplied by the resolved layout (owned by app/layout).
        let sem = &layout.colors;

        // Build a simple vertex list
        let mut verts: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();

        // Helper function to push a colored quad (background) into the provided
        // vertex/index vectors. Using a free function avoids keeping mutable borrows
        // alive across the render function scope which would conflict with other
        // mutable operations (like emitting text).
        //
        // NOTE: The WGSL vertex shader expects clip-space coordinates in the
        // range [-1.0, 1.0] (NDC). The application layer provides positions in
        // pixel coordinates (top-left origin). Convert pixels -> NDC here and
        // flip Y so UI top-left maps correctly into NDC space.
        fn push_colored_quad(
            verts: &mut Vec<Vertex>,
            indices: &mut Vec<u16>,
            x: f32,
            y: f32,
            w: f32,
            h: f32,
            color: [f32; 4],
            screen_w: f32,
            screen_h: f32,
        ) {
            // Convert pixel coordinates (top-left origin) -> NDC clip space used by the shader.
            // NDC x: -1..1 left->right, NDC y: -1..1 bottom->top. We want top-left origin for UI,
            // so map y accordingly by flipping.
            fn pixel_to_ndc(px: f32, py: f32, sw: f32, sh: f32) -> [f32; 2] {
                let nx = (px / sw) * 2.0 - 1.0;
                let ny = 1.0 - (py / sh) * 2.0;
                [nx, ny]
            }

            let base = verts.len() as u16;
            let a = pixel_to_ndc(x, y, screen_w, screen_h);
            let b = pixel_to_ndc(x + w, y, screen_w, screen_h);
            let c = pixel_to_ndc(x + w, y + h, screen_w, screen_h);
            let d = pixel_to_ndc(x, y + h, screen_w, screen_h);

            let v0 = Vertex { pos: a, uv: [0.0, 0.0], color };
            let v1 = Vertex { pos: b, uv: [0.0, 0.0], color };
            let v2 = Vertex { pos: c, uv: [0.0, 0.0], color };
            let v3 = Vertex { pos: d, uv: [0.0, 0.0], color };

            verts.push(v0);
            verts.push(v1);
            verts.push(v2);
            verts.push(v3);
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        // Convert render panels into visible content quads and text.
        info!("[renderer] render_panels count = {}", render_panels.len());

        // --- VISUAL DEBUG: guaranteed visible debug quads (solid colors, no alpha) ---
        // If the window still renders clear/gray, vertex coordinates or shader mapping
        // are wrong. These three rectangles should be unmissable:
        //  - inset fullscreen magenta
        //  - top-left green quarter
        //  - centered blue rectangle
        {
            // inset magenta fullscreen
            let inset = 8.0f32;
            let rx = inset;
            let ry = inset;
            let rw = (width as f32) - inset * 2.0;
            let rh = (height as f32) - inset * 2.0;
            push_colored_quad(&mut verts, &mut indices, rx, ry, rw, rh, [1.0, 0.0, 1.0, 1.0], width, height);
        }

        {
            // top-left quarter green
            let rw = width * 0.5;
            let rh = height * 0.5;
            push_colored_quad(&mut verts, &mut indices, 0.0, 0.0, rw, rh, [0.0, 1.0, 0.0, 1.0], width, height);
        }

        {
            // centered blue
            let rw = width * 0.25;
            let rh = height * 0.25;
            let rx = (width - rw) * 0.5;
            let ry = (height - rh) * 0.5;
            push_colored_quad(&mut verts, &mut indices, rx, ry, rw, rh, [0.0, 0.4, 1.0, 1.0], width, height);
        }
        // --- end visual debug ---

        // For each panel supplied by the app, create a header and content block and queue title/content text.
        let header_h = 28.0f32;
        let content_padding = 8.0f32;
        for panel in render_panels.iter() {
            info!("drawing panel id='{}' title='{}' visible={}", panel.id, panel.title, panel.visible);
            if !panel.visible {
                info!("panel '{}' is hidden; skipping", panel.id);
                continue;
            }

            // Map panel id -> rect
            let target = match panel.id.as_str() {
                "titlebar" => layout.title_bar,
                "sidebar" => layout.sidebar,
                "editor" => layout.editor,
                "right_panel" => layout.right_panel,
                "bottom_panel" => layout.bottom_panel,
                "status_bar" => layout.status_bar,
                other => {
                    info!("unknown panel id '{}', skipping", other);
                    continue;
                }
            };

            // Header strip at the top of the panel rect
            let hx = target.x;
            let hy = target.y;
            let hw = target.w;
            let hh = header_h.min(target.h.max(0.0));
            let header_color = [0.12, 0.13, 0.16, 1.0];
            push_colored_quad(&mut verts, &mut indices, hx, hy, hw, hh, header_color, width, height);

            // Content inset: a smaller block inside the panel for visual differentiation
            let cx = target.x + content_padding;
            let cy = target.y + hh + content_padding;
            let cw = (target.w - content_padding * 2.0).max(0.0);
            let ch = (target.h - hh - content_padding * 2.0).max(0.0);
            let content_color = [0.08, 0.09, 0.11, 1.0];
            if cw > 0.0 && ch > 0.0 {
                push_colored_quad(&mut verts, &mut indices, cx, cy, cw, ch, content_color, width, height);
            }

            // Queue header/title text
            let title_x = hx + 8.0;
            let title_y = hy + 6.0;
            let _ = self.emit_text(&mut verts, &mut indices, title_x, title_y, &panel.title, [0.95, 0.95, 0.95, 1.0], width, height);

            // Queue body/content text (first line only, if any)
            if !panel.content.is_empty() {
                let content_x = cx + 6.0;
                let content_y = cy + 6.0;
                let _ = self.emit_text(&mut verts, &mut indices, content_x, content_y, &panel.content, [0.8, 0.8, 0.8, 1.0], width, height);
            }

            // Log counts per panel
            let quad_count = (verts.len() / 4) as usize;
            info!("panel '{}' queued: quads_total={} verts_total={} indices_total={}", panel.id, quad_count, verts.len(), indices.len());
        }

        // Log final totals
        info!("[renderer] final verts={}, indices={}", verts.len(), indices.len());

        // Dump first few vertices so we can inspect coordinate space (pos, uv, color).
        let max_log = std::cmp::min(8usize, verts.len());
        info!("vertex[0..{}] dump:", max_log);
        for i in 0..max_log {
            let v = verts[i];
            info!(
                "v[{}] pos=({:.4}, {:.4}) uv=({:.4}, {:.4}) color=({:.4}, {:.4}, {:.4}, {:.4})",
                i, v.pos[0], v.pos[1], v.uv[0], v.uv[1], v.color[0], v.color[1], v.color[2], v.color[3]
            );
        }

        // Upload vertex/index data
        let vb_bytes = bytemuck::cast_slice(&verts);
        self.queue.write_buffer(&self.vertex_buffer, 0, vb_bytes);

        let ib_bytes = bytemuck::cast_slice(&indices);
        self.queue.write_buffer(&self.index_buffer, 0, ib_bytes);

        // Acquire current surface texture (wgpu 29 CurrentSurfaceTexture API)
        let current = self.surface.get_current_texture();

        match current {
            wgpu::CurrentSurfaceTexture::Success(frame) => {
                let view = frame.texture.create_view(&TextureViewDescriptor::default());

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
                            depth_slice: None,
                        })],
                        ..Default::default()
                    });

                    // Ensure pipeline and buffers are bound
                    rpass.set_pipeline(&self.text_pipeline);
                    rpass.set_bind_group(0, &self.font_atlas.bind_group, &[]);
                    rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

                    // Debug: choose non-indexed draw for the visibility test.
                    if verts.len() > 0 && indices.is_empty() {
                        // Log debug draw
                        info!("issuing debug non-indexed draw: verts={}", verts.len());
                        // Draw vertices directly (no indices). Each vertex is a triangle list vertex.
                        rpass.draw(0..(verts.len() as u32), 0..1);
                    } else {
                        // Normal path: indexed draw
                        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        rpass.draw_indexed(0..indices.len() as u32, 0, 0..1);
                    }
                }

                self.queue.submit(Some(encoder.finish()));
                frame.present();
                info!("submitted frame");
                Ok(())
            }

            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                let view = frame.texture.create_view(&TextureViewDescriptor::default());

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
                            depth_slice: None,
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
                frame.present();
                Err(RenderError::SurfaceOutdated)
            }

            wgpu::CurrentSurfaceTexture::Timeout => {
                debug!("Surface timeout; skipping frame");
                Err(RenderError::SurfaceTimeout)
            }

            wgpu::CurrentSurfaceTexture::Occluded => {
                debug!("Surface occluded; skipping frame");
                Err(RenderError::SurfaceOccluded)
            }

            wgpu::CurrentSurfaceTexture::Outdated => {
                debug!("Surface outdated; reconfigure required");
                Err(RenderError::SurfaceOutdated)
            }

            wgpu::CurrentSurfaceTexture::Lost => {
                debug!("Surface lost; reconfigure required");
                Err(RenderError::SurfaceLost)
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                debug!("Surface validation variant encountered");
                Err(RenderError::SurfaceValidation("validation error".to_string()))
            }
        }
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
