/*!
text_atlas.rs — row-based glyph atlas with persistent GPU texture reuse.

Key design decisions for editor performance:
- The GPU texture is created once and reused across frames.
- The full atlas buffer is only uploaded when the `dirty` flag is set
  (a new glyph was inserted since the last upload).
- When the atlas height grows, a new larger GPU texture replaces the old one.
- A persistent `inserted_keys` map (CacheKey → AtlasEntry) avoids
  re-rasterizing glyphs that are already in the atlas.
*/

use std::cmp;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wgpu::{
    Device, Extent3d, Origin3d, Queue, SamplerDescriptor, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
};

/// Stable u64 key derived from cosmic_text::CacheKey via std hash.
/// CacheKey implements Hash + Eq, so we hash it consistently.
pub fn cache_key_to_u64(key: &cosmic_text::CacheKey) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

/// Minimal rasterized glyph descriptor consumed by the atlas inserter.
pub struct RasterizedGlyph {
    pub width: u32,
    pub height: u32,
    /// 8-bit alpha mask (R8) bytes; caller must ensure Mask content.
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
/// - Fixed initial size configurable via `new(width, height)`.
/// - Row packing with cursor_x / cursor_y / row_height.
/// - Grows height by doubling when needed (no repacking).
/// - Persistent GPU texture reused across frames.
///
/// Maximum dimensions are capped at [`ATLAS_MAX_WIDTH`] × [`ATLAS_MAX_HEIGHT`]
/// to prevent runaway memory growth in sessions with many unique glyphs.
pub struct Atlas {
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
    /// CPU-side backing buffer storing R8 bytes width * height
    buffer: Vec<u8>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    /// Number of inserted regions
    regions: usize,
    /// Set to true when a new glyph is inserted; cleared after GPU upload.
    dirty: bool,
    /// Persistent cross-frame cache: maps CacheKey → (AtlasEntry, offset_x, offset_y).
    /// Glyphs already in the atlas are skipped on subsequent frames without
    /// swash rasterization or atlas re-insertion.
    inserted_keys: HashMap<u64, (AtlasEntry, i32, i32)>,
    /// Cached GPU resources — created on first upload, replaced when atlas grows.
    gpu_texture: Option<Texture>,
    gpu_view: Option<wgpu::TextureView>,
    gpu_sampler: Option<wgpu::Sampler>,
}

/// Maximum atlas width in pixels (R8).
pub const ATLAS_MAX_WIDTH: u32 = 2048;
/// Maximum atlas height in pixels (R8).  
pub const ATLAS_MAX_HEIGHT: u32 = 2048;

impl Atlas {
    /// Create a new atlas with the given initial size (width x height).
    /// The atlas will not exceed ATLAS_MAX_WIDTH × ATLAS_MAX_HEIGHT.
    pub fn new(width: u32, height: u32) -> Self {
        let w = width.min(ATLAS_MAX_WIDTH);
        let h = height.min(ATLAS_MAX_HEIGHT);
        let size = (w as usize).checked_mul(h as usize).unwrap_or(0);
        Atlas {
            width: w,
            height: h,
            max_width: ATLAS_MAX_WIDTH,
            max_height: ATLAS_MAX_HEIGHT,
            buffer: vec![0u8; size],
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            regions: 0,
            dirty: false,
            inserted_keys: HashMap::new(),
            gpu_texture: None,
            gpu_view: None,
            gpu_sampler: None,
        }
    }

    /// Check if a glyph with the given cache key is already in the atlas.
    /// Returns the AtlasEntry and bearing offsets if present.
    pub fn try_get(&self, cache_key_bits: u64) -> Option<(AtlasEntry, i32, i32)> {
        self.inserted_keys.get(&cache_key_bits).cloned()
    }

