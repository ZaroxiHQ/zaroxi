/*!
text_pipeline.rs - helpers for text pipeline bind groups and samplers.

This small module centralizes creation of sampling resources compatible with the
text pipeline created in renderer::pipelines. It currently exposes helpers to
create a sampler and to build a bind group once a compatible BindGroupLayout is
available. Keeping this logic here avoids scattering sampler creation across
the renderer codebase.
*/

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Device,
    SamplerDescriptor, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexStepMode,
};

/// Create a default filtering sampler for text atlas sampling.
pub fn create_default_text_sampler(device: &Device) -> wgpu::Sampler {
    // Use linear filtering for smoother mask sampling when scaled.
    device.create_sampler(&SamplerDescriptor {
        label: Some("text_atlas_sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    })
}

/// Build a bind group for an atlas texture & sampler using the provided layout.
///
/// The caller is expected to pass a BindGroupLayout that matches the one used by
/// the text pipeline. This helper keeps the construction site concise.
pub fn build_atlas_bind_group(
    device: &Device,
    layout: &BindGroupLayout,
    texture_view: &TextureView,
    sampler: &wgpu::Sampler,
) -> BindGroup {
    let entries = &[
        BindGroupEntry { binding: 0, resource: BindingResource::TextureView(texture_view) },
        BindGroupEntry { binding: 1, resource: BindingResource::Sampler(sampler) },
    ];

    device.create_bind_group(&BindGroupDescriptor {
        label: Some("text_atlas_bind_group"),
        layout,
        entries,
    })
}

/// Return the instance vertex buffer layout expected by the text pipeline.
///
/// Instance layout (per-instance, step mode = Instance):
/// - location(0) pos_ndc: Float32x2  offset 0
/// - location(1) size_ndc:Float32x2  offset 8
/// - location(2) uv0:     Float32x2  offset 16
/// - location(3) uv1:     Float32x2  offset 24
/// - location(4) color:   Float32x4  offset 32
///
/// stride = 48 bytes
pub fn instance_buffer_layout() -> VertexBufferLayout<'static> {
    // Leak a small static slice for the attribute descriptors; repeated calls are cheap.
    let attrs: &'static [VertexAttribute] = Box::leak(Box::new([
        VertexAttribute { offset: 0, shader_location: 0, format: VertexFormat::Float32x2 },
        VertexAttribute { offset: 8, shader_location: 1, format: VertexFormat::Float32x2 },
        VertexAttribute { offset: 16, shader_location: 2, format: VertexFormat::Float32x2 },
        VertexAttribute { offset: 24, shader_location: 3, format: VertexFormat::Float32x2 },
        VertexAttribute { offset: 32, shader_location: 4, format: VertexFormat::Float32x4 },
    ]));
    VertexBufferLayout { array_stride: 48, step_mode: VertexStepMode::Instance, attributes: attrs }
}
