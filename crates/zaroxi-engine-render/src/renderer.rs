use crate::error::RenderError;
use log::{debug, info};
use std::marker::PhantomData;
use std::path::PathBuf;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use wgpu::{
    Backends, BindGroup, BindGroupLayout, Buffer, CommandEncoderDescriptor, Device, DeviceDescriptor,
    Features, Instance, InstanceDescriptor, Limits, PresentMode, Queue, RequestAdapterOptions, Surface,
    SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor, Color,
    Extent3d, TextureDescriptor, TextureDimension, TextureView, SamplerDescriptor,
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
        info!(
            "font atlas upload: format=R8Unorm size={}x{} bytes_per_row={}",
            atlas_w, atlas_h, atlas_w
        );

        // Detailed atlas diagnostics: total bytes, non-zero bytes, max value,
        // and first non-zero index (if any). These help detect an entirely
        // blank atlas (glyph rasterization failure).
        let total_bytes = atlas_buf.len();
        let non_zero = atlas_buf.iter().filter(|&&b| b != 0u8).count();
        let max_val = *atlas_buf.iter().max().unwrap_or(&0u8);
        let first_non_zero = atlas_buf.iter().position(|&b| b != 0u8);
        info!(
            "font atlas stats bytes={} non_zero={} max={} first_non_zero={:?}",
            total_bytes, non_zero, max_val, first_non_zero
        );

        let first_n = std::cmp::min(8usize, atlas_buf.len());
        info!("font atlas first {} bytes: {:?}", first_n, &atlas_buf[..first_n]);

        // Fail fast if the atlas is entirely blank; this indicates glyph rasterization
        // produced no coverage and must be investigated on the CPU side.
        if non_zero == 0 {
            return Err(RenderError::Other(
                "font atlas is entirely zero; glyph rasterization/upload source is blank"
                    .to_string(),
            ));
        }

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

        info!("font atlas upload completed ({}x{})", atlas_w, atlas_h);

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

        info!(
            "font atlas sampler: mag_filter={:?} min_filter={:?} address_mode=ClampToEdge",
            wgpu::FilterMode::Linear,
            wgpu::FilterMode::Linear
        );

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

        info!("font atlas bind_group created; shader coverage channel assumed = .r");

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
    // Minimal debug pipeline that draws solid vertex colors (no texture/sampler).
    debug_pipeline: wgpu::RenderPipeline,
    // Solid-shape pipeline used for all non-text UI quads (panels / borders).
    shape_pipeline: wgpu::RenderPipeline,
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

        // Diagnostic: log text pipeline target format and blend usage so we can
        // correlate shader behavior with pipeline state.
        {
            use std::mem;
            let vertex_size = mem::size_of::<Vertex>();
            let expected_vertex_size = 32usize; // vec2 + vec2 + vec4 -> (2+2+4)*4 = 32 bytes

            info!("text pipeline created: color_format={:?}, blend=ALPHA_BLENDING", config.format);
            info!("Vertex struct: size_of::<Vertex>() = {}", vertex_size);
            info!("Vertex buffer layout (Rust -> WGSL):");
            info!("  - @location(0) pos : Float32x2  @ offset 0");
            info!("  - @location(1) uv  : Float32x2  @ offset 8");
            info!("  - @location(2) color: Float32x4 @ offset 16");
            info!("  - array_stride = {} (bytes)", vertex_size);

            // Sanity check: ensure Rust Vertex size matches expected WGSL layout size.
            if vertex_size != expected_vertex_size {
                return Err(RenderError::Other(format!(
                    "Vertex size mismatch: expected {} bytes (vec2+vec2+vec4), got {}",
                    expected_vertex_size, vertex_size
                )));
            }
        }

        // Create a minimal solid-color pipeline for debug-only draws.
        // This pipeline does not sample any textures or use bind groups.
        let debug_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("debug-color-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("debug_color_shader.wgsl").into()),
        });

        let debug_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("debug-pipeline-layout"),
            bind_group_layouts: &[],
            ..Default::default()
        });

        let debug_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("debug-pipeline"),
            layout: Some(&debug_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &debug_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &debug_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    // No blending: replace output directly.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Shape pipeline: dedicated minimal solid-color pipeline used for all
        // non-text UI geometry (panels, borders, dividers). This avoids sampling
        // the font atlas or relying on text bind groups for simple colored quads.
        let shape_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shape-color-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shape_shader.wgsl").into()),
        });

        let shape_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shape-pipeline-layout"),
            bind_group_layouts: &[],
            ..Default::default()
        });

        let shape_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shape-pipeline"),
            layout: Some(&shape_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shape_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shape_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    // No blending: replace output directly for shape fills.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
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
            debug_pipeline,
            shape_pipeline,
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
    pub fn render_with_layout(&mut self, _app_state: &AppState, layout: &RenderLayout, render_panels: &[zaroxi_app::view_model::RenderPanel]) -> Result<(), RenderError> {
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
        //
        // Semantic mapping (for reviewer):
        // - root app bg            -> colors.app_background
        // - titlebar bg            -> colors.title_bar_background
        // - sidebar bg             -> colors.sidebar_background
        // - editor bg              -> colors.editor_background
        // - assistant bg           -> colors.assistant_panel_background
        // - bottom panel bg        -> colors.panel_background
        // - statusbar bg           -> colors.status_bar_background
        // - panel headers          -> colors.panel_header_background
        // - borders/dividers       -> colors.border / colors.divider
        //
        // Diagnostic override: set FORCE_DIAGNOSTIC_COLORS = true to force highly
        // contrasting colors (red/green/blue/...) for quick visual verification.
        const FORCE_DIAGNOSTIC_COLORS: bool = false;
        // DIAGNOSTIC_TEXT_ONLY: if true, skip the shape pass and render only text
        // (useful to verify text/atlas/pipeline independently).
        const DIAGNOSTIC_TEXT_ONLY: bool = false;
        // When true, avoid any scissor operations (if present). Kept true by
        // default for the isolated diagnostic run.
        const DIAGNOSTIC_DISABLE_SCISSOR: bool = true;
        // Optional forced text color when DIAGNOSTIC_TEXT_ONLY is enabled.
        // Set to Some([r,g,b,a]) to force all text to a bright color for visibility.
        const DIAGNOSTIC_FORCE_TEXT_COLOR: Option<[f32; 4]> = None;
        // DIAGNOSTIC_FULLSCREEN_QUAD: inject a full-screen solid quad into the
        // shape (panel) vertex list to validate render-pass / pipeline state.
        const DIAGNOSTIC_FULLSCREEN_QUAD: bool = true;
        // DIAGNOSTIC_INJECT_CENTER_TEXT: inject a single small diagnostic quad
        // into the text vertex list, centered on screen (NDC) to validate text path.
        const DIAGNOSTIC_INJECT_CENTER_TEXT: bool = true;
        info!("debug geometry injection enabled={}, FORCE_DIAGNOSTIC_COLORS={}, DIAGNOSTIC_TEXT_ONLY={}, DIAGNOSTIC_DISABLE_SCISSOR={}, DIAGNOSTIC_FULLSCREEN_QUAD={}, DIAGNOSTIC_INJECT_CENTER_TEXT={}",
            DEBUG_RENDER, FORCE_DIAGNOSTIC_COLORS, DIAGNOSTIC_TEXT_ONLY, DIAGNOSTIC_DISABLE_SCISSOR, DIAGNOSTIC_FULLSCREEN_QUAD, DIAGNOSTIC_INJECT_CENTER_TEXT);

        let sem = &layout.colors;

        // Build separate lists for shape (panel) geometry and text geometry so we
        // can render them with different pipelines:
        //  - panel_verts / panel_indices -> drawn with shape_pipeline
        //  - text_verts  / text_indices  -> drawn with text_pipeline (font sampling)
        let mut panel_verts: Vec<Vertex> = Vec::new();
        let mut panel_indices: Vec<u16> = Vec::new();
        let mut text_verts: Vec<Vertex> = Vec::new();
        let mut text_indices: Vec<u16> = Vec::new();

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

        // Debug injection flag: keeps visual debug geometry and debug pass off by default.
        // Set to `true` when you need to re-enable the quick NDC/vertex layout checks.
        const DEBUG_RENDER: bool = false;
        info!("debug geometry injection enabled={}", DEBUG_RENDER);

        if DEBUG_RENDER {
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
                push_colored_quad(&mut panel_verts, &mut panel_indices, rx, ry, rw, rh, [1.0, 0.0, 1.0, 1.0], width, height);
            }

            {
                // top-left quarter green
                let rw = width * 0.5;
                let rh = height * 0.5;
                push_colored_quad(&mut panel_verts, &mut panel_indices, 0.0, 0.0, rw, rh, [0.0, 1.0, 0.0, 1.0], width, height);
            }

            {
                // centered blue
                let rw = width * 0.25;
                let rh = height * 0.25;
                let rx = (width - rw) * 0.5;
                let ry = (height - rh) * 0.5;
                push_colored_quad(&mut panel_verts, &mut panel_indices, rx, ry, rw, rh, [0.0, 0.4, 1.0, 1.0], width, height);
            }
            // --- end visual debug ---
        }

        // DIAGNOSTIC: optionally inject a fullscreen red quad into the panel (shape) list.
        if DIAGNOSTIC_FULLSCREEN_QUAD {
            info!("DIAGNOSTIC: injecting fullscreen red quad into panel_verts");
            // push full-screen in pixel coords; push_colored_quad will convert to NDC.
            push_colored_quad(
                &mut panel_verts,
                &mut panel_indices,
                0.0,
                0.0,
                width as f32,
                height as f32,
                [1.0, 0.0, 0.0, 1.0],
                width,
                height,
            );
        }

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
            // Log scissor/rect that would be used for this panel (diagnostic).
            info!("panel '{}' scissor_rect = {:?}", panel.id, target);

            // Header strip at the top of the panel rect
            let hx = target.x;
            let hy = target.y;
            let hw = target.w;
            let hh = header_h.min(target.h.max(0.0));

            // Choose a semantic header color per-panel (defaults -> panel_header_background).
            let header_color: [f32; 4] = if FORCE_DIAGNOSTIC_COLORS {
                match panel.id.as_str() {
                    "titlebar" => [1.0, 0.0, 0.0, 1.0],     // red
                    "sidebar" => [0.0, 1.0, 0.0, 1.0],      // green
                    "editor" => [0.0, 0.0, 1.0, 1.0],       // blue
                    "right_panel" => [1.0, 1.0, 0.0, 1.0],  // yellow
                    "bottom_panel" => [0.0, 1.0, 1.0, 1.0], // cyan
                    "status_bar" => [1.0, 0.0, 1.0, 1.0],   // magenta
                    _ => [1.0, 0.2, 0.4, 1.0],              // fallback bright
                }
            } else {
                // Default semantic mapping
                match panel.id.as_str() {
                    "titlebar" => color_to_rgba(&sem.title_bar_background),
                    "sidebar" => color_to_rgba(&sem.panel_header_background),
                    "editor" => color_to_rgba(&sem.panel_header_background),
                    "right_panel" => color_to_rgba(&sem.panel_header_background),
                    "bottom_panel" => color_to_rgba(&sem.panel_header_background),
                    "status_bar" => color_to_rgba(&sem.panel_header_background),
                    _ => color_to_rgba(&sem.panel_header_background),
                }
            };

            info!("panel '{}' header_color = {:?}", panel.id, header_color);
            push_colored_quad(&mut panel_verts, &mut panel_indices, hx, hy, hw, hh, header_color, width, height);

            // Content inset: a smaller block inside the panel for visual differentiation
            let cx = target.x + content_padding;
            let cy = target.y + hh + content_padding;
            let cw = (target.w - content_padding * 2.0).max(0.0);
            let ch = (target.h - hh - content_padding * 2.0).max(0.0);
            // Choose a semantic content/background color per-panel.
            let content_color: [f32; 4] = if FORCE_DIAGNOSTIC_COLORS {
                match panel.id.as_str() {
                    "titlebar" => [0.6, 0.0, 0.0, 1.0],      // darker red
                    "sidebar" => [0.0, 0.6, 0.0, 1.0],       // darker green
                    "editor" => [0.0, 0.0, 0.6, 1.0],        // darker blue
                    "right_panel" => [0.6, 0.6, 0.0, 1.0],   // darker yellow
                    "bottom_panel" => [0.0, 0.6, 0.6, 1.0],  // darker cyan
                    "status_bar" => [0.6, 0.0, 0.6, 1.0],    // darker magenta
                    _ => [0.12, 0.12, 0.12, 1.0],            // fallback
                }
            } else {
                match panel.id.as_str() {
                    "titlebar" => color_to_rgba(&sem.app_chrome_background),
                    "sidebar" => color_to_rgba(&sem.sidebar_background),
                    "editor" => color_to_rgba(&sem.editor_background),
                    "right_panel" => color_to_rgba(&sem.assistant_panel_background),
                    "bottom_panel" => color_to_rgba(&sem.panel_background),
                    "status_bar" => color_to_rgba(&sem.status_bar_background),
                    _ => color_to_rgba(&sem.panel_background),
                }
            };

            info!("panel '{}' content_color = {:?}", panel.id, content_color);
            if cw > 0.0 && ch > 0.0 {
                push_colored_quad(&mut panel_verts, &mut panel_indices, cx, cy, cw, ch, content_color, width, height);
            }

            // Queue header/title text
            let title_x = hx + 8.0;
            let title_y = hy + 6.0;
            // When running diagnostics we may force a single bright color for all text
            let title_color: [f32; 4] = if DIAGNOSTIC_TEXT_ONLY {
                DIAGNOSTIC_FORCE_TEXT_COLOR.unwrap_or([1.0, 1.0, 1.0, 1.0])
            } else {
                [0.95, 0.95, 0.95, 1.0]
            };
            let _ = self.emit_text(&mut text_verts, &mut text_indices, title_x, title_y, &panel.title, title_color, width, height);

            // Queue body/content text (first line only, if any)
            if !panel.content.is_empty() {
                let content_x = cx + 6.0;
                let content_y = cy + 6.0;
                let content_color: [f32; 4] = if DIAGNOSTIC_TEXT_ONLY {
                    DIAGNOSTIC_FORCE_TEXT_COLOR.unwrap_or([1.0, 1.0, 1.0, 1.0])
                } else {
                    [0.8, 0.8, 0.8, 1.0]
                };
                let _ = self.emit_text(&mut text_verts, &mut text_indices, content_x, content_y, &panel.content, content_color, width, height);
            }

            // Log counts per panel
            let quad_count = (panel_verts.len() / 4) as usize;
            info!(
                "panel '{}' queued: panel_quads={} panel_verts={} panel_indices={} text_verts={} text_indices={}",
                panel.id,
                quad_count,
                panel_verts.len(),
                panel_indices.len(),
                text_verts.len(),
                text_indices.len()
            );
        }

        // Optional diagnostic: inject a small centered diagnostic text quad directly
        // into the text geometry (NDC quad). This bypasses atlas sampling to verify
        // text pipeline/vertex mapping independently.
        if DIAGNOSTIC_INJECT_CENTER_TEXT {
            info!("DIAGNOSTIC: injecting centered diagnostic text quad (NDC) into text_verts");
            // small quad in NDC coordinates centered at (0,0)
            let size_x = 0.25f32;
            let size_y = 0.12f32;
            let nx0 = -size_x * 0.5;
            let ny0 = -size_y * 0.5;
            let nx1 = size_x * 0.5;
            let ny1 = size_y * 0.5;
            // Use color red opaque and zero UVs (shader diagnostic)
            let base = text_verts.len() as u16;
            let a = Vertex { pos: [nx0, ny0], uv: [0.0, 0.0], color: [1.0, 0.0, 0.0, 1.0] };
            let b = Vertex { pos: [nx1, ny0], uv: [0.0, 0.0], color: [1.0, 0.0, 0.0, 1.0] };
            let c = Vertex { pos: [nx1, ny1], uv: [0.0, 0.0], color: [1.0, 0.0, 0.0, 1.0] };
            let d = Vertex { pos: [nx0, ny1], uv: [0.0, 0.0], color: [1.0, 0.0, 0.0, 1.0] };
            text_verts.push(a);
            text_verts.push(b);
            text_verts.push(c);
            text_verts.push(d);
            text_indices.extend_from_slice(&[base, base+1, base+2, base, base+2, base+3]);
        }

        // Merge panel + text geometry into final buffers. Text indices must be
        // offset by the number of panel vertices.
        let panel_vertex_count = panel_verts.len() as u16;
        let mut verts: Vec<Vertex> = panel_verts;
        // start with panel indices
        let mut indices: Vec<u16> = panel_indices.clone();
        // append adjusted text indices
        for idx in text_indices.iter() {
            indices.push(idx.wrapping_add(panel_vertex_count));
        }
        // append text verts
        verts.extend(text_verts.into_iter());

        // Log final totals
        info!("[renderer] final verts={}, indices={}", verts.len(), indices.len());

        // Warn if vertex positions appear to be outside expected NDC range.
        // Many vertices should already be in NDC (shape quads converted on CPU),
        // while some text verts may still be in pixel coordinates — use a loose
        // detection to highlight clearly out-of-bounds values.
        let mut oob_count = 0usize;
        for (i, v) in verts.iter().enumerate() {
            if v.pos[0].abs() > 1.05 || v.pos[1].abs() > 1.05 {
                oob_count += 1;
                info!("OOB_VERTEX i={} pos=({:.4},{:.4}) uv=({:.4},{:.4}) color=({:.4},{:.4},{:.4},{:.4})",
                    i, v.pos[0], v.pos[1], v.uv[0], v.uv[1], v.color[0], v.color[1], v.color[2], v.color[3]);
            }
        }
        if oob_count > 0 {
            info!("vertex OOB summary: total_verts={} out_of_bounds={}", verts.len(), oob_count);
        } else {
            info!("vertex positions all within expected NDC/pixel ranges (no obvious OOB)");
        }

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

        // Log first few indices to validate index buffer contents & format.
        let max_idx_log = std::cmp::min(12usize, indices.len());
        info!("index[0..{}] dump:", max_idx_log);
        for i in 0..max_idx_log {
            info!("i[{}] = {}", i, indices[i]);
        }

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

                    info!("debug pass enabled={}", DEBUG_RENDER);

                    // If DEBUG_RENDER is enabled, draw the full scene with the debug
                    // solid-color pipeline (no textures/samplers) to validate geometry.
                    if DEBUG_RENDER {
                        rpass.set_pipeline(&self.debug_pipeline);
                        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

                        let total_indices_len = indices.len() as u32;
                        if total_indices_len == 0 {
                            let verts_to_draw = verts.len() as u32;
                            info!("debug non-indexed draw (full): verts={}", verts_to_draw);
                            rpass.draw(0..verts_to_draw, 0..1);
                        } else {
                            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                            info!("debug indexed draw (full): indices_drawn={}", total_indices_len);
                            rpass.draw_indexed(0..total_indices_len, 0, 0..1);
                        }
                    }

                    // SHAPE PASS: draw only the panel/background geometry using the
                    // dedicated shape_pipeline (no font sampling).
                    let panel_indices_len = panel_indices.len() as u32;
                    let total_indices_len = indices.len() as u32;

                    if !DIAGNOSTIC_TEXT_ONLY {
                        if panel_indices_len > 0 {
                            rpass.set_pipeline(&self.shape_pipeline);
                            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                            info!("shape pass indexed draw: indices_drawn={}", panel_indices_len);
                            rpass.draw_indexed(0..panel_indices_len, 0, 0..1);
                        }
                    } else {
                        info!("DIAGNOSTIC_TEXT_ONLY enabled: skipping shape pass");
                    }

                    // TEXT PASS: draw glyph/text geometry using the text pipeline and font atlas.
                    if total_indices_len > panel_indices_len {
                        info!("binding text pipeline and font_atlas bind_group for text pass (DIAGNOSTIC_TEXT_ONLY={})", DIAGNOSTIC_TEXT_ONLY);
                        rpass.set_pipeline(&self.text_pipeline);
                        // Rebind the font atlas bind group (must be set after switching pipeline).
                        rpass.set_bind_group(0, &self.font_atlas.bind_group, &[]);
                        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        info!("text pass indexed draw: indices_drawn={} (offset {})", total_indices_len - panel_indices_len, panel_indices_len);
                        rpass.draw_indexed(panel_indices_len..total_indices_len, 0, 0..1);
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

                    info!("debug pass enabled={}", DEBUG_RENDER);

                    // If DEBUG_RENDER is enabled, draw the full scene with the debug
                    // solid-color pipeline (no textures/samplers) to validate geometry.
                    if DEBUG_RENDER {
                        rpass.set_pipeline(&self.debug_pipeline);
                        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        let total_indices_len = indices.len() as u32;
                        if total_indices_len == 0 {
                            let verts_to_draw = verts.len() as u32;
                            info!("debug non-indexed draw (full, suboptimal path): verts={}", verts_to_draw);
                            rpass.draw(0..verts_to_draw, 0..1);
                        } else {
                            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                            info!("debug indexed draw (full, suboptimal path): indices_drawn={}", total_indices_len);
                            rpass.draw_indexed(0..total_indices_len, 0, 0..1);
                        }
                    }

                    // SHAPE PASS: draw only the panel/background geometry using the
                    // dedicated shape_pipeline (no font sampling).
                    let panel_indices_len = panel_indices.len() as u32;
                    let total_indices_len = indices.len() as u32;

                    if !DIAGNOSTIC_TEXT_ONLY {
                        if panel_indices_len > 0 {
                            rpass.set_pipeline(&self.shape_pipeline);
                            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                            info!("shape pass indexed draw (suboptimal path): indices_drawn={}", panel_indices_len);
                            rpass.draw_indexed(0..panel_indices_len, 0, 0..1);
                        }
                    } else {
                        info!("DIAGNOSTIC_TEXT_ONLY enabled (suboptimal path): skipping shape pass");
                    }

                    // TEXT PASS
                    if total_indices_len > panel_indices_len {
                        info!("binding text pipeline and font_atlas bind_group for text pass (suboptimal path, DIAGNOSTIC_TEXT_ONLY={})", DIAGNOSTIC_TEXT_ONLY);
                        rpass.set_pipeline(&self.text_pipeline);
                        rpass.set_bind_group(0, &self.font_atlas.bind_group, &[]);
                        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        info!("text pass indexed draw (suboptimal path): indices_drawn={} (offset {})", total_indices_len - panel_indices_len, panel_indices_len);
                        rpass.draw_indexed(panel_indices_len..total_indices_len, 0, 0..1);
                    }
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
    fn emit_text(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u16>, mut x: f32, y: f32, text: &str, color: [f32;4], screen_w: f32, screen_h: f32) -> Result<(), RenderError> {
        // Helper to convert pixel coords to NDC for logging (vertex shader expects NDC).
        fn pixel_to_ndc(px: f32, py: f32, sw: f32, sh: f32) -> [f32; 2] {
            let nx = (px / sw) * 2.0 - 1.0;
            let ny = 1.0 - (py / sh) * 2.0;
            [nx, ny]
        }

        let base_index = verts.len() as u16;
        let mut glyph_count = 0usize;
        let mut first_glyph_logged = false;
        let log_interesting_string = text.contains("Zaroxi Studio") || text.contains("Explorer");

        for ch in text.chars() {
            let glyph = self.font_atlas.glyphs.get(&ch);
            if glyph.is_none() {
                // skip unknown glyphs
                continue;
            }
            let g = glyph.unwrap();
            if g.width == 0 || g.height == 0 {
                x += g.advance;
                glyph_count += 1;
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

            // For the very first glyph of the first call that produced text vertices,
            // log the pre-NDC and computed NDC coords for the four quad vertices.
            if !first_glyph_logged {
                let ndc_a = pixel_to_ndc(x0, y0, screen_w, screen_h);
                let ndc_b = pixel_to_ndc(x1, y0, screen_w, screen_h);
                let ndc_c = pixel_to_ndc(x1, y1, screen_w, screen_h);
                let ndc_d = pixel_to_ndc(x0, y1, screen_w, screen_h);
                info!("emit_text first glyph '{}' quad pixels = [({},{}), ({},{}), ({},{}), ({},{})]", ch, x0, y0, x1, y0, x1, y1, x0, y1);
                info!("emit_text first glyph '{}' quad NDC    = [({:.4},{:.4}), ({:.4},{:.4}), ({:.4},{:.4}), ({:.4},{:.4})]", ch, ndc_a[0], ndc_a[1], ndc_b[0], ndc_b[1], ndc_c[0], ndc_c[1], ndc_d[0], ndc_d[1]);
                info!("emit_text first glyph '{}' uv = [({},{}), ({},{})]", ch, u0, v0, u1, v1);
                info!("emit_text first glyph '{}' color rgba = {:?}", ch, color);
                first_glyph_logged = true;
            }

            // If this call is for one of the interesting strings, log placement for the first few glyphs.
            if log_interesting_string && glyph_count < 6 {
                info!(
                    "glyph debug: text='{}' char='{}' idx={} uv_rect=({:.4},{:.4})-({:.4},{:.4}) screen_rect=({:.1},{:.1})-({:.1},{:.1}) advance={:.3}",
                    text,
                    ch,
                    glyph_count,
                    u0,
                    v0,
                    u1,
                    v1,
                    x0,
                    y0,
                    x1,
                    y1,
                    g.advance
                );
            }

            let a = Vertex { pos: [x0, y0], uv: [u0, v0], color };
            let b = Vertex { pos: [x1, y0], uv: [u1, v0], color };
            let c = Vertex { pos: [x1, y1], uv: [u1, v1], color };
            let d = Vertex { pos: [x0, y1], uv: [u0, v1], color };

            verts.push(a); verts.push(b); verts.push(c); verts.push(d);
            let i0 = base_index + (verts.len() as u16 - 4);
            indices.extend_from_slice(&[i0, i0+1, i0+2, i0, i0+2, i0+3]);

            x += g.advance;
            glyph_count += 1;
        }
        Ok(())
    }
}
