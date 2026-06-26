use crate::error::RenderError;
use log::{debug, info};
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

static GUI_TEXT_FRAME_COUNTER: AtomicUsize = AtomicUsize::new(0);
static GPU_FRAME_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn gpu_trace_enabled() -> bool {
    std::env::var("ZAROXI_RENDER_TRACE").as_deref() == Ok("1")
}
use wgpu::{
    Backends, BindGroup, BindGroupLayout, Buffer, Color, CommandEncoderDescriptor,
    CompositeAlphaMode, Device, DeviceDescriptor, Extent3d, Features, Instance, InstanceDescriptor,
    Limits, PresentMode, Queue, RequestAdapterOptions, SamplerDescriptor, Surface,
    SurfaceConfiguration, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

// AppState is an interface-side type. Provide a minimal local stub here so the
// render crate can compile without pulling the full interface-app crate and to
// avoid accidental cyclic dependencies during incremental development.
// The real AppState lives in the interface/application layer and should be
// passed by callers when available; the renderer never inspects this stub.
#[allow(dead_code)]
pub struct AppState;

use crate::renderer::debug::{
    DISABLE_TEXT_PASS, FIRST_GLYPH_LOGGED, FORCE_MAGENTA_SIDEBAR, LOGGED_EDITOR, LOGGED_SIDEBAR,
    LOGGED_SIDEBAR_PACKED, LOGGED_TITLEBAR, RENDER_DEBUG, TEXT_SAMPLER_NEAREST, init_debug_flags,
    render_debug_enabled, render_timing_enabled, text_pass_disabled, validation_scene_enabled,
};
use crate::renderer::geometry::{Vertex, pixel_to_ndc, push_colored_quad};
use zaroxi_core_engine_font::load_bundled_monospace;
use zaroxi_core_engine_text::plain::layout_plain_lines;

/// Internal context that groups per-frame geometry buffers and screen size.
/// Introduced to reduce the responsibility surface of core.rs and to provide
/// a single place to extend frame-related helpers in subsequent refactors.
///
/// This is a move-free, behavior-preserving helper: it does not change any
/// public API or rendering logic.
struct FrameContext<'a> {
    pub screen_w: f32,
    pub screen_h: f32,
    pub panel_verts: &'a mut Vec<Vertex>,
    pub panel_indices: &'a mut Vec<u16>,
    pub text_verts: &'a mut Vec<Vertex>,
    pub text_indices: &'a mut Vec<u16>,
}

impl<'a> FrameContext<'a> {
    pub fn new(
        screen_w: f32,
        screen_h: f32,
        panel_verts: &'a mut Vec<Vertex>,
        panel_indices: &'a mut Vec<u16>,
        text_verts: &'a mut Vec<Vertex>,
        text_indices: &'a mut Vec<u16>,
    ) -> Self {
        Self { screen_w, screen_h, panel_verts, panel_indices, text_verts, text_indices }
    }

    /// Convenience wrapper delegating to the shared geometry helper.
    pub fn push_colored_quad(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
        corner_radius: f32,
    ) {
        crate::renderer::geometry::push_colored_quad(
            self.panel_verts,
            self.panel_indices,
            x,
            y,
            w,
            h,
            color,
            self.screen_w,
            self.screen_h,
            corner_radius,
        );
    }
}

/// Simple rectangle used by the resolved layout.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Minimal panel colors consumed by the renderer.
///
/// The renderer owns this struct; callers convert their theme data into
/// `PanelColors` before constructing a `RenderLayout`. This keeps the render
/// crate free of interface-layer theme dependencies.
#[derive(Debug, Clone, Copy)]
pub struct PanelColors {
    pub panel_header_background: [f32; 4],
    pub panel_background: [f32; 4],
    pub editor_cursor: [f32; 4],
    pub editor_selection: [f32; 4],
    pub editor_line_highlight: [f32; 4],
    pub text_default: [f32; 4],
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
    pub colors: PanelColors,
}

/* Vertex type and vertex-layout helpers moved to renderer/geometry.rs */

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
    // text subsystem renderer (CosmicText)
    text_renderer: Box<dyn crate::renderer::text::TextRenderer + Send + Sync>,

    // vertex/index buffers reused each frame
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,

    /// Frame render start time (set when ZAROXI_RENDER_TIMING=1).
    frame_start: Option<std::time::Instant>,
}

/// Persistent renderer state that can survive across frames.
///
/// Owns `Arc<Window>` (shared with `ZaroxiWindow`), which enables safe creation
/// of a `Surface<'static>` without unsafe transmute. The surface, GPU device,
/// pipelines, and text-renderer atlas/caches all persist across frames — only
/// the per-frame texture, view, and encoder are created fresh each redraw.
pub struct RenderCore {
    _window: Arc<Window>,
    _instance: Instance,
    _adapter: wgpu::Adapter,
    device: Device,
    queue: Queue,
    clear_color: Color,

    text_pipeline: Option<wgpu::RenderPipeline>,
    text_bind_layout: Option<BindGroupLayout>,
    debug_pipeline: Option<wgpu::RenderPipeline>,
    shape_pipeline: Option<wgpu::RenderPipeline>,
    text_renderer: Option<Box<dyn crate::renderer::text::TextRenderer + Send + Sync>>,
    initialized_format: Option<TextureFormat>,

