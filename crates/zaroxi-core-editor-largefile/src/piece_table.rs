//! Piece-table document buffer for large editable files.
//!
//! The piece table stores the original file content in an immutable
//! `Vec<u8>` and appends all edits to a `String`.  An ordered list of
//! `Piece` structures describes the current document.  A line-offset
//! cache (rebuilt lazily with `memchr` SIMD) provides O(1) line access.
//!
//! Memory for a 308k-line / ~20 MB file:
//!  - `original`: ~20 MB  (loaded once, never grows)
//!  - `added`:    0 KB    (grows only with actual edits)
//!  - `pieces`:   ~48 B   (starts at 1 piece)
//!  - line cache: ~2.5 MB  (308k × 8 B)
//!  - total:      ~23 MB   (vs ~60 MB for ropey)

use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use memchr::memchr_iter;

#[derive(Clone, Debug)]
struct Piece {
    source: Source,
    start: usize,
    len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Source {
    Original,
    Added,
}

/// Piece-table document buffer.
pub struct PieceTable {
    original: Vec<u8>,
    added: String,
    pieces: Vec<Piece>,
    line_cache: RefCell<Vec<usize>>,
    line_cache_valid: RefCell<bool>,
    total_bytes: usize,
    pub is_modified: bool,
    pub path: PathBuf,
}

impl PieceTable {
    pub fn open(path: &Path) -> io::Result<Self> {
        let original = fs::read(path)?;
        let total_bytes = original.len();
        let pieces = if total_bytes > 0 {
            vec![Piece { source: Source::Original, start: 0, len: total_bytes }]
        } else {
            vec![]
        };
        Ok(Self {
            original,
            added: String::new(),
            pieces,
            line_cache: RefCell::new(Vec::new()),
            line_cache_valid: RefCell::new(false),
            total_bytes,
            is_modified: false,
            path: path.to_path_buf(),
        })
    }

    // ── internal helpers ────────────────────────────────────

    fn source_bytes(&self, source: Source) -> &[u8] {
        match source {
            Source::Original => &self.original,
            Source::Added => self.added.as_bytes(),
        }
    }

    fn piece_bytes(&self, p: &Piece) -> &[u8] {
        let src = self.source_bytes(p.source);
        let end = (p.start + p.len).min(src.len());
        &src[p.start..end]
    }

    fn piece_at_offset(&self, mut offset: usize) -> (usize, usize) {
        for (i, piece) in self.pieces.iter().enumerate() {
            if offset <= piece.len {
                return (i, offset);
            }
            offset -= piece.len;
        }
        (self.pieces.len(), 0)
    }

    // ── line cache ──────────────────────────────────────────

    fn rebuild_line_cache(&self) {
        let mut cache = self.line_cache.borrow_mut();
        cache.clear();
        cache.push(0);
        let mut doc_offset = 0usize;
        let mut nl_offsets: Vec<usize> = Vec::new();
        for piece in &self.pieces {
            let bytes = self.piece_bytes(piece);
            for nl_pos in memchr_iter(b'\n', bytes) {
                let next = doc_offset + nl_pos + 1;
                if next < self.total_bytes {
                    nl_offsets.push(next);
                }
            }
            doc_offset += piece.len;
        }
        cache.extend(&nl_offsets);
        *self.line_cache_valid.borrow_mut() = true;
    }

    fn ensure_line_cache(&self) {
        if !*self.line_cache_valid.borrow() {
            self.rebuild_line_cache();
        }
    }

    // ── public API ──────────────────────────────────────────

    pub fn total_lines(&self) -> usize {
        self.ensure_line_cache();
        self.line_cache.borrow().len()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }

    pub fn original_bytes(&self) -> usize {
        self.original.len()
    }

    pub fn added_bytes(&self) -> usize {
        self.added.len()
    }

