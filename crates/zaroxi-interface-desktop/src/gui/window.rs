/*!
Minimal GUI-3 window bootstrap.

Responsibilities:
- Provide a small, interface-facing runner that opens a native window (winit),
  initializes wgpu, and paints simple colored rectangles for the ShellFrame
  regions produced by ShellFrame::new(size).
- Keep layout and widget semantics in the interface crate; rendering here is a
  narrow adapter for visualization and manual verification.
- Fall back to a transcript-only mode in environments where windowing cannot be
  initialized (for CI or headless runs).

This file is intentionally self-contained and minimal. It uses pollster to
synchronously initialize wgpu in order to keep the public API synchronous.
*/

use std::error::Error;
use std::num::NonZeroU32;
use std::time::Duration;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

use crate::gui::{ShellFrame, Size};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    // position in normalized device coordinates (x,y)
    pos: [f32; 2],
    // color rgb
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x3
    ];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Public runner: attempt to open a native window and render the shell. Returns
/// an Err if windowing/wgpu initialization fails; callers may fall back to a
/// transcript-only path when that happens.
pub fn run_shell_window(shell: ShellFrame) -> Result<(), Box<dyn Error>> {
    // Try to build the event loop and window. If this fails (e.g. in headless CI),
    // return an error and allow the caller to fallback to transcript printing.
    let event_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Zaroxi - GUI Shell")
        .with_inner_size(PhysicalSize::new(shell.size.width, shell.size.height))
        .with_resizable(true);

    let window = match wb.build(&event_loop) {
        Ok(w) => w,
        Err(e) => return Err(Box::new(e)),
    };

    // Initialize wgpu synchronously using pollster.
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) }?;

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .ok_or_else(|| "No suitable GPU adapter found")?;

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::downlevel_defaults(),
            label: Some("zaroxi_gui_device"),
        },
        None,
    ))?;

    let size = window.inner_size();
    let surface_format = surface.get_supported_formats(&adapter)[0];
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    // Build shader module (simple vertex + fragment)
    let shader_src = r#"
struct VertexInput {
    @location(0) pos: vec2<f32>;
    @location(1) color: vec3<f32>;
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>;
    @location(0) color: vec3<f32>;
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.pos, 0.0, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("zaroxi_chrome_shader"),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    // Create pipeline
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("zaroxi_pipeline_layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("zaroxi_render_pipeline"),
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
                blend: Some(wgpu::BlendState::alpha_blending()),
                write_mask: wgpu::ColorWrites::ALL,
            })],
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
        multiview: None,
    });

    // Build vertex data from shell regions
    fn rect_to_vertices(rect: &crate::gui::Rect, win_w: f32, win_h: f32, color: [f32; 3]) -> [Vertex; 6] {
        // Convert pixel-space rect (x,y,w,h) to NDC coords (-1..1).
        let left = rect.x as f32 / win_w * 2.0 - 1.0;
        let right = (rect.x + rect.width) as f32 / win_w * 2.0 - 1.0;
        // y: top-to-bottom in pixels; NDC y is bottom-to-top, so invert.
        let top = 1.0 - rect.y as f32 / win_h * 2.0;
        let bottom = 1.0 - (rect.y + rect.height) as f32 / win_h * 2.0;

        [
            // triangle 1
            Vertex { pos: [left, top], color },
            Vertex { pos: [right, top], color },
            Vertex { pos: [left, bottom], color },
            // triangle 2
            Vertex { pos: [left, bottom], color },
            Vertex { pos: [right, top], color },
            Vertex { pos: [right, bottom], color },
        ]
    }

    // helper: pick stable color palette
    let palette: &[[f32; 3]] = &[
        [0.06, 0.10, 0.15], // background (not used per-rect)
        [0.09, 0.28, 0.44], // toolbar
        [0.12, 0.18, 0.25], // rail
        [0.10, 0.14, 0.22], // sidebar
        [0.16, 0.20, 0.28], // editor header
        [0.14, 0.16, 0.22], // editor content
        [0.07, 0.09, 0.12], // minimap
        [0.08, 0.10, 0.14], // dock
        [0.05, 0.08, 0.10], // ai header
        [0.03, 0.05, 0.07], // status
    ];

    let mut vertices: Vec<Vertex> = Vec::new();
    let win_w = config.width as f32;
    let win_h = config.height as f32;
    for (i, r) in shell.regions.iter().enumerate() {
        let color = palette.get(i + 1).copied().unwrap_or([0.12, 0.12, 0.12]);
        let verts = rect_to_vertices(&r.rect, win_w, win_h, color);
        vertices.extend_from_slice(&verts);
    }

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("zaroxi_vertex_buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // Background clear color derived from shell theme surface token (interpreted as hex).
    // Fallback to a dark bluish color if parsing fails.
    fn hex_to_color(s: &str) -> wgpu::Color {
        let s = s.trim_start_matches('#');
        if s.len() == 6 {
            if let Ok(v) = u32::from_str_radix(s, 16) {
                let r = ((v >> 16) & 0xFF) as f64 / 255.0;
                let g = ((v >> 8) & 0xFF) as f64 / 255.0;
                let b = (v & 0xFF) as f64 / 255.0;
                return wgpu::Color { r, g, b, a: 1.0 };
            }
        }
        wgpu::Color { r: 0.03, g: 0.06, b: 0.1, a: 1.0 }
    }

    let clear_color = hex_to_color(shell.theme.surface);

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::RedrawRequested(_) => {
                // Acquire frame
                let frame = match surface.get_current_texture() {
                    Ok(t) => t,
                    Err(e) => {
                        // Reconfigure on Outdated or Lost
                        match e {
                            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                                let size = window.inner_size();
                                config.width = size.width.max(1);
                                config.height = size.height.max(1);
                                surface.configure(&device, &config);
                                return;
                            }
                            wgpu::SurfaceError::Timeout | wgpu::SurfaceError::OutOfMemory => {
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                        }
                    }
                };

                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("zaroxi_encoder"),
                });

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("zaroxi_render_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(clear_color),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    let vertex_count = vertices.len() as u32;
                    if vertex_count > 0 {
                        rpass.draw(0..vertex_count, 0..1);
                    }
                }

                queue.submit(Some(encoder.finish()));
                frame.present();
            }

            Event::MainEventsCleared => {
                // Throttle redraws a bit to avoid busy loops on some platforms.
                window.request_redraw();
                std::thread::sleep(Duration::from_millis(8));
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) => {
                    if physical_size.width > 0 && physical_size.height > 0 {
                        config.width = physical_size.width;
                        config.height = physical_size.height;
                        surface.configure(&device, &config);
                        // Ideally we would rebuild vertex buffer to match new size; for this
                        // minimal phase we keep the original vertices scaled to initial size.
                    }
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    if new_inner_size.width > 0 && new_inner_size.height > 0 {
                        config.width = new_inner_size.width;
                        config.height = new_inner_size.height;
                        surface.configure(&device, &config);
                    }
                }
                _ => {}
            },

            _ => {}
        }
    });
}