    /// Persistent surface created from the shared `Arc<Window>`.
    /// Reconfigured on resize; never destroyed until RenderCore is dropped.
    surface: Surface<'static>,
    surface_config: SurfaceConfiguration,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    /// Optional vello cockpit overlay (lazily created once a cockpit scene is set).
    vello_overlay: Option<crate::renderer::vello_overlay::VelloOverlay>,
    /// Cockpit vello scene to composite on the next frame (set by the host).
    cockpit_scene: Option<vello::Scene>,
    /// Cockpit text runs (drawn by the cosmic-text pass before the vello
    /// overlay), set by the host. These sit behind cockpit shape visuals.
    cockpit_text: Vec<CockpitText>,
    /// Overlay text runs (drawn by the cosmic-text pass AFTER the vello
    /// overlay). Popup menu option labels go here so they sit on top of
    /// popup backgrounds, selection highlights, etc.
    cockpit_overlay_text: Vec<CockpitText>,
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
            .request_device(&DeviceDescriptor {
                label: Some("zaroxi-engine-device"),
                required_features,
                required_limits,
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError::Other(format!("request_device failed: {:?}", e)))?;

        let size = window.inner_size();
        // Configure the surface (moved to renderer::surface)
        let config =
            crate::renderer::surface::configure_surface(&surface, &adapter, &device, size)?;

        // Create pipelines & bind group layouts (moved to renderer::pipelines).
        let (text_bind_layout, _text_pipeline, debug_pipeline, shape_pipeline) =
            crate::renderer::pipelines::create_pipelines(&device, &config)?;

        // Initialize the text subsystem. We now use the Cosmic Text–backed
        // TextRenderer implementation located in renderer::text::cosmic.rs.
        // The new CosmicTextRenderer is the single authoritative text renderer
        // for GUI text; the CosmicText renderer is the exclusive text path.
        let font_size = 14.0f32;
        let text_renderer: Box<dyn crate::renderer::text::TextRenderer + Send + Sync> =
            Box::new(crate::renderer::text::CosmicTextRenderer::new(
                &device,
                &queue,
                config.format,
                font_size,
                &text_bind_layout,
            )?);

        // Inform the text renderer of the initial viewport so the fallback render_pass path
        // can observe non-zero target dimensions for the first frame.
        let _ = text_renderer.resize_viewport(config.width, config.height);

        // Create a simple shader for textured text (WGSL).
        // Diagnostic: record which WGSL source is being compiled into the pipeline.
        // This helps prove whether the shader file edited during debugging is the
        // one actually used at runtime.
        let mut shader_src = include_str!("../text_shader.wgsl").to_string();
        debug!(
            "TEXT PIPELINE BUILD: shader=crates/zaroxi-engine-render/src/text_shader.wgsl len={} bytes",
            shader_src.len()
        );
        if std::env::var("ZAROXI_TEXT_SOLID_QUADS").map(|v| v == "1").unwrap_or(false) {
            shader_src = shader_src.replace(
                "const DIAGNOSTIC_MAGENTA: bool = false;",
                "const DIAGNOSTIC_MAGENTA: bool = true;",
            );
            info!("TEXT SHADER: DIAGNOSTIC_MAGENTA forced ON via ZAROXI_TEXT_SOLID_QUADS");
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text-shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
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
                buffers: &[crate::renderer::text_pipeline::instance_buffer_layout()],
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
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
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
            let expected_vertex_size = size_of::<Vertex>(); // pos(8) + uv(8) + color(16) + corner_radius(4) + _pad(4) = 40 bytes

            debug!("text pipeline created: color_format={:?}, blend=ALPHA_BLENDING", config.format);
            debug!("Vertex struct: size_of::<Vertex>() = {}", vertex_size);
            debug!("Vertex buffer layout (Rust -> WGSL):");
            debug!("  - @location(0) pos           : Float32x2  @ offset 0");
            debug!("  - @location(1) uv            : Float32x2  @ offset 8");
            debug!("  - @location(2) color         : Float32x4 @ offset 16");
            debug!("  - @location(3) corner_radius : Float32   @ offset 32");
            debug!("  - array_stride = {} (bytes)", vertex_size);

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
        // Debug pipeline creation moved to renderer::pipelines

        // Shape pipeline: dedicated minimal solid-color pipeline used for all
        // non-text UI geometry (panels, borders, dividers). This avoids sampling
        // the font atlas or relying on text bind groups for simple colored quads.
        // Shape pipeline creation moved to renderer::pipelines

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

        // Initialize runtime debug flags from env vars.
        init_debug_flags();

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
            text_renderer,
            vertex_buffer,
            index_buffer,
            index_count: 0,
            frame_start: None,
        })
    }

    /// Resize and reconfigure the surface.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) -> Result<(), RenderError> {
        if new_size.width == 0 || new_size.height == 0 {
            return Ok(());
        }
        self.size = new_size;
        // Delegate resize/reconfigure to renderer::surface (move-only refactor).
        crate::renderer::surface::resize_surface(
            &self.surface,
            &self.device,
            &mut self.config,
            new_size,
        )?;
        debug!("Reconfigured surface to {}x{}", self.config.width, self.config.height);
        // Update text renderer viewport so render_pass bridge observes fresh target dims.
        let _ = self.text_renderer.resize_viewport(self.config.width, self.config.height);
        Ok(())
    }

    /// Reconfigure surface after Lost/Outdated.
    pub fn reconfigure(&mut self) -> Result<(), RenderError> {
        crate::renderer::surface::reconfigure_surface(&self.surface, &self.device, &self.config)
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
    /// The renderer expects generic UiBlock descriptors (rect + visual hints)
    /// rather than interpreting application-specific identifiers.
    pub fn render_with_layout(
        &mut self,
        _app_state: &AppState,
        layout: &RenderLayout,
        render_blocks: &[crate::UiBlock],
    ) -> Result<(), RenderError> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }
        debug!(
            "entering render_with_layout (window {}x{}), render_blocks={}",
            self.config.width,
            self.config.height,
            render_blocks.len()
        );

        // Runtime trace: emit the concrete type name of the text_renderer trait
        // object so we can prove at runtime whether CosmicTextRenderer is selected.
        // `type_name_of_val` on the trait object will yield the concrete type name.
        let tr_type = std::any::type_name_of_val(&*self.text_renderer);
        info!("GUI_SHELL_TRACE: selected_text_renderer = {}", tr_type);

        // Log received render panels for traceability (debug only).
        if RENDER_DEBUG {
            for p in render_blocks.iter() {
                debug!(
                    "renderer received render_panel id='{}' title='{}' visible={} content_len={} spans={}",
                    p.id,
                    p.title,
                    p.visible,
                    p.content.len(),
                    p.content_spans.as_ref().map(|s| s.len()).unwrap_or(0)
                );
            }
        }

        if render_timing_enabled() {
            self.frame_start = Some(std::time::Instant::now());
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
        // Removed local per-frame diagnostic knobs (DEBUG_RENDER / DIAGNOSTIC_*)
        // to reduce comment-only scaffolding. Runtime diagnostic gating remains
        // available via `renderer::debug::render_debug_enabled()` and the
        // cross-crate flags in `renderer::debug` for observable markers.

        let sem = &layout.colors;

        // Build separate lists for shape (panel) geometry and text geometry so we
        // can render them with different pipelines:
        //  - panel_verts / panel_indices -> drawn with shape_pipeline
        //  - text_verts  / text_indices  -> drawn with text_pipeline (font sampling)
        let mut panel_verts: Vec<Vertex> = Vec::new();
        let mut panel_indices: Vec<u16> = Vec::new();
        let mut text_verts: Vec<Vertex> = Vec::new();
        let mut text_indices: Vec<u16> = Vec::new();

        // quad helper moved to renderer::geometry::push_colored_quad

        // Convert render blocks into visible content quads and text.
        if render_debug_enabled() {
            log::debug!("[renderer] render_blocks count = {}", render_blocks.len());
        }

        // VALIDATION SCENE: when enabled inject three large horizontal bands (R/G/B)
        // at the top of the shape list to validate the shape pipeline end-to-end.
        if validation_scene_enabled() {
            // three equal-height horizontal bands covering the full width.
            let band_h = (height as f32) / 3.0;
            // Top band - red
            push_colored_quad(
                &mut panel_verts,
                &mut panel_indices,
                0.0,
                0.0,
                width as f32,
                band_h,
                [1.0, 0.0, 0.0, 1.0],
                width,
                height,
                0.0,
            );
            // Middle band - green
            push_colored_quad(
                &mut panel_verts,
                &mut panel_indices,
                0.0,
                band_h,
                width as f32,
                band_h,
                [0.0, 1.0, 0.0, 1.0],
                width,
                height,
                0.0,
            );
            // Bottom band - blue
            push_colored_quad(
                &mut panel_verts,
                &mut panel_indices,
                0.0,
                band_h * 2.0,
                width as f32,
                band_h,
                [0.0, 0.0, 1.0, 1.0],
                width,
                height,
                0.0,
            );
        }

        // For each panel supplied by the app, create a header and content block and queue title/content text.
        let header_h = 28.0f32;
        let content_padding = 8.0f32;
        for block in render_blocks.iter() {
            if RENDER_DEBUG {
                debug!(
                    "drawing block id='{}' title='{}' visible={}",
                    block.id, block.title, block.visible
                );
            }
            if !block.visible {
                if RENDER_DEBUG {
                    debug!("block '{}' is hidden; skipping", block.id);
                }
                continue;
            }

            // Use block-provided target rect and visuals (app/runtime is responsible
            // for mapping ids -> rects and selecting colors).
            let target = block.rect;

            // Log scissor/rect that would be used for this block (diagnostic).
            if render_debug_enabled() {
                log::debug!("block '{}' scissor_rect = {:?}", block.id, target);
            }

            // Delegate header/content quad generation to the shapes module.
            // The shapes::queue_panel_quads helper now accepts a generic UiBlock
            // and uses the provided visual hints (header/content colors).
            let base_idx_opt = crate::renderer::shapes::queue_panel_quads(
                &mut panel_verts,
                &mut panel_indices,
                block,
                &sem,
                width,
                height,
            );

            // One-shot packed-vertex dump for the first content quad if present.
            if let Some(base_idx) = base_idx_opt {
                if panel_verts.len() >= base_idx + 4 {
                    let v0 = panel_verts[base_idx];
                    let v1 = panel_verts[base_idx + 1];
                    let v2 = panel_verts[base_idx + 2];
                    let v3 = panel_verts[base_idx + 3];
                    debug!(
                        "packed block verts: \
                         v0 pos=({:.3},{:.3}) uv=({:.3},{:.3}) color=({:.3},{:.3},{:.3},{:.3}); \
                         v1 pos=({:.3},{:.3}) uv=({:.3},{:.3}) color=({:.3},{:.3},{:.3},{:.3}); \
                         v2 pos=({:.3},{:.3}) uv=({:.3},{:.3}) color=({:.3},{:.3},{:.3},{:.3}); \
                         v3 pos=({:.3},{:.3}) uv=({:.3},{:.3}) color=({:.3},{:.3},{:.3},{:.3})",
                        v0.pos[0],
                        v0.pos[1],
                        v0.uv[0],
                        v0.uv[1],
                        v0.color[0],
                        v0.color[1],
                        v0.color[2],
                        v0.color[3],
                        v1.pos[0],
                        v1.pos[1],
                        v1.uv[0],
                        v1.uv[1],
                        v1.color[0],
                        v1.color[1],
                        v1.color[2],
                        v1.color[3],
                        v2.pos[0],
                        v2.pos[1],
                        v2.uv[0],
                        v2.uv[1],
                        v2.color[0],
                        v2.color[1],
                        v2.color[2],
                        v2.color[3],
                        v3.pos[0],
                        v3.pos[1],
                        v3.uv[0],
                        v3.uv[1],
                        v3.color[0],
                        v3.color[1],
                        v3.color[2],
                        v3.color[3],
                    );
                }
            }

            // Queue header/title text
            // Compute explicit header rect and emit title clipped to that region to ensure
            // ownership/clip correctness (Step 1: per-block text clipping).
            let hx = target.x;
            let hy = target.y;
            let hw = target.w;
            let hh = header_h.min(target.h.max(0.0));
            // Title placement: left padding and vertical centering within header.
            // Use a conservative default font size for centering to match backend font size (14px).
            const DEFAULT_FONT_SIZE: f32 = 14.0;
            let title_x = target.x + 8.0;
            let title_y = target.y + (hh - DEFAULT_FONT_SIZE) * 0.5;
            // Title color: from block hint or default white
            let title_color: [f32; 4] = block.text_color.unwrap_or(layout.colors.text_default);

            // Layout title text into glyph placements and convert to vertices/indices.
            // Use the pluggable text backend so shaping/layout logic is isolated from
            // the renderer. The backend may consult the font atlas internally.
            // Ask the text backend to lay out and ensure glyphs are rasterized/available
            // in the backend-managed atlas. The backend may use the provided queue to
            // upload missing glyph bitmaps into its internal atlas before returning
            // placed glyphs with valid UVs.
            // Queue title for the CosmicText text renderer (prepare/render will occur later).
            // Status strip: a header-only block whose `content_spans` carries the
            // right-aligned segments. Render the title left-aligned and those
            // segments right-aligned, with responsive collapse/truncation so the
            // two groups never overlap (no manual space padding for alignment).
            if block.header_only && block.content_spans.is_some() {
                let pad = 8.0f32;
                let device_scale: f32 = std::env::var("ZAROXI_SURFACE_SCALE")
                    .ok()
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1.0);
                // Status text shapes in the bundled monospace font, so per-char
                // advance gives an exact width in the title's logical space. Clamp
                // to a plausible range so a bad metric can't collapse the layout.
                let advance = self
                    .text_renderer
                    .monospace_advance_x()
                    .map(|a| a / device_scale.max(0.01))
                    .filter(|a| a.is_finite() && *a > 1.0 && *a < DEFAULT_FONT_SIZE * 2.0)
                    .unwrap_or(DEFAULT_FONT_SIZE * 0.6);
                let gap = (advance * 2.0).max(12.0);
                let right_segments: Vec<String> = block
                    .content_spans
                    .as_ref()
                    .map(|s| s.iter().map(|(t, _)| t.clone()).collect())
                    .unwrap_or_default();
                let runs = crate::renderer::header_layout::plan_status_header(
                    &block.title,
                    &right_segments,
                    hx,
                    hw,
                    pad,
                    advance,
                    gap,
                );

                let status_render_debug =
                    std::env::var("ZAROXI_STATUS_RENDER_DEBUG").as_deref() == Ok("1");
                if status_render_debug {
                    eprintln!(
                        "ZAROXI_STATUS_RENDER_DEBUG[core:legacy]: block='{}' rect=(x={:.0} w={:.0}) advance={:.2} gap={:.1} title={:?} right_segs={:?} runs={}",
                        block.id,
                        hx,
                        hw,
                        advance,
                        gap,
                        block.title,
                        right_segments,
                        runs.len()
                    );
                    for (i, r) in runs.iter().enumerate() {
                        eprintln!(
                            "ZAROXI_STATUS_RENDER_DEBUG[core:legacy]:   run[{}] text={:?} x={:.1} y={:.1} clip=(x={:.1} w={:.1})",
                            i, r.text, r.x, title_y, r.clip_x, r.clip_w
                        );
                    }
                }

                for run in &runs {
                    self.text_renderer.queue_text(crate::renderer::text::TextCommand::new_title(
                        &run.text,
                        run.x,
                        title_y,
                        title_color,
                        DEFAULT_FONT_SIZE,
                        run.clip_x,
                        hy,
                        run.clip_w,
                        hh,
                    ));
                }
            } else {
                self.text_renderer.queue_text(crate::renderer::text::TextCommand::new_title(
                    &block.title,
                    title_x,
                    title_y,
                    title_color,
                    DEFAULT_FONT_SIZE,
                    hx,
                    hy,
                    hw,
                    hh,
                ));
            }
            // Emit core queue-stage trace: what the core queued for the text backend.
            let queued_after = self.text_renderer.queued_len();
            info!(
                "GUI_TEXT_FRAME_SUMMARY: block_id='{}' title=\"{}\" queued_after={}",
                block.id, block.title, queued_after
            );
            // Concise info for queued title (no per-glyph spam).
            debug!("queued title for block='{}'", block.id);

            // Body/content text emission:
            // - The renderer is intentionally domain-agnostic. It renders whatever
            //   `block.content` the application supplies. Decisions about filtering,
            //   placeholder text, or app-specific suppression belong in the app layer.
            // - The renderer will only skip emission when there is no content to draw,
            //   with one structural exception: the titlebar and status bar are header-only
            //   regions in the UI and should not render generic body content supplied to
            //   their block.content slots by mistake. Those two regions are considered
            //   structurally header-only at the engine layer.
            let content = block.content.trim();

            // Structural-only suppression for well-known header regions:
            // - titlebar is header-only and must not render block.content.
            let is_titlebar =
                block.id == "titlebar" || block.id == "title_bar" || block.id == "title-bar";

            if is_titlebar {
                if RENDER_DEBUG && !content.is_empty() {
                    debug!(
                        "emit_text: skipping body content for structural header block='{}'",
                        block.id
                    );
                }
            } else if !content.is_empty() {
                // Emit content into the block's content area using the provided rect.
                // When clip_rect is set, use its x/w for horizontal containment
                // (prevents text bleed into adjacent panels). Vertical bounds (y/h)
                // remain header-aware to prevent overlap with the block's title area.
                // Full text is queued — CosmicText clips via per-glyph bounds check
                // during prepare. No source truncation is performed.
                let content_y = target.y + hh + content_padding;
                let content_h = (target.h - hh - content_padding * 2.0).max(0.0);
                let (text_x, clip_x, clip_w) = if let Some(ref clip) = block.clip_rect {
                    let tx = clip.x - block.content_offset_x;
                    (tx, clip.x, clip.w)
                } else {
                    let cx = target.x + content_padding;
                    let cw = (target.w - content_padding * 2.0).max(0.0);
                    (cx, cx, cw)
                };
                // Vertical scroll: shift text origin up by content_offset_y
                let text_y = content_y - block.content_offset_y;
                if clip_w > 0.0 && content_h > 0.0 {
                    // If per-span colored content is provided, emit each span as a
                    // separate text command with its own color for syntax highlighting.
                    if let Some(ref spans) = block.content_spans {
                        let line_h = DEFAULT_FONT_SIZE + 2.0;
                        let clip_bottom = content_y + content_h;
                        // Honor content_line_offset symmetrically with the plain
                        // path so a viewport-windowed span list lands at the
                        // correct absolute y (content_offset_y applies scroll).
                        let mut cursor_y =
                            text_y + block.content_line_offset.unwrap_or(0) as f32 * line_h;
                        // Fast-forward through spans for lines entirely above the clip area.
                        let mut ff_y = cursor_y;
                        let mut ff_idx: usize = 0;
                        while ff_y < content_y && ff_idx < spans.len() {
                            if spans[ff_idx].0 == "\n" {
                                ff_y += line_h;
                            }
                            ff_idx += 1;
                        }
                        let effective_spans = if ff_idx > 0 {
                            cursor_y = ff_y;
                            &spans[ff_idx..]
                        } else {
                            spans.as_slice()
                        };
                        // Accumulate each line's colored runs and shape the whole
                        // line as ONE continuous buffer (per-run colors), keeping
                        // normal editor-text layout. Only fully-visible lines are
                        // drawn (avoids glyph-edge artifacts at the boundaries).
                        let mut line_runs: Vec<(String, [f32; 4])> = Vec::new();
                        for (span_text, span_color) in effective_spans {
                            if span_text == "\n" {
                                let line_visible =
                                    cursor_y >= content_y && cursor_y + line_h <= clip_bottom;
                                if line_visible && !line_runs.is_empty() {
                                    self.text_renderer.queue_text(
                                        crate::renderer::text::TextCommand::new_body_runs(
                                            std::mem::take(&mut line_runs),
                                            text_x,
                                            cursor_y,
                                            DEFAULT_FONT_SIZE,
                                            clip_x,
                                            content_y,
                                            clip_w,
                                            content_h,
                                        ),
                                    );
                                } else {
                                    line_runs.clear();
                                }
                                cursor_y += line_h;
                                if cursor_y + line_h > clip_bottom {
                                    break;
                                }
                                continue;
                            }
                            if !span_text.is_empty() {
                                line_runs.push((span_text.clone(), *span_color));
                            }
                        }
                        let last_visible =
                            cursor_y >= content_y && cursor_y + line_h <= clip_bottom;
                        if last_visible && !line_runs.is_empty() {
                            self.text_renderer.queue_text(
                                crate::renderer::text::TextCommand::new_body_runs(
                                    line_runs,
                                    text_x,
                                    cursor_y,
                                    DEFAULT_FONT_SIZE,
                                    clip_x,
                                    content_y,
                                    clip_w,
                                    content_h,
                                ),
                            );
                        }
                    } else {
                        // Non-spans path: apply line-level vertical culling
                        // (matches the spans-path behaviour).  clip_y uses
                        // content_y so that per-glyph top-edge culling is
                        // effective.  Only lines fully within the visible
                        // clip area are queued — partial edge rows are
                        // skipped to prevent glyph clipping artifacts.
                        //
                        // Viewport-only rendering: when content_line_offset is
                        // set, `content` carries only the visible window of
                        // lines (plus overscan). The offset adjusts cursor_y
                        // so the first line in `content` starts at the correct
                        // absolute y-position, matching scroll and cursor
                        // positioning computed from absolute line numbers.
                        let clip_bottom = content_y + content_h;
                        let line_h = DEFAULT_FONT_SIZE + 2.0;
                        let mut cursor_y =
                            text_y + block.content_line_offset.unwrap_or(0) as f32 * line_h;
                        let visible_line_start = block.content_line_offset.unwrap_or(0);
                        if std::env::var("ZAROXI_DEBUG_RENDER_WINDOW").as_deref() == Ok("1") {
                            let content_byte_count = block.content.len();
                            let line_count = block.content.lines().count();
                            eprintln!(
                                "ZAROXI_DEBUG_RENDER_WINDOW: block={} clip_y={:.1} clip_bottom={:.1} line_start={} content_bytes={} content_lines={}",
                                block.id,
                                content_y,
                                clip_bottom,
                                visible_line_start,
                                content_byte_count,
                                line_count,
                            );
                        }
                        for line_str in block.content.lines() {
                            if cursor_y + line_h > clip_bottom {
                                break;
                            }
                            if cursor_y >= content_y {
                                self.text_renderer.queue_text(
                                    crate::renderer::text::TextCommand::new_body(
                                        line_str,
                                        text_x,
                                        cursor_y,
                                        title_color,
                                        DEFAULT_FONT_SIZE,
                                        clip_x,
                                        content_y,
                                        clip_w,
                                        content_h,
                                    ),
                                );
                            }
                            cursor_y += line_h;
                        }
                    }
                    debug!("queued content for block='{}'", block.id);
                } else if RENDER_DEBUG {
                    info!("emit_text: content area too small for block='{}'", block.id);
                }
            } else if RENDER_DEBUG {
                debug!("emit_text: no content for block='{}'", block.id);
            }

            // ── Cursor & line-highlight rendering ──
            if let (Some(line), Some(col)) = (block.cursor_line, block.cursor_col) {
                let content_x = if let Some(ref clip) = block.clip_rect {
                    clip.x - block.content_offset_x
                } else {
                    target.x + content_padding
                };
                let content_w = if let Some(ref clip) = block.clip_rect {
                    clip.w
                } else {
                    (target.w - content_padding * 2.0).max(0.0)
                };
                let content_y = target.y + hh + content_padding;
                let content_h = (target.h - hh - content_padding * 2.0).max(0.0);
                // Use the actual monospace glyph advance from the font system
                // for cursor and selection positioning. The advance is in
                // physical pixels (same coordinate space as block rects and
                // glyph instance positions), so the caret aligns with shaped text.
                let char_w = self.text_renderer.monospace_advance_x().unwrap_or(8.0);

                if content_w > 0.0 && content_h > 0.0 {
                    let line_h = DEFAULT_FONT_SIZE + 2.0;
                    let text_y = content_y as f32 - block.content_offset_y;
                    let line_y = text_y + line as f32 * line_h;

                    // Active line highlight background — clipped to content area
                    if block.highlight_active_line
                        && line_y >= content_y
                        && line_y + line_h <= content_y + content_h
                    {
                        let hl_color: [f32; 4] = layout.colors.editor_line_highlight;
                        push_colored_quad(
                            &mut panel_verts,
                            &mut panel_indices,
                            content_x,
                            line_y,
                            content_w,
                            line_h,
                            hl_color,
                            width,
                            height,
                            0.0,
                        );
                    }

                    // Cursor vertical bar — positioned using the actual monospace
                    // glyph advance from the font system, not a hardcoded 8.0 px stub.
                    let cursor_x = content_x + col as f32 * char_w;
                    let cursor_w = 2.0;
                    let cursor_h = line_h;
                    if cursor_x >= content_x
                        && cursor_x + cursor_w <= content_x + content_w
                        && line_y >= content_y
                        && line_y + cursor_h <= content_y + content_h
                    {
                        let cursor_color: [f32; 4] = layout.colors.editor_cursor;
                        push_colored_quad(
                            &mut panel_verts,
                            &mut panel_indices,
                            cursor_x,
                            line_y,
                            cursor_w,
                            cursor_h,
                            cursor_color,
                            width,
                            height,
                            0.0,
                        );
                    }
                }

                // Selection highlight — uses the same char_w computed from the
                // font system above (cursor vertical bar block).
                if let Some((sl, sc, el, ec)) = block.selection_range {
                    let line_h = DEFAULT_FONT_SIZE + 2.0;
                    let text_y = content_y - block.content_offset_y;
                    let sel_color: [f32; 4] = layout.colors.editor_selection;
                    for line in sl..=el {
                        let line_y = text_y + line as f32 * line_h;
                        if line_y + line_h <= content_y {
                            continue;
                        }
                        if line_y + line_h > content_y + content_h {
                            break;
                        }
                        let start_col = if line == sl { sc } else { 0 };
                        let end_col = if line == el { ec } else { 200 }; // generous max
                        let sel_x = content_x + start_col as f32 * char_w;
                        let sel_w = ((end_col.saturating_sub(start_col)) as f32 * char_w)
                            .min(content_w - (sel_x - content_x));
                        if sel_w > 0.0 {
                            push_colored_quad(
                                &mut panel_verts,
                                &mut panel_indices,
                                sel_x,
                                line_y,
                                sel_w,
                                line_h,
                                sel_color,
                                width,
                                height,
                                0.0,
                            );
                        }
                    }
                }
            }

            // Log counts per panel
            if RENDER_DEBUG {
                let quad_count = (panel_verts.len() / 4) as usize;
                debug!(
                    "block '{}' queued: panel_quads={} panel_verts={} panel_indices={} text_verts={} text_indices={}",
                    block.id,
                    quad_count,
                    panel_verts.len(),
                    panel_indices.len(),
                    text_verts.len(),
                    text_indices.len()
                );
            }
        }

        // Optional diagnostic: inject a small centered diagnostic text quad directly
        // into the text geometry (NDC quad). This bypasses atlas sampling to verify
        // text pipeline/vertex mapping independently.

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

        // Temporary diagnostics: log geometry counts to help diagnose missing text.
        // These logs are intentionally concise and should be safe in normal runs.
        let panel_indices_len = panel_indices.len() as u32;
        let total_indices_len = indices.len() as u32;
        debug!(
            "render geometry counts: panel_verts={} text_verts={} panel_indices={} text_indices={} total_verts={} total_indices={}",
            panel_vertex_count as usize,
            verts.len().saturating_sub(panel_vertex_count as usize),
            panel_indices_len,
            total_indices_len.saturating_sub(panel_indices_len),
            verts.len(),
            total_indices_len
        );

        if std::env::var("ZAROXI_DEBUG_RENDER_WINDOW").as_deref() == Ok("1") {
            let glyph_count = verts.len() / 4;
            let quad_count = glyph_count;
            eprintln!(
                "ZAROXI_DEBUG_RENDER_WINDOW: final glyph_instances={} panel_quads={} text_quads={}",
                quad_count,
                panel_vertex_count as usize / 4,
                (verts.len() - panel_vertex_count as usize) / 4,
            );
        }

        // Log final totals (debug only to avoid frame spam)
        if RENDER_DEBUG {
            debug!("[renderer] final verts={}, indices={}", verts.len(), indices.len());
        }

        // Warn if vertex positions appear to be outside expected NDC range.
        // Many vertices should already be in NDC (shape quads converted on CPU),
        // while some text verts may still be in pixel coordinates — use a loose
        // detection to highlight clearly out-of-bounds values.
        let mut oob_count = 0usize;
        for (_i, v) in verts.iter().enumerate() {
            if v.pos[0].abs() > 1.05 || v.pos[1].abs() > 1.05 {
                oob_count += 1;
            }
        }
        if render_debug_enabled() {
            if oob_count > 0 {
                log::debug!(
                    "vertex OOB summary: total_verts={} out_of_bounds={}",
                    verts.len(),
                    oob_count
                );
            } else {
                log::debug!(
                    "vertex positions all within expected NDC/pixel ranges (no obvious OOB)"
                );
            }
        }

        // Dump first few vertices so we can inspect coordinate space (pos, uv, color).
        if render_debug_enabled() {
            let max_log = std::cmp::min(8usize, verts.len());
            log::debug!("vertex[0..{}] dump:", max_log);
            for i in 0..max_log {
                let v = verts[i];
                log::debug!(
                    "v[{}] pos=({:.4}, {:.4}) uv=({:.4}, {:.4}) color=({:.4}, {:.4}, {:.4}, {:.4})",
                    i,
                    v.pos[0],
                    v.pos[1],
                    v.uv[0],
                    v.uv[1],
                    v.color[0],
                    v.color[1],
                    v.color[2],
                    v.color[3]
                );
            }
        }

        // Upload vertex/index data
        let vb_bytes = bytemuck::cast_slice(&verts);
        self.queue.write_buffer(&self.vertex_buffer, 0, vb_bytes);

        // Log first few indices to validate index buffer contents & format.
        if render_debug_enabled() {
            let max_idx_log = std::cmp::min(12usize, indices.len());
            log::debug!("index[0..{}] dump:", max_idx_log);
            for i in 0..max_idx_log {
                log::debug!("i[{}] = {}", i, indices[i]);
            }
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

                    if render_debug_enabled() {
                        log::debug!("debug pass checked");
                    }

                    // SHAPE PASS: draw only the panel/background geometry using the
                    // dedicated shape_pipeline (no font sampling).
                    let panel_indices_len = panel_indices.len() as u32;
                    let total_indices_len = indices.len() as u32;

                    // Diagnostic: log shape/text split info
                    info!(
                        "render passes: panel_indices_len={} total_indices_len={} panel_verts={} text_verts={}",
                        panel_indices_len,
                        total_indices_len,
                        panel_vertex_count as usize,
                        verts.len().saturating_sub(panel_vertex_count as usize)
                    );

                    if !false {
                        if panel_indices_len > 0 {
                            if render_debug_enabled() {
                                log::debug!(
                                    "shape pass indexed draw (suboptimal path): indices_drawn={}",
                                    panel_indices_len
                                );
                            }
                            // Diagnostic: explicit draw parameters for shape pass
                            debug!(
                                "shape pass draw_indexed: start=0 end={} count={} base_vertex=0",
                                panel_indices_len, panel_indices_len
                            );
                            crate::renderer::shapes::submit_shape_pass(
                                &mut rpass,
                                &self.shape_pipeline,
                                &self.vertex_buffer,
                                &self.index_buffer,
                                panel_indices_len,
                            );
                        }
                    } else {
                        if render_debug_enabled() {
                            log::debug!("false enabled (suboptimal path): skipping shape pass");
                        }
                    }

                    // TEXT PASS: draw glyph/text geometry using the text pipeline and font atlas.
                    // Run text pass if either legacy text indices indicate text geometry OR there are queued native backend commands.
                    // Enriched core-side tracing: compute adapter marker count (if present)
                    // and backend-text index count (derived from index splits).
                    let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
                    let mut adapter_text_ops: u32 = 0;
                    if tmp_layout.exists() {
                        if let Ok(s) = std::fs::read_to_string(&tmp_layout) {
                            if let Some(rest) = s.strip_prefix("lines=") {
                                adapter_text_ops =
                                    rest.split(" | ").filter(|p| !p.is_empty()).count() as u32;
                            }
                        }
                    }
                    let queued_len = self.text_renderer.queued_len();
                    let backend_text_ops = total_indices_len.saturating_sub(panel_indices_len);
                    info!(
                        "GUI_TEXT_FRAME_SUMMARY: panel_indices_len={} total_indices_len={} queued_len={} adapter_text_ops={} backend_text_ops={}",
                        panel_indices_len,
                        total_indices_len,
                        queued_len,
                        adapter_text_ops,
                        backend_text_ops
                    );
                    if total_indices_len > panel_indices_len || self.text_renderer.queued_len() > 0
                    {
                        if text_pass_disabled() {
                            if render_debug_enabled() {
                                log::debug!(
                                    "DISABLE_TEXT_PASS: skipping text pass (would draw {} indices)",
                                    total_indices_len.saturating_sub(panel_indices_len)
                                );
                            }
                        } else {
                            if render_debug_enabled() {
                                log::debug!(
                                    "binding text pipeline and font_atlas bind_group for text pass (queued={})",
                                    self.text_renderer.queued_len()
                                );
                            }

                            // Diagnostic: explicit draw parameters for text pass
                            debug!(
                                "text pass draw_indexed: start={} end={} count={} (panel_indices_len={} total_indices_len={})",
                                panel_indices_len,
                                total_indices_len,
                                total_indices_len.saturating_sub(panel_indices_len),
                                panel_indices_len,
                                total_indices_len
                            );

                            // Ensure CosmicText receives queued commands (if any).
                            let queued_count = self.text_renderer.queued_len();
                            info!("CosmicText: queued commands before prepare: {}", queued_count);

                            // Record intent to call the backend prepare and capture queued size immediately prior.
                            let queued_before_prepare = self.text_renderer.queued_len();
                            info!(
                                "GUI_TEXT_FRAME_SUMMARY: prepare_invoking queued_before_prepare={}",
                                queued_before_prepare
                            );
                            match self.text_renderer.prepare(&self.device, &mut self.queue) {
                                Ok(()) => {
                                    info!("GUI_TEXT_FRAME_SUMMARY: prepare_called=true");
                                }
                                Err(e) => {
                                    info!(
                                        "GUI_TEXT_FRAME_SUMMARY: prepare_called=false error={:?}",
                                        e
                                    );
                                    return Err(e);
                                }
                            }

                            info!(
                                "GUI_TEXT_RENDER_PASS_ACTIVE: render_invoking panel_indices_len={} total_indices_len={}",
                                panel_indices_len, total_indices_len
                            );
                            match self.text_renderer.render_pass(
                                &mut rpass,
                                &self.text_pipeline,
                                panel_indices_len,
                                total_indices_len,
                            ) {
                                Ok(()) => {
                                    info!("GUI_TEXT_RENDER_PASS_ACTIVE: executed=true");
                                }
                                Err(e) => {
                                    info!(
                                        "GUI_TEXT_RENDER_PASS_ACTIVE: executed=false error={:?}",
                                        e
                                    );
                                    return Err(e);
                                }
                            }
                        }
                    }
                }

                // Frame-level summary: gather adapter/backend markers (temp-files) and cosmic prepare markers
                // so we can see per-frame status for the canonical label. This is diagnostic-only and
                // intentionally non-fatal: we must not break presentation.
                let frame_idx = GUI_TEXT_FRAME_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
                let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
                let tmp_cosmic = std::env::temp_dir().join("zaroxi_gui_trace_cosmic_prepare");
                let adapter_present = tmp_layout.exists();
                let cosmic_present = tmp_cosmic.exists();

                // Extended cosmic marker parsing: shaped_glyphs_total, emitted_glyphs_total, font_resolved, buffer_size, text_len, glyph_count, atlas_entries
                let mut glyph_count: usize = 0;
                let mut atlas_entries: usize = 0;
                let mut shaped_glyphs_total: usize = 0;
                let mut emitted_glyphs_total: usize = 0;
                let mut font_resolved: bool = false;
                let mut buffer_size: String = "0x0".to_string();
                let mut text_len: usize = 0;

                if cosmic_present {
                    if let Ok(s) = std::fs::read_to_string(&tmp_cosmic) {
                        for line in s.lines() {
                            if let Some(v) = line.strip_prefix("glyph_count=") {
                                glyph_count = v.parse::<usize>().unwrap_or(0);
                            } else if let Some(v) = line.strip_prefix("atlas_entries=") {
                                atlas_entries = v.parse::<usize>().unwrap_or(0);
                            } else if let Some(v) = line.strip_prefix("shaped_glyphs_total=") {
                                shaped_glyphs_total = v.parse::<usize>().unwrap_or(0);
                            } else if let Some(v) = line.strip_prefix("emitted_glyphs_total=") {
                                emitted_glyphs_total = v.parse::<usize>().unwrap_or(0);
                            } else if let Some(v) = line.strip_prefix("font_resolved=") {
                                font_resolved = v.trim() == "true";
                            } else if let Some(v) = line.strip_prefix("buffer_size=") {
                                buffer_size = v.to_string();
                            } else if let Some(v) = line.strip_prefix("text_len=") {
                                text_len = v.parse::<usize>().unwrap_or(0);
                            }
                        }
                    }
                }

                // Build a compact frame summary combining adapter/backend/core/cosmic status.
                let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
                let mut adapter_text_ops: usize = 0;
                if tmp_layout.exists() {
                    if let Ok(s) = std::fs::read_to_string(&tmp_layout) {
                        if let Some(rest) = s.strip_prefix("lines=") {
                            adapter_text_ops = rest.split(" | ").filter(|p| !p.is_empty()).count();
                        }
                    }
                }
                let backend_text_ops = total_indices_len.saturating_sub(panel_indices_len) as usize;
                let core_text_ops = self.text_renderer.queued_len();
                // cosmic_present computed earlier
                let cosmic_prepare_called = cosmic_present;
                // pipeline_render_called: infer from whether we attempted a text pass (backend_text_ops>0 or core_text_ops>0) and DISABLE_TEXT_PASS flag
                let pipeline_render_called = (!text_pass_disabled())
                    && (backend_text_ops > 0 || core_text_ops > 0)
                    && cosmic_prepare_called;
                // overlay rects marker: read fallback marker if present
                let fallback_marker = std::env::temp_dir().join("zaroxi_gui_trace_fallback");
                let fallback_used =
                    fallback_marker.exists() || (adapter_text_ops > 0 && !cosmic_prepare_called);

                if std::env::var("ZAROXI_DEBUG_TEXT_FRAME").as_deref() == Ok("1") {
                    info!(
                        "GUI_TEXT_FRAME_SUMMARY: frame={} adapter_text_ops={} backend_text_ops={} core_text_ops={} cosmic_prepare_called={} shaped_glyphs_total={} emitted_glyphs_total={} glyphs={} atlas_entries={} font_resolved={} buffer_size={} text_len={} pipeline_render_called={} overlay_rects={} fallback_used={}",
                        frame_idx,
                        adapter_text_ops,
                        backend_text_ops,
                        core_text_ops,
                        if cosmic_prepare_called { "true" } else { "false" },
                        shaped_glyphs_total,
                        emitted_glyphs_total,
                        glyph_count,
                        atlas_entries,
                        if font_resolved { "true" } else { "false" },
                        buffer_size,
                        text_len,
                        if pipeline_render_called { "true" } else { "false" },
                        backend_text_ops,
                        if fallback_used { "true" } else { "false" }
                    );
                }

                // Hard-checks: diagnose broken links between stages.
                if adapter_text_ops > 0 && backend_text_ops == 0 {
                    info!("GUI_TEXT_BROKEN_LINK: adapter->backend");
                }
                if backend_text_ops > 0 && core_text_ops == 0 {
                    info!("GUI_TEXT_BROKEN_LINK: backend->core");
                }
                if core_text_ops > 0 && !cosmic_prepare_called {
                    info!("GUI_TEXT_BROKEN_LINK: core->cosmic_prepare");
                }
                if cosmic_prepare_called && glyph_count == 0 {
                    info!("GUI_TEXT_BROKEN_LINK: prepare->glyphs");
                }
                if glyph_count > 0 && !pipeline_render_called {
                    info!("GUI_TEXT_BROKEN_LINK: glyphs->pipeline_render");
                }

                crate::renderer::surface::submit_and_present(&self.queue, encoder, frame);
                if render_debug_enabled() {
                    log::debug!("submitted frame");
                }
                if render_timing_enabled() {
                    if let Some(start) = self.frame_start.take() {
                        let elapsed = start.elapsed();
                        eprintln!(
                            "GUI_RENDER_TIMING: frame={} duration_ms={:.2}",
                            frame_idx,
                            elapsed.as_secs_f64() * 1000.0
                        );
                    }
                }
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

                    if render_debug_enabled() {
                        log::debug!("debug pass checked (suboptimal)");
                    }

                    // SHAPE PASS: draw only the panel/background geometry using the
                    // dedicated shape_pipeline (no font sampling).
                    let panel_indices_len = panel_indices.len() as u32;
                    let total_indices_len = indices.len() as u32;

                    // Diagnostic: log shape/text split info (suboptimal path)
                    info!(
                        "render passes (suboptimal): panel_indices_len={} total_indices_len={} panel_verts={} text_verts={}",
                        panel_indices_len,
                        total_indices_len,
                        panel_vertex_count as usize,
                        verts.len().saturating_sub(panel_vertex_count as usize)
                    );

                    if !false {
                        if panel_indices_len > 0 {
                            if RENDER_DEBUG {
                                debug!(
                                    "shape pass indexed draw (suboptimal path): indices_drawn={}",
                                    panel_indices_len
                                );
                            }
                            // Diagnostic: explicit draw parameters for shape pass
                            debug!(
                                "shape pass draw_indexed (suboptimal): start=0 end={} count={} base_vertex=0",
                                panel_indices_len, panel_indices_len
                            );
                            crate::renderer::shapes::submit_shape_pass(
                                &mut rpass,
                                &self.shape_pipeline,
                                &self.vertex_buffer,
                                &self.index_buffer,
                                panel_indices_len,
                            );
                        }
                    } else {
                        info!("false enabled (suboptimal path): skipping shape pass");
                    }

                    // TEXT PASS
                    if total_indices_len > panel_indices_len {
                        if text_pass_disabled() {
                            if render_debug_enabled() {
                                log::debug!(
                                    "DISABLE_TEXT_PASS (suboptimal path): skipping text pass (would draw {} indices)",
                                    total_indices_len - panel_indices_len
                                );
                            }
                        } else {
                            if render_debug_enabled() {
                                log::debug!(
                                    "binding text pipeline and font_atlas bind_group for text pass (suboptimal path, queued={})",
                                    self.text_renderer.queued_len()
                                );
                            }

                            // Diagnostic: explicit draw parameters for text pass (suboptimal)
                            debug!(
                                "text pass draw_indexed (suboptimal): start={} end={} count={} (panel_indices_len={} total_indices_len={})",
                                panel_indices_len,
                                total_indices_len,
                                total_indices_len.saturating_sub(panel_indices_len),
                                panel_indices_len,
                                total_indices_len
                            );

                            // Prepare any queued text (shape/rasterize/upload) then render via CosmicText native path.
                            self.text_renderer.prepare(&self.device, &mut self.queue)?;
                            self.text_renderer.render_pass(
                                &mut rpass,
                                &self.text_pipeline,
                                panel_indices_len,
                                total_indices_len,
                            )?;
                        }
                    }
                }

                crate::renderer::surface::submit_and_present(&self.queue, encoder, frame);
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
}

