//! Large-file document access.
//!
//! # Types
//!
//! - [`StreamedDocument`]: read-only seek+read access.  Only the visible
//!   viewport enters RSS.  Best for viewing-only (logs, generated files).
//! - [`PieceTable`]: editable piece-table backed large file.  Original
//!   content loaded once into `Vec<u8>`; edits go into an append-only buffer.
//! - [`DocumentBuffer`]: enum unifying `ropey::Rope` (small files) and
//!   [`PieceTable`] (large files) behind a common API.
//!
//! # Unsafe-free design
//!
//! Uses only `std::fs`, `std::io`, and `memchr`.  No `memmap2`, no raw
//! pointers, no unsafe.

pub mod piece_table;

use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use memchr::memchr_iter;

pub use piece_table::PieceTable;

/// A large file accessed via seek+read — only touched lines enter RSS.
///
/// RSS profile for a 1.1 M-line file:
///  - line-offset `Vec<u32>`: ~4.3 MB  (always in RSS)
///  - `BufReader` buffer:    8 KB     (fixed)
///  - per-line `String`:     ~2 KB    (viewport only)
///  - total overhead:        ~4.5 MB
pub struct StreamedDocument {
    file: std::fs::File,
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

// ── DocumentBuffer — unified small + large file interface ────────────────

/// Backend-agnostic document buffer.
///
/// Small files (<1 MB) use `ropey::Rope` for optimal frequent-edit
/// performance.  Large files (≥1 MB) use [`PieceTable`] for lower
/// memory overhead.  The render and edit pipelines use this enum
/// without knowing which backend is active.
pub enum DocumentBuffer {
    Rope(ropey::Rope),
    Large(PieceTable),
}

impl DocumentBuffer {
    /// Threshold in bytes above which a file is considered large.
    pub const LARGE_THRESHOLD: u64 = 1024 * 1024;

    /// Open a file, choosing the appropriate backend based on size.
    /// Call in a background thread — never blocks the render thread.
    pub fn open(path: &Path) -> io::Result<Self> {
        let meta = std::fs::metadata(path)?;
        if meta.len() >= Self::LARGE_THRESHOLD {
            PieceTable::open(path).map(Self::Large)
        } else {
            let text = fs::read_to_string(path)?;
            Ok(Self::Rope(ropey::Rope::from_str(&text)))
        }
    }

    pub fn total_lines(&self) -> usize {
        match self {
            Self::Rope(r) => r.len_lines(),
            Self::Large(pt) => pt.total_lines(),
        }
    }

    pub fn total_bytes(&self) -> usize {
        match self {
            Self::Rope(r) => r.len_bytes(),
            Self::Large(pt) => pt.total_bytes(),
        }
    }

    /// Viewport lines `[first..last]` as `Vec<(line_idx, content)>`.
    pub fn lines_in_range(&self, first: usize, last: usize) -> Vec<(usize, String)> {
        match self {
            Self::Rope(r) => {
                let total = r.len_lines();
                let last = last.min(total.saturating_sub(1));
                (first..=last)
                    .map(|i| {
                        let line = r.line(i);
                        let s = line
                            .as_str()
                            .unwrap_or("")
                            .trim_end_matches('\n')
                            .trim_end_matches('\r')
                            .to_owned();
                        (i, s)
                    })
                    .collect()
            }
            Self::Large(pt) => pt.lines_in_range(first, last),
        }
    }

    /// Convert (line, col) to byte offset for edit positioning.
    pub fn line_col_to_byte_offset(&self, line: usize, col: usize) -> usize {
        match self {
            Self::Rope(r) => {
                let byte = r.line_to_byte(line.min(r.len_lines().saturating_sub(1)));
                let line_bytes =
                    r.line(line.min(r.len_lines().saturating_sub(1))).as_str().unwrap_or("").len();
                (byte + col.min(line_bytes)).min(r.len_bytes())
            }
            Self::Large(pt) => pt.line_col_to_byte_offset(line, col),
        }
    }

    pub fn insert(&mut self, byte_offset: usize, text: &str) {
        match self {
            Self::Rope(r) => {
                let idx = r.byte_to_char(byte_offset.min(r.len_bytes()));
                r.insert(idx, text);
            }
            Self::Large(pt) => pt.insert(byte_offset, text),
        }
    }

    pub fn delete(&mut self, start: usize, end: usize) {
        match self {
            Self::Rope(r) => {
                let s = r.byte_to_char(start.min(r.len_bytes()));
                let e = r.byte_to_char(end.min(r.len_bytes()));
                if s < e {
                    r.remove(s..e);
                }
            }
            Self::Large(pt) => pt.delete(start, end),
        }
    }

    pub fn is_modified(&self) -> bool {
        match self {
            Self::Rope(_) => false,
            Self::Large(pt) => pt.is_modified,
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        match self {
            Self::Rope(r) => {
                use std::io::Write;
                let mut f = fs::File::create(path)?;
                for chunk in r.chunks() {
                    f.write_all(chunk.as_bytes())?;
                }
                f.flush()?;
                Ok(())
            }
            Self::Large(pt) => pt.save(path),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Rope(_) => None,
            Self::Large(pt) => Some(pt.path.as_path()),
        }
    }
}
