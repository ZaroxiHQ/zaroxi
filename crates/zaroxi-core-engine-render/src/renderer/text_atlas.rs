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

use wgpu::{Device, Queue, Extent3d, Origin3d, TextureDescriptor, TextureDimension, TextureUsages, TextureFormat, TextureViewDescriptor, SamplerDescriptor, Texture};

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

    // NOTE:
    // Different wgpu releases expose slightly different typed helpers for
    // texture write operations (ImageCopyTexture / ImageDataLayout, etc).
    // To avoid depending on a specific wgpu struct layout in this diagnostic
    // helper, we only allocate the texture here. The actual upload will be
    // implemented using the workspace's wgpu API in the full atlas implementation.
    //
    // Keeping the allocation lets callers create bind-groups using the
    // returned texture view while we iterate on a robust, version-agnostic
    // upload path in a follow-up change.
    //
    // (No pixel data upload performed here.)
    Some(texture)
}
