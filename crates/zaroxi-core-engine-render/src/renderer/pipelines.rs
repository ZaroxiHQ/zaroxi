use crate::error::RenderError;
use crate::renderer::geometry::Vertex;
use log::info;
use wgpu::{BindGroupLayout, Device, SurfaceConfiguration};

/// Create the bind-group layouts and render pipelines used by the renderer.
///
/// Move-only refactor: this function extracts shader module/pipeline creation
/// from core.rs into a dedicated module. Behavior is preserved.
pub(crate) fn create_pipelines(
    device: &Device,
    config: &SurfaceConfiguration,
) -> Result<
    (BindGroupLayout, wgpu::RenderPipeline, wgpu::RenderPipeline, wgpu::RenderPipeline),
    RenderError,
> {
    // Create bind group layout for font atlas (texture + sampler)
    let text_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            // sampled texture (R8). For single-channel R8Unorm atlases we mark the
            // texture as non-filterable so platforms that disallow linear filtering
            // for single-channel formats won't clash with the bind layout.
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            },
            // sampler: use NonFiltering in the bind layout to match the non-filterable texture.
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            },
        ],
        label: Some("text_bind_layout"),
    });

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
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        // wgpu 29 uses multiview_mask & cache fields
        multiview_mask: None,
        cache: None,
    });

    info!("text pipeline created: color_format={:?}, blend=ALPHA_BLENDING", config.format);

    // Create a minimal solid-color pipeline for debug-only draws.
    // This pipeline does not sample any textures or use bind groups.
    let debug_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("debug-color-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../debug_color_shader.wgsl").into()),
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
    // non-text UI geometry (panels, borders, dividers).
    let shape_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("shape-color-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shape_shader.wgsl").into()),
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

    Ok((text_bind_layout, text_pipeline, debug_pipeline, shape_pipeline))
}
