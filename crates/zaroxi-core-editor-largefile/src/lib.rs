//! Streaming large-file document access.
//!
//! When a file is too large to load into a `Rope` (>10 000 lines or >1 MB),
//! this module provides a seek+read strategy: line offsets are indexed with
//! SIMD `memchr` in a background thread, and viewport lines are fetched on
//! demand via `File::seek` + `BufReader::read_line`.  Only the scrollable
//! viewport touches RSS; the rest stays on disk.
//!
//! # Unsafe-free design
//!
//! Uses only `std::fs::File`, `std::io::BufReader`, and `std::io::Seek`.
//! No `memmap2`, no raw pointers, no unsafe.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use memchr::memchr_iter;

/// A large file accessed via seek+read — only touched lines enter RSS.
///
/// RSS profile for a 1.1 M-line file:
///  - line-offset `Vec<u32>`: ~4.3 MB  (always in RSS)
///  - `BufReader` buffer:    8 KB     (fixed)
///  - per-line `String`:     ~2 KB    (viewport only)
///  - total overhead:        ~4.5 MB
pub struct StreamedDocument {
    file: File,
    line_offsets: Vec<u32>,
    total_lines: usize,
    path: PathBuf,
    byte_size: u64,
}

impl StreamedDocument {
    /// Open the file (instant — no scanning). The offset index is empty
    /// until [`index_lines`] is called.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let byte_size = file.metadata()?.len();
        Ok(Self {
            file,
            line_offsets: Vec::new(),
            total_lines: 0,
            path: path.to_path_buf(),
            byte_size,
        })
    }

    /// Scan the file once with `BufReader` + `memchr` to build the
    /// line-offset index. Call in a background thread (50–150 ms for
    /// 1 M lines). Reports progress via the callback every 200 k lines.
    pub fn index_lines(&mut self, mut progress: impl FnMut(usize)) -> io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;

        let mut reader = BufReader::with_capacity(256 * 1024, &self.file);
        let mut offsets: Vec<u32> =
            Vec::with_capacity((self.byte_size as usize / 40).min(4_000_000));
        offsets.push(0);

        let mut abs_pos: usize = 0;
        let mut line_count: usize = 0;

        loop {
            let buf = reader.fill_buf()?;
            if buf.is_empty() {
                break;
            }
            for pos in memchr_iter(b'\n', buf) {
                let next_abs = abs_pos + pos + 1;
                if (next_abs as u64) < self.byte_size {
                    offsets.push(next_abs as u32);
                }
                line_count += 1;
                if line_count % 200_000 == 0 {
                    progress(line_count);
                }
            }
            let consumed = buf.len();
            reader.consume(consumed);
            abs_pos += consumed;
        }

        self.line_offsets = offsets;
        self.total_lines = line_count;
        progress(line_count);
        Ok(())
    }

    /// Total line count (0 until `index_lines` completes).
    pub fn total_lines(&self) -> usize {
        self.total_lines
    }

    /// File path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Raw byte count of the file.
    pub fn byte_len(&self) -> usize {
        self.byte_size as usize
    }

    /// Estimated RSS of the line index alone (KB).
    pub fn index_rss_kb(&self) -> usize {
        self.line_offsets.len() * core::mem::size_of::<u32>() / 1024
    }

    /// Read one line via `seek` + `read_line`.  Only this line's bytes
    /// enter userspace RSS (plus the `BufReader` buffer, 8 KB fixed).
    /// Takes `&mut self` because `File::seek` updates the fd position.
    pub fn line(&mut self, line_idx: usize) -> String {
        if line_idx >= self.line_offsets.len() {
            return String::new();
        }
        let offset = self.line_offsets[line_idx] as u64;
        if self.file.seek(SeekFrom::Start(offset)).is_err() {
            return String::new();
        }
        let mut reader = BufReader::with_capacity(8192, &self.file);
        let mut s = String::new();
        let _ = reader.read_line(&mut s);
        if s.ends_with('\n') {
            s.pop();
        }
        if s.ends_with('\r') {
            s.pop();
        }
        s
    }

    /// Viewport lines `[first..last]` as `Vec<String>`.
    pub fn viewport_lines(&mut self, first: usize, last: usize) -> Vec<String> {
        let last = last.min(self.total_lines.saturating_sub(1));
        (first..=last).map(|i| self.line(i)).collect()
    }
}