// ── RenderCore: persistent renderer state that can be reused across frames ──

impl RenderCore {
    /// Create a persistent renderer state.
    ///
    /// Takes `Arc<Window>` for shared window ownership. The surface is created
    /// immediately from the `Arc` (safe `Surface<'static>` — no transmute).
    /// Pipelines and text renderer are lazily initialised on the first
    /// `render_to_window` call so they match the actual surface colour format.
    pub async fn new(
        window: Arc<Window>,
        clear_color: [f64; 4],
        surface_size: winit::dpi::PhysicalSize<u32>,
    ) -> Result<Self, RenderError> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            flags: wgpu::InstanceFlags::empty(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });

        // Create surface from Arc<Window> — yields Surface<'static> safely.
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|e| RenderError::Other(format!("create_surface failed: {:?}", e)))?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| RenderError::Other(format!("request_adapter failed: {:?}", e)))?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("zaroxi-engine-device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError::Other(format!("request_device failed: {:?}", e)))?;

        // Configure the persistent surface immediately.
        let surface_config =
            crate::renderer::surface::configure_surface(&surface, &adapter, &device, surface_size)?;

        // Create vertex/index buffers at a reasonable size.
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vb"),
            size: 131072,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ib"),
            size: 131072,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        {
            use std::mem;
            let vertex_size = mem::size_of::<Vertex>();
            let expected_vertex_size = size_of::<Vertex>();
            if vertex_size != expected_vertex_size {
                return Err(RenderError::Other(format!(
                    "Vertex size mismatch: expected {} bytes, got {}",
                    expected_vertex_size, vertex_size
                )));
            }
        }

        init_debug_flags();

        info!(
            "RenderCore ready: {}x{} format={:?} (pipelines deferred)",
            surface_config.width, surface_config.height, surface_config.format
        );

        Ok(Self {
            _window: window,
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            clear_color: Color {
                r: clear_color[0],
                g: clear_color[1],
                b: clear_color[2],
                a: clear_color[3],
            },
            text_pipeline: None,
            text_bind_layout: None,
            debug_pipeline: None,
            shape_pipeline: None,
            text_renderer: None,
            initialized_format: None,
            surface,
            surface_config,
            vertex_buffer,
            index_buffer,
            vello_overlay: None,
            cockpit_scene: None,
            cockpit_text: Vec::new(),
            cockpit_overlay_text: Vec::new(),
        })
    }

    /// Ensure pipelines and text renderer are initialised for the given
    /// surface configuration.  Called from `render_to_window` before the
    /// first frame (and again if the format changes).
    fn ensure_initialized(&mut self, config: &SurfaceConfiguration) -> Result<(), RenderError> {
        let format = config.format;

        if self.initialized_format == Some(format)
            && self.text_pipeline.is_some()
            && self.text_renderer.is_some()
        {
            return Ok(());
        }

        let (bind_layout, text_pipeline, debug_pipeline, shape_pipeline) =
            crate::renderer::pipelines::create_pipelines(&self.device, config)?;

        let font_size = 14.0f32;
        let text_renderer: Box<dyn crate::renderer::text::TextRenderer + Send + Sync> =
            Box::new(crate::renderer::text::CosmicTextRenderer::new(
                &self.device,
                &self.queue,
                format,
                font_size,
                &bind_layout,
            )?);

        self.text_bind_layout = Some(bind_layout);
        self.text_pipeline = Some(text_pipeline);
        self.debug_pipeline = Some(debug_pipeline);
        self.shape_pipeline = Some(shape_pipeline);
        self.text_renderer = Some(text_renderer);
        self.initialized_format = Some(format);

        info!("RenderCore pipelines initialised for format {:?}", format);
        Ok(())
    }

    /// Render a frame to the persistent surface.
    ///
    /// The surface, GPU device, pipelines, and text-renderer atlas/caches
    /// all persist across frames. Only the per-frame swapchain texture,
    /// texture view, and command encoder are created fresh each redraw.
    ///
    /// Reconfigures the surface on resize. Handles surface-lost/outdated
    /// by returning the appropriate error so the caller can retry.
    pub fn render_to_window(
        &mut self,
        surface_size: winit::dpi::PhysicalSize<u32>,
        layout: &RenderLayout,
        render_blocks: &[crate::UiBlock],
    ) -> Result<RenderPerf, RenderError> {
        // Reconfigure the persistent surface if the size changed.
        if self.surface_config.width != surface_size.width
            || self.surface_config.height != surface_size.height
        {
            let mut new_cfg = self.surface_config.clone();
            new_cfg.width = surface_size.width.max(1);
            new_cfg.height = surface_size.height.max(1);
            self.surface.configure(&self.device, &new_cfg);
            self.surface_config = new_cfg;
            if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_FRAMEFLOW: surface reconfigured to {}x{}",
                    self.surface_config.width, self.surface_config.height
                );
            }
        }

        let config = self.surface_config.clone();
        self.ensure_initialized(&config)?;

        let _ = self
            .text_renderer
            .as_ref()
            .unwrap()
            .resize_viewport(self.surface_config.width, self.surface_config.height);

        // Lazily create the cockpit vello overlay the first time a scene is set.
        if self.cockpit_scene.is_some() && self.vello_overlay.is_none() {
            match crate::renderer::vello_overlay::VelloOverlay::new(
                &self.device,
                self.surface_config.format,
            ) {
                Ok(o) => self.vello_overlay = Some(o),
                Err(e) => eprintln!("ZAROXI_COCKPIT: vello overlay init failed: {:?}", e),
            }
        }

        render_frame_inner(
            &self.device,
            &mut self.queue,
            self.text_pipeline.as_ref().unwrap(),
            self.shape_pipeline.as_ref().unwrap(),
            self.text_renderer.as_deref_mut().unwrap(),
            &self.clear_color,
            &self.surface,
            &self.surface_config,
            &self.vertex_buffer,
            &self.index_buffer,
            layout,
            render_blocks,
            self.vello_overlay.as_mut(),
            self.cockpit_scene.as_ref(),
            &self.cockpit_text,
            &self.cockpit_overlay_text,
        )
    }

    /// Access the device (for compatibility).
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Access the queue (for compatibility).
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Access the text renderer for external queries (e.g. monospace advance).
    pub fn text_renderer(
        &self,
    ) -> Option<&(dyn crate::renderer::text::TextRenderer + Send + Sync)> {
        self.text_renderer.as_deref()
    }

    /// Override the text shaping budget (ms) for upcoming frames, or `None` to
    /// restore the steady-state budget. Used by the app to run an open-burst
    /// that completes the freshly-visible viewport in one pass.
    pub fn set_shape_budget_ms(&self, ms: Option<f32>) {
        if let Some(tr) = self.text_renderer.as_deref() {
            tr.set_shape_budget_ms(ms);
        }
    }

    /// Set the cockpit vello scene to composite on the next frame; `None`
    /// disables the overlay. The scene is built by the host
    /// (`zaroxi-interface-widgets`); the renderer only composites it on top of
    /// the main GUI. Cockpit *text* is drawn by the cosmic-text pass, not vello.
    pub fn set_cockpit_scene(&mut self, scene: Option<vello::Scene>) {
        self.cockpit_scene = scene;
    }

    /// Set the cockpit text runs to draw via cosmic-text on the next frame.
    /// Pass an empty vec to clear. Positions are physical px in surface space.
    pub fn set_cockpit_text(&mut self, items: Vec<CockpitText>) {
        self.cockpit_text = items;
    }

    /// Set overlay text runs (popup option labels) to draw AFTER the vello
    /// overlay pass. Pass an empty vec to clear. These sit on top of cockpit
    /// shape visuals (popup backgrounds, selection highlights).
    pub fn set_cockpit_overlay_text(&mut self, items: Vec<CockpitText>) {
        self.cockpit_overlay_text = items;
    }
}