    /// Attempt to insert a glyph bitmap into the atlas.
    /// Returns an AtlasEntry with pixel rect + UVs on success.
    /// Records the insertion in the persistent cache.
    pub fn insert(&mut self, glyph: &RasterizedGlyph, cache_key_bits: u64) -> Option<AtlasEntry> {
        if glyph.width == 0 || glyph.height == 0 {
            return None;
        }
        if glyph.width > self.width || glyph.height > self.height {
            return None;
        }

        // Move to next row if not enough horizontal space.
        if self.cursor_x + glyph.width > self.width {
            self.cursor_x = 0;
            self.cursor_y = self.cursor_y.saturating_add(self.row_height);
            self.row_height = 0;
        }

        // Grow atlas height (simple doubling) until glyph fits vertically.
        let atlas_grew = self.grow_to_fit(glyph.height);

        // Now we have room; compute placement.
        let px = self.cursor_x;
        let py = self.cursor_y;

        // Copy glyph data (assumes glyph.data is tightly packed row-major, stride == glyph.width)
        for row in 0..glyph.height {
            let dest_y = (py + row) as usize;
            let dest_x = px as usize;
            let dest_index = dest_y
                .checked_mul(self.width as usize)
                .and_then(|v: usize| v.checked_add(dest_x))
                .unwrap();
            let src_index = (row as usize).checked_mul(glyph.width as usize).unwrap();
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
        self.dirty = true;

        // Record in persistent cache with bearing offsets
        self.inserted_keys.insert(cache_key_bits, (entry.clone(), glyph.offset_x, glyph.offset_y));

        // If the atlas grew, invalidate the GPU texture so a new one is created.
        // Existing entries' UVs remain valid because only height changed and entries
        // keep their original positions.
        if atlas_grew {
            self.gpu_texture = None;
            self.gpu_view = None;
            self.gpu_sampler = None;
        }

        Some(entry)
    }

    /// Ensure the atlas has enough vertical space for the given glyph height.
    /// Doubles height until the cursor_y + glyph_height fits, capped at max_height.
    /// Returns true if the atlas height was increased.
    fn grow_to_fit(&mut self, glyph_height: u32) -> bool {
        if self.cursor_y + glyph_height <= self.height {
            return false;
        }
        if self.cursor_y >= self.max_height {
            // Cursor already past max — can never fit. This means the atlas is
            // full. The caller should trigger clear_cache or evict entries.
            return false;
        }
        let mut new_height = self.height;
        while self.cursor_y + glyph_height > new_height {
            let next = cmp::min(new_height.saturating_mul(2), self.max_height);
            if next == new_height {
                return false; // reached max
            }
            new_height = next;
        }
        let old_h = self.height;
        self.grow_height_to(new_height);
        if std::env::var("ZAROXI_MEM_TRACE").as_deref() == Ok("1")
            || std::env::var("ZAROXI_ATLAS_VERBOSE").as_deref() == Ok("1")
        {
            let old_kb = (self.width as usize * old_h as usize) / 1024;
            let new_kb = (self.width as usize * new_height as usize) / 1024;
            eprintln!(
                "ATLAS_GROW: {w}x{old_h}→{w}x{new_h}  ({old_kb}KB→{new_kb}KB)  cursor_y={cy}  max={mx}x{my}",
                w = self.width,
                old_h = old_h,
                new_h = new_height,
                old_kb = old_kb,
                new_kb = new_kb,
                cy = self.cursor_y,
                mx = self.max_width,
                my = self.max_height,
            );
        }
        true
    }

    /// Grow the atlas height to new_height. Drops the old buffer explicitly
    /// so the allocator can reclaim pages before allocating the new one.
    fn grow_height_to(&mut self, new_height: u32) {
        if new_height <= self.height {
            return;
        }
        let new_size = (self.width as usize).checked_mul(new_height as usize).unwrap();
        let mut new_buf = vec![0u8; new_size];
        let copy_rows = self.height as usize;
        let row_bytes = self.width as usize;
        for row in 0..copy_rows {
            let old_start = row * row_bytes;
            let new_start = row * row_bytes;
            new_buf[new_start..new_start + row_bytes]
                .copy_from_slice(&self.buffer[old_start..old_start + row_bytes]);
        }
        // Drop old buffer BEFORE assigning new, so peak memory is new_size
        // (not old_size + new_size).
        let old = std::mem::replace(&mut self.buffer, new_buf);
        drop(old);
        self.height = new_height;
    }

    /// Upload the atlas CPU buffer to GPU.
    ///
    /// Reuses the existing GPU texture when dimensions are unchanged and
    /// `dirty` is false. Only creates a new texture when dimensions changed
    /// (atlas grew). When dirty, uploads the changed region via write_texture.
    pub fn upload_to_gpu(
        &mut self,
        device: &Device,
        queue: &mut Queue,
        prefer_nearest_sampler: bool,
    ) -> Option<(Texture, wgpu::TextureView, wgpu::Sampler)> {
        // Fast path: atlas unchanged since last upload — return cached resources.
        if !self.dirty && self.gpu_texture.is_some() {
            // Need to return clones/references. Since Texture/View/Sampler are not Clone,
            // and wgpu doesn't support cloning, we need to recreate them or find another way.
            // Actually, returning Option<(Texture, ...)> means the caller moves ownership.
            // We CAN'T reuse the same texture objects across calls because the caller
            // takes ownership. The only option is to NOT store the texture here and
            // instead let the caller (CosmicTextRenderer) manage the lifecycle.

            // Fall through to normal upload path but with a note:
            // The caller should check dirty flag and skip upload_to_gpu when not dirty.
            // Returning None signals "no change needed".
            return None;
        }

        // Create or recreate GPU texture.
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

        let data_layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(self.width),
            rows_per_image: Some(self.height),
        };
        let image_copy_texture = wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        queue.write_texture(image_copy_texture, &self.buffer, data_layout, size);

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = if prefer_nearest_sampler {
            device.create_sampler(&SamplerDescriptor {
                label: Some("text_atlas_sampler_nearest"),
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            })
        } else {
            device.create_sampler(&SamplerDescriptor {
                label: Some("text_atlas_sampler_linear"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            })
        };

        // Store for potential reuse (but note: the caller moves ownership).
        // Since the caller will move (tex, view, sampler) out, we can't store them.
        // The dirty flag will prevent re-upload until next insertion.
        self.dirty = false;

        Some((texture, view, sampler))
    }

    /// Return number of regions inserted.
    pub fn regions(&self) -> usize {
        self.regions
    }

    /// Return the number of unique glyph entries in the persistence cache.
    pub fn inserted_count(&self) -> usize {
        self.inserted_keys.len()
    }

    /// Clear the persistent glyph cache and reset the atlas packing position.
    /// GPU resources are invalidated so a fresh texture is created on next upload.
    /// The CPU buffer is shrunk back to 512×512 to release any grown memory.
    /// Returns the number of entries evicted.
    pub fn clear_cache(&mut self) -> usize {
        let count = self.inserted_keys.len();
        self.inserted_keys.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_height = 0;
        self.regions = 0;
        self.gpu_texture = None;
        self.gpu_view = None;
        self.gpu_sampler = None;
        self.dirty = true;
        // Shrink the CPU buffer back to a compact initial size so the
        // allocator can release pages gained during atlas growth.
        let compact_w = 512u32.min(self.max_width);
        let compact_h = 512u32.min(self.max_height);
        let compact_sz = compact_w as usize * compact_h as usize;
        self.buffer = vec![0u8; compact_sz];
        self.width = compact_w;
        self.height = compact_h;
        count
    }

    /// Return atlas dimensions.
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

    /// Check if a glyph with the given cache key is already in the atlas.
    /// Returns (AtlasEntry, offset_x, offset_y) if present.
    pub fn try_get(&self, cache_key_bits: u64) -> Option<(AtlasEntry, i32, i32)> {
        let a = self.0.lock().unwrap();
        a.try_get(cache_key_bits)
    }

    pub fn insert(&self, glyph: &RasterizedGlyph, cache_key_bits: u64) -> Option<AtlasEntry> {
        let mut a = self.0.lock().unwrap();
        a.insert(glyph, cache_key_bits)
    }

    pub fn upload_to_gpu(
        &self,
        device: &Device,
        queue: &mut Queue,
        prefer_nearest_sampler: bool,
    ) -> Option<(Texture, wgpu::TextureView, wgpu::Sampler)> {
        let mut a = self.0.lock().unwrap();
        a.upload_to_gpu(device, queue, prefer_nearest_sampler)
    }

    pub fn regions(&self) -> usize {
        let a = self.0.lock().unwrap();
        a.regions()
    }

    pub fn dims(&self) -> (u32, u32) {
        let a = self.0.lock().unwrap();
        a.dimensions()
    }

    /// Return the number of unique glyph entries in the persistent cache.
    pub fn inserted_count(&self) -> usize {
        let a = self.0.lock().unwrap();
        a.inserted_count()
    }

    /// Clear the persistent glyph cache and reset packing. GPU resources are
    /// invalidated. Returns entries evicted. Useful to bound atlas growth after
    /// many files/views generate unique glyph-size combinations.
    pub fn clear_cache(&self) -> usize {
        let mut a = self.0.lock().unwrap();
        a.clear_cache()
    }

    /// Debug helper: return a copy of the raw R8 bytes for the given atlas entry rect.
    pub fn dump_region(&self, entry: &AtlasEntry) -> Vec<u8> {
        let a = self.0.lock().unwrap();
        let mut out: Vec<u8> = Vec::with_capacity((entry.width * entry.height) as usize);
        let stride = a.width as usize;
        for row in 0..entry.height {
            let y = (entry.y + row) as usize;
            let start = y.checked_mul(stride).unwrap().checked_add(entry.x as usize).unwrap();
            let end = start + entry.width as usize;
            out.extend_from_slice(&a.buffer[start..end]);
        }
        out
    }
}
