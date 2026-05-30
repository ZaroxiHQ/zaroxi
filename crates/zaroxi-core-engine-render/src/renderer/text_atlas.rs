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

use wgpu::{Device, Queue, Texture};

/// NOTE: Atlas packing/upload is intentionally NOT implemented in this helper.
///
/// The project previously used a tiny 2x2 placeholder atlas which masked the
/// real problem: glyph bitmaps were never packed/uploaded, producing zero-area
/// atlas entries and invisible text. Until a real atlas packer and upload
/// pipeline is implemented this function returns None to make the lack of a
/// real atlas explicit and to avoid pretending success.
///
/// Implementing a production atlas requires:
/// - a packing strategy (skyline/bin-pack, shelf, or shelf+eviction),
/// - a staging buffer -> copy_buffer_to_texture upload path,
/// - growth/resize semantics (repack or multi-page atlases),
/// - correct TextureFormat selection (R8Unorm for single-channel glyph coverage).
///
/// See the renderer's TODOs and the issue tracker for follow-ups.
pub fn create_debug_atlas(_device: &Device, _queue: &mut Queue, _format: wgpu::TextureFormat) -> Option<Texture> {
    // Deliberately return None to signal "atlas packing not implemented yet".
    None
}
