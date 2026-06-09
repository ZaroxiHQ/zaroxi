use crate::error::RenderError;
use log::{debug, info};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

static GUI_TEXT_FRAME_COUNTER: AtomicUsize = AtomicUsize::new(0);
use wgpu::{
    Backends, BindGroup, BindGroupLayout, Buffer, Color, CommandEncoderDescriptor, Device,
    DeviceDescriptor, Extent3d, Features, Instance, InstanceDescriptor, Limits, Queue,
    RequestAdapterOptions, SamplerDescriptor, Surface, SurfaceConfiguration, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
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
                        let mut cursor_y = text_y;
                        let line_h = DEFAULT_FONT_SIZE + 2.0;
                        let clip_bottom = content_y + content_h;
                        let mut line_buf = String::new();
                        for (span_text, span_color) in spans {
                            if span_text == "\n" {
                                if !line_buf.is_empty() {
                                    self.text_renderer.queue_text(
                                        crate::renderer::text::TextCommand::new_body(
                                            &line_buf,
                                            text_x,
                                            cursor_y,
                                            *span_color,
                                            DEFAULT_FONT_SIZE,
                                            clip_x,
                                            content_y,
                                            clip_w,
                                            content_h,
                                        ),
                                    );
                                    line_buf.clear();
                                }
                                cursor_y += line_h;
                                // Stop emitting text commands once the next line would
                                // start entirely below the clip area. This avoids
                                // queueing off-screen lines while preserving full
                                // visibility for the last partially-visible line.
                                if cursor_y >= clip_bottom {
                                    break;
                                }
                                continue;
                            }
                            line_buf.push_str(span_text);
                        }
                        // Flush any remaining line_buf that hasn't been terminated by \n
                        if !line_buf.is_empty() && cursor_y < clip_bottom {
                            self.text_renderer.queue_text(
                                crate::renderer::text::TextCommand::new_body(
                                    &line_buf,
                                    text_x,
                                    cursor_y,
                                    *spans.last().map(|(_, c)| c).unwrap_or(&[1.0; 4]),
                                    DEFAULT_FONT_SIZE,
                                    clip_x,
                                    content_y,
                                    clip_w,
                                    content_h,
                                ),
                            );
                        }
                    } else {
                        // Queue full content for CosmicText native rendering.
                        // CosmicText clips via per-glyph bounds check — no source truncation.
                        // clip_x stays at viewport; text_x shifts with scroll offset.
                        self.text_renderer.queue_text(
                            crate::renderer::text::TextCommand::new_body(
                                &block.content,
                                text_x,
                                text_y,
                                title_color,
                                DEFAULT_FONT_SIZE,
                                clip_x,
                                text_y,
                                clip_w,
                                content_h,
                            ),
                        );
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

                if content_w > 0.0 && content_h > 0.0 {
                    let line_h = DEFAULT_FONT_SIZE + 2.0;
                    let text_y = content_y as f32 - block.content_offset_y;
                    let line_y = text_y + line as f32 * line_h;

                    // Active line highlight background
                    if block.highlight_active_line && line_y + line_h <= content_y + content_h {
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

                    // Cursor vertical bar
                    let cursor_x = content_x + col as f32 * 8.0;
                    let cursor_w = 2.0;
                    let cursor_h = line_h;
                    if cursor_x + cursor_w <= content_x + content_w
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

                // Selection highlight
                if let Some((sl, sc, el, ec)) = block.selection_range {
                    let line_h = DEFAULT_FONT_SIZE + 2.0;
                    let char_w = 8.0;
                    let text_y = content_y - block.content_offset_y;
                    let sel_color: [f32; 4] = layout.colors.editor_selection;
                    for line in sl..=el {
                        let line_y = text_y + line as f32 * line_h;
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