/// A positioned cockpit text run to be drawn by the cosmic-text pass.
///
/// The cockpit vello overlay draws only vector visuals; its text is delegated to
/// the renderer's authoritative cosmic-text path. The host (desktop) supplies
/// these per frame via [`RenderCore::set_cockpit_text`]; the renderer converts
/// each into a [`crate::renderer::text::TextCommand`] queued before the text pass.
#[derive(Debug, Clone, PartialEq)]
pub struct CockpitText {
    /// String to render (BiDi/Arabic shaped by cosmic-text).
    pub text: String,
    /// Left edge in physical px.
    pub x: f32,
    /// Top edge in physical px.
    pub y: f32,
    /// Font size in px.
    pub size_px: f32,
    /// RGBA color.
    pub color: [f32; 4],
    /// Optional clip rect `(x, y, w, h)` passed through from the widget that
    /// produced this text. Glyphs outside this region are culled by the cosmic
    /// text layer.
    pub clip_rect: Option<(f32, f32, f32, f32)>,
}

/// Per-frame render-side timing + counters, gated behind `ZAROXI_PERF_TRACE=1`.
///
/// Populated by `render_frame_inner` on the live success path and returned to
/// the caller (the app frame loop) so a single consolidated per-frame
/// `ZAROXI_PERF_TRACE` line can be printed with both app-side and render-side
/// phases. All ms fields are wall-clock milliseconds for the current frame.
///
/// Phase split (see `CosmicTextRenderer::prepare`):
/// - `text_shape_ms`: per-command cosmic shaping + per-glyph rasterization loop
///   (CPU-bound; the dominant suspect for the hot path).
/// - `text_prepare_ms`: GPU atlas upload + instance-buffer build/upload.
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderPerf {
    pub text_shape_ms: f32,
    pub text_prepare_ms: f32,
    pub gpu_encode_ms: f32,
    pub gpu_submit_present_ms: f32,
    pub text_cmd_count: usize,
    pub glyph_count: usize,
    /// Lines whose shaping was deferred by the per-frame budget (staged first
    /// paint). >0 means the caller should request another frame to finish.
    /// Populated regardless of `ZAROXI_PERF_TRACE` so staged paint works always.
    pub shaping_pending: usize,
    /// UI-element buckets whose prepared glyph instances were reused from the
    /// retained per-element draw-payload cache (no re-shaping) this frame.
    pub elements_reused: usize,
    /// UI-element buckets that were re-emitted (content changed) this frame.
    pub elements_rebuilt: usize,
    /// Bytes uploaded to the GPU text instance buffer this frame.
    pub gpu_upload_bytes: usize,
    /// Why the instance buffer was (or was not) re-uploaded: `"reused"`,
    /// `"rebuilt"`, `"partial"`, or `"none"`.
    pub gpu_upload_reason: &'static str,
    /// Lines actually shaped (cache miss) this frame.
    pub lines_shaped: usize,
    /// Lines considered (visible window queued) this frame.
    pub lines_considered: usize,
}

