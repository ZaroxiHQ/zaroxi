//! Large-file document access without rope piece-table overhead.
//!
//! When a file is too large to load into a `Rope` (>10 000 lines or >1 MB),
//! this module loads it as an owned `Vec<u8>` and indexes line offsets with
//! SIMD `memchr` in a background thread so the render thread never blocks.
//!
//! Compared to the piece-table `Rope` path this avoids:
//!  - `inserts: Vec<String>` (empty for unedited files, but allocated)
//!  - `pieces: Vec<Piece>` (grows with every edit)
//!  - `InMemoryBufferStore::inner: HashMap<String, String>` duplicate storage
//!  - per-line `String` heap allocations in `Vec<String>` intermediate form
//!
//! The full file bytes live in a single `Vec<u8>`; line lookups return
//! `&str` slices into it (zero-copy after the initial load).
//!
//! For a 1.1 M-line file at ~80 chars/line:
//!  - raw bytes:         ~88 MB
//!  - line-offset Vec:    ~4.3 MB
//!  - total doc overhead: ~92 MB
//!  - vs rope + store:   ~300+ MB for the same file

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use memchr::memchr_iter;

/// An owned large file with pre-computed line offsets.
///
/// All fields are `Send` so the struct can travel from the background loader
/// thread to the render thread through an `mpsc::channel`.
pub struct MappedDocument {
    /// Owned file bytes (loaded once, never mutated).
    data: Vec<u8>,
    /// Byte offset of each line start within `data`.
    line_offsets: Vec<u32>,
    total_lines: usize,
    path: PathBuf,
}

impl MappedDocument {
    /// Load the file and index line offsets in a background thread.
    ///
    /// Uses `memchr` SIMD for fast `\n` scanning.  For a 1 M-line file
    /// this takes ~30–60 ms on the background worker.
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let data = fs::read(path)?;

        let byte_count = data.len();
        let mut offsets: Vec<u32> = Vec::with_capacity(byte_count / 40);
        offsets.push(0);

        for pos in memchr_iter(b'\n', &data) {
            if pos + 1 < byte_count {
                offsets.push((pos + 1) as u32);
            }
        }
        let total_lines = offsets.len();

        Ok(Self { data, line_offsets: offsets, total_lines, path: path.to_path_buf() })
    }

    /// Total line count.
    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    /// File path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of raw bytes of file content.
    pub fn byte_len(&self) -> usize {
        self.data.len()
    }

    /// Get a single line as `&str` — zero-copy into the owned buffer.
    ///
    /// Returns `""` for out-of-range indices or non-UTF-8 content.
    pub fn line(&self, line_idx: usize) -> &str {
        if line_idx >= self.line_offsets.len() {
            return "";
        }
        let start = self.line_offsets[line_idx] as usize;
        let end = if line_idx + 1 < self.line_offsets.len() {
            self.line_offsets[line_idx + 1] as usize
        } else {
            self.data.len()
        };
        let mut slice = &self.data[start..end.min(self.data.len())];
        slice = slice.strip_suffix(b"\n").unwrap_or(slice);
        slice = slice.strip_suffix(b"\r").unwrap_or(slice);
        std::str::from_utf8(slice).unwrap_or("")
    }

    /// Return lines `[first..last]` as `Vec<String>` for the existing
    /// `ContentViewModel` plumbing.
    pub fn viewport_lines(&self, first: usize, last: usize) -> Vec<String> {
        let last = last.min(self.total_lines.saturating_sub(1));
        (first..=last).map(|i| self.line(i).to_string()).collect()
    }
}
