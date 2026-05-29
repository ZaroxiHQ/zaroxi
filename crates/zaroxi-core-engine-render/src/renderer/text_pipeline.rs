/*!
text_pipeline.rs - helpers for text pipeline bind groups and samplers.

This small module centralizes creation of sampling resources compatible with the
text pipeline created in renderer::pipelines. It currently exposes helpers to
create a sampler and to build a bind group once a compatible BindGroupLayout is
available. Keeping this logic here avoids scattering sampler creation across
the renderer codebase.
*/

use wgpu::{Device, BindGroupLayout, BindGroup, TextureView, SamplerDescriptor, BindGroupEntry, BindGroupDescriptor, BindingResource};

/// Create a default filtering sampler for text atlas sampling.
pub fn create_default_text_sampler(device: &Device) -> wgpu::Sampler {
    device.create_sampler(&SamplerDescriptor {
        label: Some("text_atlas_sampler"),
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
        BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(texture_view),
        },
        BindGroupEntry {
            binding: 1,
            resource: BindingResource::Sampler(sampler),
        },
    ];

    device.create_bind_group(&BindGroupDescriptor {
        label: Some("text_atlas_bind_group"),
        layout,
        entries,
    })
}