/// Whether `ZAROXI_PERF_TRACE=1` is set. Cheap env read; only consulted on the
/// success path so timing overhead is paid solely when tracing is requested.
fn perf_trace_enabled() -> bool {
    std::env::var("ZAROXI_PERF_TRACE").as_deref() == Ok("1")
}

/// Map a render block id to the UI-element class used by the text renderer's
/// per-element retained draw-payload cache. This determines which glyph
/// instances can be reused independently when only part of the shell changes.
/// Misclassification only affects cache bucketing (perf), never correctness.
fn element_for_block(id: &str) -> u32 {
    use crate::renderer::text::element as el;
    if id == "editor_content" || id.contains("ContentArea") || id.contains("content_area") {
        el::EDITOR_CONTENT
    } else if id.contains("gutter") {
        el::GUTTER
    } else if id.contains("status") {
        el::STATUS_BAR
    } else if id.contains("ai_panel") || id.starts_with("ai_") {
        el::AI_PANEL
    } else if id.contains("explorer") || id.contains("sidebar") || id.contains("side_panel") {
        el::SIDE_PANEL
    } else if id.contains("bottom") || id.contains("terminal") {
        el::BOTTOM_PANEL
    } else if id == "toolbar"
        || id.contains("titlebar")
        || id.contains("title_bar")
        || id.contains("title-bar")
        || id.contains("tab")
        || id.contains("header")
        || id.contains("rail")
        || id.contains("chrome")
    {
        el::CHROME
    } else {
        el::OTHER
    }
}