    pub fn line(&self, line_idx: usize) -> String {
        self.ensure_line_cache();
        let cache = self.line_cache.borrow();
        if line_idx >= cache.len() {
            return String::new();
        }
        let start_byte = cache[line_idx];
        let end_byte = cache.get(line_idx + 1).copied().unwrap_or(self.total_bytes);
        let len = end_byte.saturating_sub(start_byte);
        let mut result = Vec::with_capacity(len);
        let mut remaining_start = start_byte;
        let mut remaining_len = len;
        let mut doc_offset = 0usize;
        for piece in &self.pieces {
            if remaining_len == 0 {
                break;
            }
            let piece_end = doc_offset + piece.len;
            if piece_end <= remaining_start {
                doc_offset = piece_end;
                continue;
            }
            let local_start = remaining_start.saturating_sub(doc_offset);
            let local_end = (remaining_start + remaining_len - doc_offset).min(piece.len);
            let pb = self.piece_bytes(piece);
            result.extend_from_slice(&pb[local_start..local_end.min(pb.len())]);
            let taken = local_end - local_start;
            remaining_len = remaining_len.saturating_sub(taken);
            remaining_start = doc_offset + local_end;
            doc_offset = piece_end;
        }
        if result.last() == Some(&b'\n') {
            result.pop();
        }
        if result.last() == Some(&b'\r') {
            result.pop();
        }
        String::from_utf8_lossy(&result).into_owned()
    }

    pub fn lines_in_range(&self, start: usize, end: usize) -> Vec<(usize, String)> {
        self.ensure_line_cache();
        let cache = self.line_cache.borrow();
        let last = end.min(cache.len().saturating_sub(1));
        let indices: Vec<usize> = (start..=last).collect();
        drop(cache);
        indices.into_iter().map(|i| (i, self.line(i))).collect()
    }

    pub fn line_col_to_byte_offset(&self, line: usize, col: usize) -> usize {
        self.ensure_line_cache();
        let cache = self.line_cache.borrow();
        let line_start = cache.get(line).copied().unwrap_or(self.total_bytes);
        (line_start + col).min(self.total_bytes)
    }

    // ── editing ─────────────────────────────────────────────

    pub fn insert(&mut self, byte_offset: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let byte_offset = byte_offset.min(self.total_bytes);
        let added_start = self.added.len();
        self.added.push_str(text);
        let text_len = text.len();
        let new_piece = Piece { source: Source::Added, start: added_start, len: text_len };
        let (piece_idx, offset_in_piece) = self.piece_at_offset(byte_offset);
        if piece_idx >= self.pieces.len() {
            self.pieces.push(new_piece);
        } else if offset_in_piece == 0 {
            self.pieces.insert(piece_idx, new_piece);
        } else if offset_in_piece == self.pieces[piece_idx].len {
            self.pieces.insert(piece_idx + 1, new_piece);
        } else {
            let orig = self.pieces[piece_idx].clone();
            let left = Piece { source: orig.source, start: orig.start, len: offset_in_piece };
            let right = Piece {
                source: orig.source,
                start: orig.start + offset_in_piece,
                len: orig.len - offset_in_piece,
            };
            self.pieces.splice(piece_idx..=piece_idx, [left, new_piece, right]);
        }
        self.total_bytes += text_len;
        self.is_modified = true;
        *self.line_cache_valid.borrow_mut() = false;
    }

    pub fn delete(&mut self, start_offset: usize, end_offset: usize) {
        if start_offset >= end_offset || start_offset >= self.total_bytes {
            return;
        }
        let end_offset = end_offset.min(self.total_bytes);
        let delete_len = end_offset - start_offset;
        let (start_piece, start_in_piece) = self.piece_at_offset(start_offset);
        if start_piece >= self.pieces.len() {
            return;
        }
        if start_in_piece > 0 && start_in_piece < self.pieces[start_piece].len {
            let orig = self.pieces[start_piece].clone();
            let left = Piece { source: orig.source, start: orig.start, len: start_in_piece };
            let right = Piece {
                source: orig.source,
                start: orig.start + start_in_piece,
                len: orig.len - start_in_piece,
            };
            self.pieces.splice(start_piece..=start_piece, [left, right]);
        }
        let delete_start = if start_in_piece > 0 { start_piece + 1 } else { start_piece };
        let mut remaining = delete_len;
        let i = delete_start;
        while i < self.pieces.len() && remaining > 0 {
            if self.pieces[i].len <= remaining {
                remaining -= self.pieces[i].len;
                self.pieces.remove(i);
            } else {
                self.pieces[i].start += remaining;
                self.pieces[i].len -= remaining;
                remaining = 0;
            }
        }
        self.total_bytes = self.total_bytes.saturating_sub(delete_len);
        self.is_modified = true;
        *self.line_cache_valid.borrow_mut() = false;
    }

    // ── persistence ─────────────────────────────────────────

    pub fn save(&self, path: &Path) -> io::Result<()> {
        use std::io::Write;
        let mut file = fs::File::create(path)?;
        for piece in &self.pieces {
            file.write_all(self.piece_bytes(piece))?;
        }
        file.flush()?;
        Ok(())
    }
}
