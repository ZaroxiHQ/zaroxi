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
use std::sync::atomic::Ordering;

use zaroxi_app::AppState;
use zaroxi_theme::{SemanticColors, Color as ThemeColor};

use crate::renderer::debug::{
    render_debug_enabled, RENDER_DEBUG, TEXT_SAMPLER_NEAREST, FIRST_GLYPH_LOGGED,
    LOGGED_TITLEBAR, LOGGED_SIDEBAR, LOGGED_EDITOR, LOGGED_SIDEBAR_PACKED,
    FORCE_MAGENTA_SIDEBAR, DISABLE_TEXT_PASS, VALIDATION_SCENE,
};
use crate::renderer::geometry::{Vertex, push_colored_quad, color_to_rgba};

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
    pub fn push_colored_quad(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
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

use crate::renderer::text::FontAtlas;

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
        // Configure the surface (moved to renderer::surface)
        let config = crate::renderer::surface::configure_surface(&surface, &adapter, &device, size)?;

        // Create pipelines & bind group layouts (moved to renderer::pipelines).
        let (text_bind_layout, text_pipeline, debug_pipeline, shape_pipeline) =
            crate::renderer::pipelines::create_pipelines(&device, &config)?;

        // Build font atlas now
        let font_size = 14.0;
        let font_atlas = FontAtlas::new(&device, &queue, &text_bind_layout, font_size)?;

        // Create a simple shader for textured text (WGSL).
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../text_shader.wgsl").into()),
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
        // Delegate resize/reconfigure to renderer::surface (move-only refactor).
        crate::renderer::surface::resize_surface(&self.surface, &self.device, &mut self.config, new_size)?;
        debug!("Reconfigured surface to {}x{}", self.config.width, self.config.height);
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
    pub fn render_with_layout(&mut self, _app_state: &AppState, layout: &RenderLayout, render_panels: &[zaroxi_app::view_model::RenderPanel]) -> Result<(), RenderError> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }
        debug!(
            "entering render_with_layout (window {}x{}), render_panels={}",
            self.config.width,
            self.config.height,
            render_panels.len()
        );

        // Log received render panels for traceability (debug only).
        if RENDER_DEBUG {
            for p in render_panels {
                debug!("renderer received render_panel id='{}' title='{}' visible={}", p.id, p.title, p.visible);
            }
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
        // When true, avoid any scissor operations (if present). Disabled by default.
        const DIAGNOSTIC_DISABLE_SCISSOR: bool = false;
        // Optional forced text color when DIAGNOSTIC_TEXT_ONLY is enabled.
        // Set to Some([r,g,b,a]) to force all text to a bright color for visibility.
        const DIAGNOSTIC_FORCE_TEXT_COLOR: Option<[f32; 4]> = None;
        // DIAGNOSTIC_FULLSCREEN_QUAD: inject a full-screen solid quad into the
        // shape (panel) vertex list to validate render-pass / pipeline state.
        // Disabled by default to avoid contaminating normal rendering.
        const DIAGNOSTIC_FULLSCREEN_QUAD: bool = false;
        // DIAGNOSTIC_INJECT_CENTER_TEXT: inject a single small diagnostic quad
        // into the text vertex list, centered on screen (NDC) to validate text path.
        // Disabled by default to avoid contaminating normal rendering.
        const DIAGNOSTIC_INJECT_CENTER_TEXT: bool = false;
        if render_debug_enabled() {
            log::debug!("debug geometry injection enabled={}, FORCE_DIAGNOSTIC_COLORS={}, DIAGNOSTIC_TEXT_ONLY={}, DIAGNOSTIC_DISABLE_SCISSOR={}, DIAGNOSTIC_FULLSCREEN_QUAD={}, DIAGNOSTIC_INJECT_CENTER_TEXT={}",
                DEBUG_RENDER, FORCE_DIAGNOSTIC_COLORS, DIAGNOSTIC_TEXT_ONLY, DIAGNOSTIC_DISABLE_SCISSOR, DIAGNOSTIC_FULLSCREEN_QUAD, DIAGNOSTIC_INJECT_CENTER_TEXT);
        }

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

        // Convert render panels into visible content quads and text.
        if render_debug_enabled() {
            log::debug!("[renderer] render_panels count = {}", render_panels.len());
        }

        // Debug injection flag: keeps visual debug geometry and debug pass off by default.
        // Set to `true` when you need to re-enable the quick NDC/vertex layout checks.
        const DEBUG_RENDER: bool = false;
        if render_debug_enabled() {
            log::debug!("debug geometry injection enabled={}", DEBUG_RENDER);
        }

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

        // VALIDATION SCENE: when enabled inject three large horizontal bands (R/G/B)
        // at the top of the shape list to validate the shape pipeline end-to-end.
        if VALIDATION_SCENE {
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
            );
        }

        // For each panel supplied by the app, create a header and content block and queue title/content text.
        let header_h = 28.0f32;
        let content_padding = 8.0f32;
        for panel in render_panels.iter() {
            if RENDER_DEBUG {
                debug!("drawing panel id='{}' title='{}' visible={}", panel.id, panel.title, panel.visible);
            }
            if !panel.visible {
                if RENDER_DEBUG {
                    debug!("panel '{}' is hidden; skipping", panel.id);
                }
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
            if render_debug_enabled() {
                log::debug!("panel '{}' scissor_rect = {:?}", panel.id, target);
            }

            // Delegate header/content quad generation to the shapes module.
            // The shapes::queue_panel_quads helper will compute header and content
            // colors (using semantic colors), perform one-shot CPU logging for
            // those colors and push the appropriate quads into the provided
            // vertex/index vectors. It returns the base vertex index for the
            // content quad (if any) so the caller can still emit the packed
            // vertex dump that used to live here.
            let base_idx_opt = crate::renderer::shapes::queue_panel_quads(
                &mut panel_verts,
                &mut panel_indices,
                panel,
                target,
                &sem,
                width,
                height,
            );

            // One-shot packed-vertex dump for the sidebar (shows final pos/uv/color pushed to the buffer).
            if let Some(base_idx) = base_idx_opt {
                if panel.id.as_str() == "sidebar" && LOGGED_SIDEBAR_PACKED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                    if panel_verts.len() >= base_idx + 4 {
                        let v0 = panel_verts[base_idx];
                        let v1 = panel_verts[base_idx + 1];
                        let v2 = panel_verts[base_idx + 2];
                        let v3 = panel_verts[base_idx + 3];
                        info!(
                            "packed sidebar verts: \
                             v0 pos=({:.3},{:.3}) uv=({:.3},{:.3}) color=({:.3},{:.3},{:.3},{:.3}); \
                             v1 pos=({:.3},{:.3}) uv=({:.3},{:.3}) color=({:.3},{:.3},{:.3},{:.3}); \
                             v2 pos=({:.3},{:.3}) uv=({:.3},{:.3}) color=({:.3},{:.3},{:.3},{:.3}); \
                             v3 pos=({:.3},{:.3}) uv=({:.3},{:.3}) color=({:.3},{:.3},{:.3},{:.3})",
                            v0.pos[0], v0.pos[1], v0.uv[0], v0.uv[1], v0.color[0], v0.color[1], v0.color[2], v0.color[3],
                            v1.pos[0], v1.pos[1], v1.uv[0], v1.uv[1], v1.color[0], v1.color[1], v1.color[2], v1.color[3],
                            v2.pos[0], v2.pos[1], v2.uv[0], v2.uv[1], v2.color[0], v2.color[1], v2.color[2], v2.color[3],
                            v3.pos[0], v3.pos[1], v3.uv[0], v3.uv[1], v3.color[0], v3.color[1], v3.color[2], v3.color[3],
                        );
                    }
                }
            }

            // Queue header/title text
            let title_x = target.x + 8.0;
            let title_y = target.y + 6.0;
            // When running diagnostics we may force a single bright color for all text
            let title_color: [f32; 4] = if DIAGNOSTIC_TEXT_ONLY {
                DIAGNOSTIC_FORCE_TEXT_COLOR.unwrap_or([1.0, 1.0, 1.0, 1.0])
            } else {
                [0.95, 0.95, 0.95, 1.0]
            };
            let _ = self.emit_text(&mut text_verts, &mut text_indices, title_x, title_y, &panel.title, title_color, width, height);

            // Emit a single diagnostic log for the first non-space glyph in the titlebar.
            // This does not alter geometry - it only logs placement/UV for quick verification.
            if panel.id.as_str() == "titlebar" {
                if let Some(ch) = panel.title.chars().find(|c| !c.is_whitespace()) {
                    if let Some(g) = self.font_atlas.glyphs.get(&ch) {
                        let x0_px = title_x + g.xoffset as f32;
                        let y0_px = title_y + g.yoffset as f32;
                        let x1_px = x0_px + g.width as f32;
                        let y1_px = y0_px + g.height as f32;
                        let u0 = g.u0;
                        let v0 = g.v0;
                        let u1 = g.u1;
                        let v1 = g.v1;
                        // convert to NDC for quick validation
                        let ndc_x0 = (x0_px / width) * 2.0 - 1.0;
                        let ndc_y0 = 1.0 - (y0_px / height) * 2.0;
                        let ndc_x1 = (x1_px / width) * 2.0 - 1.0;
                        let ndc_y1 = 1.0 - (y1_px / height) * 2.0;
                        // Emit this sample glyph debug line only once to avoid per-frame spam.
                        if FIRST_GLYPH_LOGGED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() && render_debug_enabled() {
                            log::debug!(
                                "sample glyph debug: char='{}' screen_rect=({:.1},{:.1})-({:.1},{:.1}) ndc_rect=({:.4},{:.4})-({:.4},{:.4}) uv=({:.4},{:.4})-({:.4},{:.4}) advance={:.3}",
                                ch, x0_px, y0_px, x1_px, y1_px, ndc_x0, ndc_y0, ndc_x1, ndc_y1, u0, v0, u1, v1, g.advance
                            );
                        }
                    }
                }
            }

            // Queue body/content text (first line only, if any)
            if !panel.content.is_empty() {
                let hh = header_h.min(target.h.max(0.0));
                let cx = target.x + content_padding;
                let cy = target.y + hh + content_padding;
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
            if RENDER_DEBUG {
                let quad_count = (panel_verts.len() / 4) as usize;
                debug!(
                    "panel '{}' queued: panel_quads={} panel_verts={} panel_indices={} text_verts={} text_indices={}",
                    panel.id,
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

        // Temporary diagnostics: log geometry counts to help diagnose missing text.
        // These logs are intentionally concise and should be safe in normal runs.
        let panel_indices_len = panel_indices.len() as u32;
        let total_indices_len = indices.len() as u32;
        info!(
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
                log::debug!("vertex OOB summary: total_verts={} out_of_bounds={}", verts.len(), oob_count);
            } else {
                log::debug!("vertex positions all within expected NDC/pixel ranges (no obvious OOB)");
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
                    i, v.pos[0], v.pos[1], v.uv[0], v.uv[1], v.color[0], v.color[1], v.color[2], v.color[3]
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
                        log::debug!("debug pass enabled={}", DEBUG_RENDER);
                    }

                    // If DEBUG_RENDER is enabled, draw the full scene with the debug
                    // solid-color pipeline (no textures/samplers) to validate geometry.
                    if DEBUG_RENDER {
                        rpass.set_pipeline(&self.debug_pipeline);
                        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

                        let total_indices_len = indices.len() as u32;
                        if total_indices_len == 0 {
                            let verts_to_draw = verts.len() as u32;
                            if RENDER_DEBUG {
                                debug!("debug non-indexed draw (full): verts={}", verts_to_draw);
                            }
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
                            if render_debug_enabled() {
                                log::debug!("shape pass indexed draw (suboptimal path): indices_drawn={}", panel_indices_len);
                            }
                            crate::renderer::shapes::submit_shape_pass(&mut rpass, &self.shape_pipeline, &self.vertex_buffer, &self.index_buffer, panel_indices_len);
                        }
                    } else {
                        if render_debug_enabled() {
                            log::debug!("DIAGNOSTIC_TEXT_ONLY enabled (suboptimal path): skipping shape pass");
                        }
                    }

                    // TEXT PASS: draw glyph/text geometry using the text pipeline and font atlas.
                    if total_indices_len > panel_indices_len {
                        if DISABLE_TEXT_PASS {
                            if render_debug_enabled() {
                                log::debug!("DISABLE_TEXT_PASS enabled: skipping text pass (would draw {} indices)", total_indices_len - panel_indices_len);
                            }
                        } else {
                            if render_debug_enabled() {
                                log::debug!("binding text pipeline and font_atlas bind_group for text pass (DIAGNOSTIC_TEXT_ONLY={})", DIAGNOSTIC_TEXT_ONLY);
                            }
                            crate::renderer::text::submit_text_pass(&mut rpass, &self.text_pipeline, &self.font_atlas, &self.vertex_buffer, &self.index_buffer, panel_indices_len, total_indices_len);
                        }
                    }
                }

                crate::renderer::surface::submit_and_present(&self.queue, encoder, frame);
                if render_debug_enabled() {
                    log::debug!("submitted frame");
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
                            if RENDER_DEBUG {
                                debug!("shape pass indexed draw (suboptimal path): indices_drawn={}", panel_indices_len);
                            }
                            crate::renderer::shapes::submit_shape_pass(&mut rpass, &self.shape_pipeline, &self.vertex_buffer, &self.index_buffer, panel_indices_len);
                        }
                    } else {
                        info!("DIAGNOSTIC_TEXT_ONLY enabled (suboptimal path): skipping shape pass");
                    }

                    // TEXT PASS
                    if total_indices_len > panel_indices_len {
                        if DISABLE_TEXT_PASS {
                            if render_debug_enabled() {
                                log::debug!("DISABLE_TEXT_PASS enabled (suboptimal path): skipping text pass (would draw {} indices)", total_indices_len - panel_indices_len);
                            }
                        } else {
                            if render_debug_enabled() {
                                log::debug!("binding text pipeline and font_atlas bind_group for text pass (suboptimal path, DIAGNOSTIC_TEXT_ONLY={})", DIAGNOSTIC_TEXT_ONLY);
                            }
                            crate::renderer::text::submit_text_pass(&mut rpass, &self.text_pipeline, &self.font_atlas, &self.vertex_buffer, &self.index_buffer, panel_indices_len, total_indices_len);
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

    /// Emit text into the provided vertex/index arrays using the font atlas.
    fn emit_text(&self, verts: &mut Vec<Vertex>, indices: &mut Vec<u16>, x: f32, y: f32, text: &str, color: [f32;4], screen_w: f32, screen_h: f32) -> Result<(), RenderError> {
        crate::renderer::text::emit_text(&self.font_atlas, verts, indices, x, y, text, color, screen_w, screen_h)
    }
}
