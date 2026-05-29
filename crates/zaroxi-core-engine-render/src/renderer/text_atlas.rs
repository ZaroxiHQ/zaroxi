/*!
text_atlas.rs - small helper for glyph atlas creation and upload (placeholder)

This module contains minimal helpers used by the CosmicTextRenderer to
create a debug atlas texture and (in future) to pack per-glyph bitmaps
into a single GPU texture.

Current responsibilities:
- Provide a small debug-atlas creation helper used during diagnostics.
- Keep atlas packing/raster details encapsulated so a future full
  implementation can be added without touching renderer core.

Note: This file intentionally contains only small, well-scoped helpers.
Full glyph packing, eviction, and multi-page atlas support will be
implemented in a follow-up change.
*/

use wgpu::{Device, Queue, Extent3d, ImageDataLayout, ImageCopyTexture, Origin3d, TextureDescriptor, TextureDimension, TextureUsages, TextureFormat, TextureViewDescriptor, SamplerDescriptor, Texture};

/// Create a tiny 2x2 RGBA debug atlas and upload the provided bytes.
///
/// Returns the created texture (so callers can create bind-groups as needed).
pub fn create_debug_atlas(device: &Device, queue: &mut Queue, format: TextureFormat) -> Option<Texture> {
    let pixel_bytes: [u8; 16] = [
        255, 255, 255, 255, // opaque white
        0, 0, 0, 0,         // transparent
        0, 0, 0, 0,         // transparent
        255, 255, 255, 255, // opaque white
    ];

    let size = Extent3d { width: 2, height: 2, depth_or_array_layers: 1 };

    let tex_desc = TextureDescriptor {
        label: Some("debug_text_atlas"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    };

    let texture = device.create_texture(&tex_desc);

    let image_copy = ImageCopyTexture {
        texture: &texture,
        mip_level: 0,
        origin: Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    };

    let layout = ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(std::num::NonZeroU32::new(4 * 2).unwrap()),
        rows_per_image: Some(std::num::NonZeroU32::new(2).unwrap()),
    };

    queue.write_texture(image_copy, &pixel_bytes, layout, size);

    Some(texture)
}
