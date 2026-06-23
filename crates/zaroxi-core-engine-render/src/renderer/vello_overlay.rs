//! Cockpit vello overlay composite.
//!
//! Rasterizes a `vello::Scene` (the cockpit UI built by `zaroxi-interface-widgets`)
//! into an `Rgba8Unorm` storage target, then **blits** it over the swapchain
//! surface with an alpha-blended full-screen pass. This is the on-screen half of
//! the deferred "phase 2": vello 0.9 has no `render_to_surface`, so the path is
//! `Scene -> storage target -> alpha-blend blit onto the surface view`.
//!
//! It is driven only when the host enables the cockpit (`ZAROXI_COCKPIT=1`) and a
//! scene is present; otherwise the live render path is untouched.
//!
//! NOTE: this is GPU code validated by compilation only in CI; it requires
//! on-device validation. When inactive the existing GUI is byte-identical.

use vello::peniko::Color;
use vello::{AaConfig, RenderParams, Renderer, RendererOptions, Scene};

/// vello renderer + intermediate target + alpha-blend blit pipeline.
pub struct VelloOverlay {
    renderer: Renderer,
    blit_pipeline: wgpu::RenderPipeline,
    blit_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    target: Option<Target>,
}

struct Target {
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

/// vello requires an `Rgba8Unorm` storage target for `render_to_texture`.
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

const BLIT_WGSL: &str = r#"
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    var out: VsOut;
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    out.uv = vec2<f32>(x, y);
    out.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(src_tex, src_samp, in.uv);
}
"#;

impl VelloOverlay {
    /// Create the overlay: a vello renderer plus the alpha-blend blit pipeline
    /// that composites the vello target onto `surface_format`.
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> Result<Self, vello::Error> {
        let renderer = Renderer::new(device, RendererOptions::default())?;

        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cockpit-blit-layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cockpit-blit-shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cockpit-blit-pipeline-layout"),
            bind_group_layouts: &[Some(&blit_layout)],
            ..Default::default()
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cockpit-blit-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cockpit-blit-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Ok(Self { renderer, blit_pipeline, blit_layout, sampler, target: None })
    }

    fn ensure_target(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let stale = match &self.target {
            Some(t) => t.width != width || t.height != height,
            None => true,
        };
        if stale {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("cockpit-vello-target"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TARGET_FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.target = Some(Target { view, width, height });
        }
    }

    /// Render `scene` into the intermediate target and blit it over
    /// `surface_view` using `encoder` (alpha-blended `LoadOp::Load`, preserving
    /// whatever the main pass already drew).
    pub fn composite(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        scene: &Scene,
        width: u32,
        height: u32,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        self.ensure_target(device, width, height);
        let target = self.target.as_ref().expect("target ensured above");

        // vello submits its own GPU work to `queue` here; on a single queue this
        // completes before the blit pass encoded below is submitted.
        let params = RenderParams {
            base_color: Color::new([0.0, 0.0, 0.0, 0.0]), // transparent: only widgets show
            width,
            height,
            antialiasing_method: AaConfig::Area,
        };
        if self
            .renderer
            .render_to_texture(device, queue, scene, &target.view, &params)
            .is_err()
        {
            return;
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cockpit-blit-bind-group"),
            layout: &self.blit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&target.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cockpit-blit-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // preserve the main GUI underneath
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });
        rpass.set_pipeline(&self.blit_pipeline);
        rpass.set_bind_group(0, &bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }
}