/// Shared frame rendering logic used by both Renderer and RenderCore.
fn render_frame_inner(
    device: &Device,
    queue: &mut Queue,
    text_pipeline: &wgpu::RenderPipeline,
    shape_pipeline: &wgpu::RenderPipeline,
    text_renderer: &mut (dyn crate::renderer::text::TextRenderer + Send + Sync),
    clear_color: &Color,
    surface: &Surface,
    config: &SurfaceConfiguration,
    vertex_buffer: &Buffer,
    index_buffer: &Buffer,
    layout: &RenderLayout,
    render_blocks: &[crate::UiBlock],
    cockpit_overlay: Option<&mut crate::renderer::vello_overlay::VelloOverlay>,
    cockpit_scene: Option<&vello::Scene>,
    cockpit_text: &[CockpitText],
    cockpit_overlay_text: &[CockpitText],
) -> Result<RenderPerf, RenderError> {
    if config.width == 0 || config.height == 0 {
        return Ok(RenderPerf::default());
    }

    let mut frame_start =
        if render_timing_enabled() { Some(std::time::Instant::now()) } else { None };

    let width = config.width as f32;
    let height = config.height as f32;
    let sem = &layout.colors;

    if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
        eprintln!(
            "ZAROXI_DIAG: render_frame — surface={:.0}x{:.0} layout_sidebar=({:.0},{:.0},{:.0},{:.0}) layout_editor=({:.0},{:.0},{:.0},{:.0}) nblocks={}",
            width,
            height,
            layout.sidebar.x,
            layout.sidebar.y,
            layout.sidebar.w,
            layout.sidebar.h,
            layout.editor.x,
            layout.editor.y,
            layout.editor.w,
            layout.editor.h,
            render_blocks.len(),
        );
    }

    let mut panel_verts: Vec<Vertex> = Vec::new();
    let mut panel_indices: Vec<u16> = Vec::new();
    let mut text_verts: Vec<Vertex> = Vec::new();
    let mut text_indices: Vec<u16> = Vec::new();

    if validation_scene_enabled() {
        let band_h = (height as f32) / 3.0;
        push_colored_quad(
            &mut panel_verts,
            &mut panel_indices,
            0.0,
            0.0,
            width as f32,
            band_h,
            [1.0, 0.0, 0.0, 1.0],
            width,
            height,
            0.0,
        );
        push_colored_quad(
            &mut panel_verts,
            &mut panel_indices,
            0.0,
            band_h,
            width as f32,
            band_h,
            [0.0, 1.0, 0.0, 1.0],
            width,
            height,
            0.0,
        );
        push_colored_quad(
            &mut panel_verts,
            &mut panel_indices,
            0.0,
            band_h * 2.0,
            width as f32,
            band_h,
            [0.0, 0.0, 1.0, 1.0],
            width,
            height,
            0.0,
        );
    }

    let header_h = 28.0f32;
    let content_padding = 8.0f32;
    const DEFAULT_FONT_SIZE: f32 = 14.0;

    for block in render_blocks.iter() {
        if !block.visible {
            continue;
        }
        let target = block.rect;
        // UI-element class for this block, applied to every text command it
        // queues so the text renderer can retain/reuse this element's prepared
        // glyph instances independently of the rest of the shell.
        let block_elem = element_for_block(&block.id);

        crate::renderer::shapes::queue_panel_quads(
            &mut panel_verts,
            &mut panel_indices,
            block,
            &sem,
            width,
            height,
        );

        let hx = target.x;
        let hy = target.y;
        let hw = target.w;
        let hh = header_h.min(target.h.max(0.0));
        let title_x = target.x + 8.0;
        let title_y = target.y + (hh - DEFAULT_FONT_SIZE) * 0.5;
        let title_color: [f32; 4] = block.text_color.unwrap_or(layout.colors.text_default);

        // Status strip: a header-only block whose `content_spans` carry the
        // right-aligned segments. Render the title left-aligned and those
        // segments right-aligned via the responsive planner so the two groups
        // never overlap. This is the LIVE gui_shell path (`render_to_window` →
        // `render_frame_inner`); the equivalent branch in `render_with_layout`
        // is the legacy `Renderer` path and is not exercised by the desktop app.
        let status_render_debug = std::env::var("ZAROXI_STATUS_RENDER_DEBUG").as_deref() == Ok("1");
        if block.header_only && block.content_spans.is_some() {
            let pad = 8.0f32;
            let device_scale: f32 = std::env::var("ZAROXI_SURFACE_SCALE")
                .ok()
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.0);
            let advance = text_renderer
                .monospace_advance_x()
                .map(|a| a / device_scale.max(0.01))
                .filter(|a| a.is_finite() && *a > 1.0 && *a < DEFAULT_FONT_SIZE * 2.0)
                .unwrap_or(DEFAULT_FONT_SIZE * 0.6);
            let gap = (advance * 2.0).max(12.0);
            let right_segments: Vec<String> = block
                .content_spans
                .as_ref()
                .map(|spans| spans.iter().map(|(t, _)| t.clone()).collect())
                .unwrap_or_default();
            let runs = crate::renderer::header_layout::plan_status_header(
                &block.title,
                &right_segments,
                hx,
                hw,
                pad,
                advance,
                gap,
            );

            if status_render_debug {
                eprintln!(
                    "ZAROXI_STATUS_RENDER_DEBUG[core]: branch=dual_run block='{}' header_only={} title={:?} right_segs={} rect=(x={:.0} y={:.0} w={:.0} h={:.0}) advance={:.2} gap={:.1} runs={}",
                    block.id,
                    block.header_only,
                    block.title,
                    right_segments.len(),
                    hx,
                    hy,
                    hw,
                    hh,
                    advance,
                    gap,
                    runs.len()
                );
                for (i, run) in runs.iter().enumerate() {
                    eprintln!(
                        "ZAROXI_STATUS_RENDER_DEBUG[core]:   run[{}] text={:?} x={:.1} y={:.1} clip=(x={:.1} w={:.1}) queued=true",
                        i, run.text, run.x, title_y, run.clip_x, run.clip_w
                    );
                }
            }

            for run in &runs {
                text_renderer.queue_text(
                    crate::renderer::text::TextCommand::new_title(
                        &run.text,
                        run.x,
                        title_y,
                        title_color,
                        DEFAULT_FONT_SIZE,
                        run.clip_x,
                        hy,
                        run.clip_w,
                        hh,
                    )
                    .with_element(block_elem),
                );
            }
        } else {
            if status_render_debug && block.id == "status_bar" {
                eprintln!(
                    "ZAROXI_STATUS_RENDER_DEBUG[core]: branch=fallback_title_only block='{}' header_only={} has_spans={} title={:?} — right segments NOT rendered",
                    block.id,
                    block.header_only,
                    block.content_spans.is_some(),
                    block.title
                );
            }
            text_renderer.queue_text(
                crate::renderer::text::TextCommand::new_title(
                    &block.title,
                    title_x,
                    title_y,
                    title_color,
                    DEFAULT_FONT_SIZE,
                    hx,
                    hy,
                    hw,
                    hh,
                )
                .with_element(block_elem),
            );
        }

        let content = block.content.trim();
        let is_titlebar =
            block.id == "titlebar" || block.id == "title_bar" || block.id == "title-bar";

        if is_titlebar {
        } else if !content.is_empty() {
            let content_y = target.y + hh + content_padding;
            let content_h = (target.h - hh - content_padding * 2.0).max(0.0);
            let (text_x, clip_x, clip_w) = if let Some(ref clip) = block.clip_rect {
                let tx = clip.x - block.content_offset_x;
                (tx, clip.x, clip.w)
            } else {
                let cx = target.x + content_padding;
                let cw = (target.w - content_padding * 2.0).max(0.0);
                (cx, cx, cw)
            };
            let text_y = content_y - block.content_offset_y;
            if clip_w > 0.0 && content_h > 0.0 {
                if let Some(ref spans) = block.content_spans {
                    let line_h = DEFAULT_FONT_SIZE + 2.0;
                    let clip_bottom = content_y + content_h;
                    // Honor content_line_offset symmetrically with the plain
                    // path so a viewport-windowed span list (whose first run is
                    // absolute line `content_line_offset`) lands at the correct
                    // absolute y. content_offset_y still applies the scroll.
                    let mut cursor_y =
                        text_y + block.content_line_offset.unwrap_or(0) as f32 * line_h;

                    // Fast-forward whole lines entirely above the visible clip
                    // area so scrolling does not pay O(total_lines) cost.
                    let mut ff_y = cursor_y;
                    let mut ff_idx: usize = 0;
                    while ff_y < content_y && ff_idx < spans.len() {
                        if spans[ff_idx].0 == "\n" {
                            ff_y += line_h;
                        }
                        ff_idx += 1;
                    }
                    let effective_spans = if ff_idx > 0 {
                        cursor_y = ff_y;
                        &spans[ff_idx..]
                    } else {
                        spans.as_slice()
                    };

                    // Accumulate each logical line's colored runs and shape the
                    // whole line as ONE continuous buffer (per-run colors via
                    // rich-text attrs). This preserves syntax colors while
                    // keeping normal continuous editor-text layout — no
                    // per-segment positioning, no advance drift.
                    let mut line_runs: Vec<(String, [f32; 4])> = Vec::new();
                    for (span_text, span_color) in effective_spans {
                        if span_text == "\n" {
                            let line_visible =
                                cursor_y >= content_y && cursor_y + line_h <= clip_bottom;
                            if line_visible && !line_runs.is_empty() {
                                text_renderer.queue_text(
                                    crate::renderer::text::TextCommand::new_body_runs(
                                        std::mem::take(&mut line_runs),
                                        text_x,
                                        cursor_y,
                                        DEFAULT_FONT_SIZE,
                                        clip_x,
                                        content_y,
                                        clip_w,
                                        content_h,
                                    )
                                    .with_element(block_elem),
                                );
                            } else {
                                line_runs.clear();
                            }
                            cursor_y += line_h;
                            if cursor_y + line_h > clip_bottom {
                                break;
                            }
                            continue;
                        }
                        if !span_text.is_empty() {
                            line_runs.push((span_text.clone(), *span_color));
                        }
                    }
                    // Flush a trailing line not terminated by "\n".
                    let last_visible = cursor_y >= content_y && cursor_y + line_h <= clip_bottom;
                    if last_visible && !line_runs.is_empty() {
                        text_renderer.queue_text(
                            crate::renderer::text::TextCommand::new_body_runs(
                                line_runs,
                                text_x,
                                cursor_y,
                                DEFAULT_FONT_SIZE,
                                clip_x,
                                content_y,
                                clip_w,
                                content_h,
                            )
                            .with_element(block_elem),
                        );
                    }
                } else {
                    let clip_bottom = content_y + content_h;
                    let line_h = DEFAULT_FONT_SIZE + 2.0;
                    // Viewport-only rendering: when content_line_offset is set,
                    // `content` carries only the visible window of lines (plus
                    // overscan). The offset adjusts cursor_y so the first line
                    // in `content` starts at the correct absolute y-position,
                    // matching scroll and cursor positioning computed from
                    // absolute line numbers.
                    let mut cursor_y =
                        text_y + block.content_line_offset.unwrap_or(0) as f32 * line_h;
                    if std::env::var("ZAROXI_DEBUG_RENDER_WINDOW").as_deref() == Ok("1") {
                        let visible_line_start = block.content_line_offset.unwrap_or(0);
                        let content_byte_count = block.content.len();
                        let line_count = block.content.lines().count();
                        eprintln!(
                            "ZAROXI_DEBUG_RENDER_WINDOW: block={} clip_y={:.1} clip_bottom={:.1} line_start={} content_bytes={} content_lines={}",
                            block.id,
                            content_y,
                            clip_bottom,
                            visible_line_start,
                            content_byte_count,
                            line_count,
                        );
                    }
                    for line_str in block.content.lines() {
                        if cursor_y + line_h > clip_bottom {
                            break;
                        }
                        if cursor_y >= content_y {
                            text_renderer.queue_text(
                                crate::renderer::text::TextCommand::new_body(
                                    line_str,
                                    text_x,
                                    cursor_y,
                                    title_color,
                                    DEFAULT_FONT_SIZE,
                                    clip_x,
                                    content_y,
                                    clip_w,
                                    content_h,
                                )
                                .with_element(block_elem),
                            );
                        }
                        cursor_y += line_h;
                    }
                }
            }
        }

        if let (Some(line), Some(col)) = (block.cursor_line, block.cursor_col) {
            let content_x = if let Some(ref clip) = block.clip_rect {
                clip.x - block.content_offset_x
            } else {
                target.x + content_padding
            };
            let content_w = if let Some(ref clip) = block.clip_rect {
                clip.w
            } else {
                (target.w - content_padding * 2.0).max(0.0)
            };
            let content_y = target.y + hh + content_padding;
            let content_h = (target.h - hh - content_padding * 2.0).max(0.0);
            // Use the actual monospace glyph advance from the font system
            // in physical pixels for cursor and selection positioning.
            let char_w = text_renderer.monospace_advance_x().unwrap_or(8.0);
            if content_w > 0.0 && content_h > 0.0 {
                let line_h = DEFAULT_FONT_SIZE + 2.0;
                let text_y = content_y as f32 - block.content_offset_y;
                let line_y = text_y + line as f32 * line_h;
                if block.highlight_active_line
                    && line_y >= content_y
                    && line_y + line_h <= content_y + content_h
                {
                    let hl_color: [f32; 4] = layout.colors.editor_line_highlight;
                    push_colored_quad(
                        &mut panel_verts,
                        &mut panel_indices,
                        content_x,
                        line_y,
                        content_w,
                        line_h,
                        hl_color,
                        width,
                        height,
                        0.0,
                    );
                }
                let cursor_x = content_x + col as f32 * char_w;
                let cursor_w = 2.0;
                let cursor_h = line_h;
                if cursor_x >= content_x
                    && cursor_x + cursor_w <= content_x + content_w
                    && line_y >= content_y
                    && line_y + cursor_h <= content_y + content_h
                {
                    let cursor_color: [f32; 4] = layout.colors.editor_cursor;
                    push_colored_quad(
                        &mut panel_verts,
                        &mut panel_indices,
                        cursor_x,
                        line_y,
                        cursor_w,
                        cursor_h,
                        cursor_color,
                        width,
                        height,
                        0.0,
                    );
                }
            }
            if let Some((sl, sc, el, ec)) = block.selection_range {
                let line_h = DEFAULT_FONT_SIZE + 2.0;
                let text_y = content_y - block.content_offset_y;
                let sel_color: [f32; 4] = layout.colors.editor_selection;
                for line in sl..=el {
                    let line_y = text_y + line as f32 * line_h;
                    if line_y + line_h <= content_y {
                        continue;
                    }
                    if line_y + line_h > content_y + content_h {
                        break;
                    }
                    let start_col = if line == sl { sc } else { 0 };
                    let end_col = if line == el { ec } else { 200 };
                    let sel_x = content_x + start_col as f32 * char_w;
                    let sel_w = ((end_col.saturating_sub(start_col)) as f32 * char_w)
                        .min(content_w - (sel_x - content_x));
                    if sel_w > 0.0 {
                        push_colored_quad(
                            &mut panel_verts,
                            &mut panel_indices,
                            sel_x,
                            line_y,
                            sel_w,
                            line_h,
                            sel_color,
                            width,
                            height,
                            0.0,
                        );
                    }
                }
            }
        }
    }

    let panel_vertex_count = panel_verts.len() as u16;
    let mut verts: Vec<Vertex> = panel_verts;
    let mut indices: Vec<u16> = panel_indices.clone();
    for idx in text_indices.iter() {
        indices.push(idx.wrapping_add(panel_vertex_count));
    }
    verts.extend(text_verts.into_iter());

    let vb_bytes = bytemuck::cast_slice(&verts);
    queue.write_buffer(vertex_buffer, 0, vb_bytes);
    let ib_bytes = bytemuck::cast_slice(&indices);
    queue.write_buffer(index_buffer, 0, ib_bytes);

    let gpu_frame = GPU_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed);

    if gpu_trace_enabled() {
        let mut vert_hash: u64 = 0;
        for v in &verts {
            vert_hash = vert_hash.wrapping_mul(31).wrapping_add((v.pos[0] * 100.0) as u64);
            vert_hash = vert_hash.wrapping_mul(31).wrapping_add((v.pos[1] * 100.0) as u64);
        }
        let mut idx_hash: u64 = 0;
        for i in &indices {
            idx_hash = idx_hash.wrapping_mul(31).wrapping_add(*i as u64);
        }
        let text_queued = text_renderer.queued_len();
        eprintln!(
            "ZAROXI_RENDER_TRACE: gpu_frame frame={} nverts={} nidx={} vert_hash={:016x} idx_hash={:016x} text_queued={}",
            gpu_frame,
            verts.len(),
            indices.len(),
            vert_hash,
            idx_hash,
            text_queued,
        );
    }

    let current = surface.get_current_texture();
    match current {
        wgpu::CurrentSurfaceTexture::Success(frame_tex) => {
            if gpu_trace_enabled() {
                eprintln!("ZAROXI_RENDER_TRACE: present_frame frame={} acquire=Success", gpu_frame);
            }
            if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                eprintln!("ZAROXI_FRAMEFLOW: get_current_texture = Success");
            }
            let view = frame_tex.texture.create_view(&TextureViewDescriptor::default());
            let perf_on = perf_trace_enabled();
            let encode_start = std::time::Instant::now();
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("zaroxi-render-encoder"),
            });

            let panel_indices_len = panel_indices.len() as u32;
            let total_indices_len = indices.len() as u32;

            // Queue cockpit text (non-popup: status bar, minimap, settings labels)
            // into the cosmic-text layer. These render BEFORE the vello overlay so
            // the overlay's opaque popup backgrounds etc. cover them.
            for ct in cockpit_text {
                let (clip_x, clip_y, clip_w, clip_h) =
                    ct.clip_rect.unwrap_or((0.0, 0.0, config.width as f32, config.height as f32));
                text_renderer.queue_text(crate::renderer::text::TextCommand::new_body(
                    &ct.text, ct.x, ct.y, ct.color, ct.size_px, clip_x, clip_y, clip_w, clip_h,
                ));
            }
            let mut text_cmd_count: usize = 0;
            let mut prepare_wall_ms: f32 = 0.0;

            let has_text = total_indices_len > panel_indices_len || text_renderer.queued_len() > 0;
            if has_text && !text_pass_disabled() {
                text_cmd_count = text_renderer.queued_len();
                let prep_start = std::time::Instant::now();
                text_renderer.prepare(device, queue)?;
                prepare_wall_ms = prep_start.elapsed().as_secs_f32() * 1000.0;
            }

            // Pass 1: shapes + cockpit text (Clear — erases previous frame).
            // Cockpit text here is behind the vello overlay (Pass 2) so popup
            // backgrounds cover settings labels etc.
            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shape-text-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(*clear_color),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });

                if panel_indices_len > 0 {
                    crate::renderer::shapes::submit_shape_pass(
                        &mut rpass,
                        shape_pipeline,
                        vertex_buffer,
                        index_buffer,
                        panel_indices_len,
                    );
                }

                if text_cmd_count > 0 {
                    text_renderer.render_pass(
                        &mut rpass,
                        text_pipeline,
                        panel_indices_len,
                        total_indices_len,
                    )?;
                }
            }

            // Pass 2: cockpit vello overlay (Load — adds cockpit vector visuals
            // on top of shape + text pass). Popup backgrounds, selection
            // highlights etc. cover the text rendered in Pass 1.
            if let (Some(overlay), Some(scene)) = (cockpit_overlay, cockpit_scene) {
                overlay.composite(
                    device,
                    &*queue,
                    &mut encoder,
                    &view,
                    scene,
                    config.width,
                    config.height,
                );
            }

            // Queue overlay text (popup option labels) and render it in a
            // separate pass AFTER the vello overlay, so it sits on top of
            // popup backgrounds and selection highlights.
            let mut overlay_text_cmd_count: usize = 0;
            if !cockpit_overlay_text.is_empty() && !text_pass_disabled() {
                for ct in cockpit_overlay_text {
                    let (clip_x, clip_y, clip_w, clip_h) = ct.clip_rect.unwrap_or((
                        0.0,
                        0.0,
                        config.width as f32,
                        config.height as f32,
                    ));
                    text_renderer.queue_text(crate::renderer::text::TextCommand::new_body(
                        &ct.text, ct.x, ct.y, ct.color, ct.size_px, clip_x, clip_y, clip_w, clip_h,
                    ));
                }
                overlay_text_cmd_count = text_renderer.queued_len();
                text_renderer.prepare(device, queue)?;
            }

            // Pass 3: overlay text (Load — popup labels on top of vello overlay).
            if overlay_text_cmd_count > 0 {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("overlay-text-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });

                text_renderer.render_pass(
                    &mut rpass,
                    text_pipeline,
                    panel_indices_len,
                    total_indices_len,
                )?;
            }

            let gpu_encode_ms =
                (encode_start.elapsed().as_secs_f32() * 1000.0 - prepare_wall_ms).max(0.0);
            let submit_start = std::time::Instant::now();
            crate::renderer::surface::submit_and_present(queue, encoder, frame_tex);
            let gpu_submit_present_ms = submit_start.elapsed().as_secs_f32() * 1000.0;
            if gpu_trace_enabled() {
                eprintln!(
                    "ZAROXI_RENDER_TRACE: present_frame frame={} submit=done present=done",
                    gpu_frame
                );
            }
            if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                eprintln!("ZAROXI_FRAMEFLOW: queue.submit + present() done");
            }
            if render_timing_enabled() {
                if let Some(start) = frame_start.take() {
                    let elapsed = start.elapsed();
                    eprintln!(
                        "GUI_RENDER_TIMING: duration_ms={:.2}",
                        elapsed.as_secs_f64() * 1000.0
                    );
                }
            }
            let perf = if perf_on {
                RenderPerf {
                    text_shape_ms: text_renderer.perf_shape_ms(),
                    text_prepare_ms: text_renderer.perf_prepare_ms(),
                    gpu_encode_ms,
                    gpu_submit_present_ms,
                    text_cmd_count,
                    glyph_count: text_renderer.perf_glyph_count(),
                    shaping_pending: text_renderer.perf_pending_lines(),
                    elements_reused: text_renderer.perf_elements_reused(),
                    elements_rebuilt: text_renderer.perf_elements_rebuilt(),
                    gpu_upload_bytes: text_renderer.perf_gpu_upload_bytes(),
                    gpu_upload_reason: text_renderer.perf_gpu_upload_reason(),
                    lines_shaped: text_renderer.perf_lines_shaped(),
                    lines_considered: text_renderer.perf_lines_considered(),
                }
            } else {
                // Staged first paint + open-settle trace need shaping_pending /
                // shaped / considered even when perf tracing is off, so always
                // populate them.
                RenderPerf {
                    shaping_pending: text_renderer.perf_pending_lines(),
                    lines_shaped: text_renderer.perf_lines_shaped(),
                    lines_considered: text_renderer.perf_lines_considered(),
                    ..RenderPerf::default()
                }
            };
            Ok(perf)
        }
        wgpu::CurrentSurfaceTexture::Suboptimal(frame_tex) => {
            if gpu_trace_enabled() {
                eprintln!(
                    "ZAROXI_RENDER_TRACE: present_frame frame={} acquire=Suboptimal",
                    gpu_frame
                );
            }
            if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                eprintln!("ZAROXI_FRAMEFLOW: get_current_texture = Suboptimal (rendering anyway)");
            }
            let view = frame_tex.texture.create_view(&TextureViewDescriptor::default());
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("zaroxi-render-encoder"),
            });
            let panel_indices_len = panel_indices.len() as u32;
            let total_indices_len = indices.len() as u32;
            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("main-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(*clear_color),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                if panel_indices_len > 0 {
                    crate::renderer::shapes::submit_shape_pass(
                        &mut rpass,
                        shape_pipeline,
                        vertex_buffer,
                        index_buffer,
                        panel_indices_len,
                    );
                }
                if total_indices_len > panel_indices_len {
                    if !text_pass_disabled() {
                        text_renderer.prepare(device, queue)?;
                        text_renderer.render_pass(
                            &mut rpass,
                            text_pipeline,
                            panel_indices_len,
                            total_indices_len,
                        )?;
                    }
                }
            }
            crate::renderer::surface::submit_and_present(queue, encoder, frame_tex);
            if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_FRAMEFLOW: Suboptimal frame presented; returning SurfaceOutdated for reconfigure"
                );
            }
            Err(RenderError::SurfaceOutdated)
        }
        wgpu::CurrentSurfaceTexture::Timeout => {
            if gpu_trace_enabled() {
                eprintln!("ZAROXI_RENDER_TRACE: present_frame frame={} acquire=Timeout", gpu_frame);
            }
            info!("current surface texture timed out — retry needed");
            Err(RenderError::SurfaceTimeout)
        }
        wgpu::CurrentSurfaceTexture::Occluded => {
            if gpu_trace_enabled() {
                eprintln!(
                    "ZAROXI_RENDER_TRACE: present_frame frame={} acquire=Occluded",
                    gpu_frame
                );
            }
            info!("current surface texture occluded — retry when visible");
            Err(RenderError::SurfaceOccluded)
        }
        wgpu::CurrentSurfaceTexture::Outdated => {
            if gpu_trace_enabled() {
                eprintln!(
                    "ZAROXI_RENDER_TRACE: present_frame frame={} acquire=Outdated",
                    gpu_frame
                );
            }
            info!("surface outdated — reconfigure needed");
            Err(RenderError::SurfaceOutdated)
        }
        wgpu::CurrentSurfaceTexture::Lost => {
            if gpu_trace_enabled() {
                eprintln!("ZAROXI_RENDER_TRACE: present_frame frame={} acquire=Lost", gpu_frame);
            }
            info!("surface lost — reconfigure needed");
            Err(RenderError::SurfaceLost)
        }
        wgpu::CurrentSurfaceTexture::Validation => {
            if gpu_trace_enabled() {
                eprintln!(
                    "ZAROXI_RENDER_TRACE: present_frame frame={} acquire=Validation",
                    gpu_frame
                );
            }
            Err(RenderError::SurfaceValidation("validation error".to_string()))
        }
    }
}
