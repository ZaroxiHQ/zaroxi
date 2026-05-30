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

use std::cmp;
use std::sync::{Arc, Mutex};
use wgpu::{Device, Queue, Texture, Extent3d, Origin3d, TextureDescriptor, TextureDimension, TextureUsages, TextureFormat, ImageCopyTexture, ImageDataLayout, TextureViewDescriptor, SamplerDescriptor};

/// Minimal rasterized glyph descriptor consumed by the atlas inserter.
pub struct RasterizedGlyph {
    pub width: u32,
    pub height: u32,
    /// 8-bit alpha mask (R8) or RGBA bytes depending on content; caller must ensure Mask content.
    pub data: Vec<u8>,
    /// horizontal bearing / left
    pub offset_x: i32,
    /// vertical bearing / top
    pub offset_y: i32,
}

/// Atlas insertion result (pixel rect + UVs).
#[derive(Clone, Debug)]
pub struct AtlasEntry {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Simple row-based atlas supporting R8Unorm glyph bitmaps.
///
/// - Fixed initial size (1024x1024)
/// - Row packing with cursor_x / cursor_y / row_height
/// - Grows height by doubling when needed (no repacking)
pub struct Atlas {
    width: u32,
    height: u32,
    /// CPU-side backing buffer storing R8 bytes width * height
    buffer: Vec<u8>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    /// Number of inserted regions (best-effort)
    regions: usize,
    /// Optional GPU-side metadata (not persisted here); upload returns Texture/View/Sampler
}

impl Atlas {
    /// Create a new atlas with the given initial size (width x height).
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width.checked_mul(height).unwrap_or(0)) as usize;
        Atlas {
            width,
            height,
            buffer: vec![0u8; size],
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            regions: 0,
        }
    }

    /// Attempt to insert a glyph bitmap into the atlas.
    /// Returns an AtlasEntry with pixel rect + UVs on success.
    pub fn insert(&mut self, glyph: &RasterizedGlyph) -> Option<AtlasEntry> {
        if glyph.width == 0 || glyph.height == 0 {
            return None;
        }
        if glyph.width > self.width || glyph.height > self.height {
            // glyph is larger than atlas dimensions: insertion not possible
            return None;
        }

        // Move to next row if not enough horizontal space.
        if self.cursor_x + glyph.width > self.width {
            self.cursor_x = 0;
            self.cursor_y = self.cursor_y.saturating_add(self.row_height);
            self.row_height = 0;
        }

        // Grow atlas height (simple doubling) until glyph fits vertically.
        while self.cursor_y + glyph.height > self.height {
            // Double height, but avoid overflow.
            let new_height = cmp::min(self.height.saturating_mul(2), 16384);
            if new_height == self.height {
                // cannot grow further
                return None;
            }
            self.grow_height_to(new_height);
        }

        // Now we have room; compute placement.
        let px = self.cursor_x;
        let py = self.cursor_y;

        // Copy glyph data (assumes glyph.data is tightly packed row-major, stride == glyph.width)
        for row in 0..glyph.height {
            let dest_y = (py + row) as usize;
            let dest_x = px as usize;
            let dest_index = dest_y
                .checked_mul(self.width as usize)
                .and_then(|v| v.checked_add(dest_x))
                .unwrap();
            let src_index = (row as usize)
                .checked_mul(glyph.width as usize)
                .unwrap();
            let src_slice = &glyph.data[src_index..src_index + glyph.width as usize];
            self.buffer[dest_index..dest_index + glyph.width as usize].copy_from_slice(src_slice);
        }

        // Build AtlasEntry
        let entry = AtlasEntry {
            u0: (px as f32) / (self.width as f32),
            v0: (py as f32) / (self.height as f32),
            u1: ((px + glyph.width) as f32) / (self.width as f32),
            v1: ((py + glyph.height) as f32) / (self.height as f32),
            x: px,
            y: py,
            width: glyph.width,
            height: glyph.height,
        };

        // Advance cursor and row_height
        self.cursor_x = self.cursor_x.saturating_add(glyph.width);
        if glyph.height > self.row_height {
            self.row_height = glyph.height;
        }
        self.regions += 1;

        Some(entry)
    }

    /// Grow the atlas height to new_height. Copies existing buffer into new buffer.
    fn grow_height_to(&mut self, new_height: u32) {
        if new_height <= self.height { return; }
        let new_size = (self.width as usize).checked_mul(new_height as usize).unwrap();
        let mut new_buf = vec![0u8; new_size];
        // Copy old rows
        for row in 0..self.height as usize {
            let old_start = row * self.width as usize;
            let new_start = row * self.width as usize;
            new_buf[new_start..new_start + self.width as usize]
                .copy_from_slice(&self.buffer[old_start..old_start + self.width as usize]);
        }
        self.buffer = new_buf;
        self.height = new_height;
    }

    /// Upload the atlas CPU buffer into a GPU texture (R8Unorm) and return Texture, View, Sampler.
    /// Overwrites full texture contents via queue.write_texture.
    pub fn upload_to_gpu(&self, device: &Device, queue: &mut Queue) -> Option<(Texture, wgpu::TextureView, wgpu::Sampler)> {
        // Create texture descriptor for R8Unorm
        let size = Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 };
        let tex_desc = TextureDescriptor {
            label: Some("text_atlas_r8"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&tex_desc);

        // Write texture via queue.write_texture using tightly packed rows (bytes_per_row = width)
        let image_copy = ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d { x: 0, y: 0, z: 0 },
            aspect: wgpu::TextureAspect::All,
        };

        let layout = ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(std::num::NonZeroU32::new(self.width).unwrap()),
            rows_per_image: Some(std::num::NonZeroU32::new(self.height).unwrap()),
        };

        // write_texture expects &[u8] with length = width*height
        queue.write_texture(image_copy, &self.buffer, layout, size);

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor::default());

        Some((texture, view, sampler))
    }

    /// Return number of regions inserted (best-effort)
    pub fn regions(&self) -> usize {
        self.regions
    }

    /// Return atlas dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Thread-safe wrapper for Atlas to be shared by renderer.
#[derive(Clone)]
pub struct SharedAtlas(Arc<Mutex<Atlas>>);

impl SharedAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        SharedAtlas(Arc::new(Mutex::new(Atlas::new(width, height))))
    }

    pub fn insert(&self, glyph: &RasterizedGlyph) -> Option<AtlasEntry> {
        let mut a = self.0.lock().unwrap();
        a.insert(glyph)
    }

    pub fn upload_to_gpu(&self, device: &Device, queue: &mut Queue) -> Option<(Texture, wgpu::TextureView, wgpu::Sampler)> {
        let a = self.0.lock().unwrap();
        a.upload_to_gpu(device, queue)
    }

    pub fn regions(&self) -> usize {
        let a = self.0.lock().unwrap();
        a.regions()
    }

    pub fn dims(&self) -> (u32,u32) {
        let a = self.0.lock().unwrap();
        a.dimensions()
    }
}
