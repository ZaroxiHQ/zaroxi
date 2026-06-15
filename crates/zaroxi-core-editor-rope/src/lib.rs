//! A piece-table-backed rope data structure for efficient text editing.
//!
//! The `Rope` supports:
//! - O(1) construction from a `String`
//! - O(log n) insert, delete, replace at character indices
//! - O(1) line-start lookup via `line_starts` index
//! - O(log L) char→line/col via binary search
//! - Iteration over lines or characters
//!
//! All positions are in **characters** (UTF-32 code points), not bytes.
//! This matches the `TextEdit` semantics used by the workspace service.

use std::fmt;
use std::ops::Range;

fn env_diag_enabled() -> bool {
    std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1")
}

/// A piece referencing a span of the original buffer or an inserted string.
#[derive(Clone, Debug)]
struct Piece {
    /// Which buffer this piece references:
    /// - `None` means the original buffer
    /// - `Some(idx)` means the insert-buffer at index `idx`
    source: Option<usize>,
    /// Byte range within the source buffer.
    byte_range: Range<usize>,
    /// Precomputed char count for this piece.
    char_count: usize,
    /// Precomputed newline count for this piece.
    line_count: usize,
}

/// A piece-table rope.
///
/// The original text is stored once; inserts add new pieces
/// referencing either the original or new insert buffers.
#[derive(Clone)]
pub struct Rope {
    original: String,
    inserts: Vec<String>,
    pieces: Vec<Piece>,
    /// Cached total character count.
    total_chars: usize,
    /// Cached total line count (1-based: empty rope == 1 line).
    total_lines: usize,
    /// Cumulative character offsets where each logical line starts.
    /// `line_starts[0] == 0`, `line_starts.len() == total_lines`.
    /// Maintained incrementally on insert/delete for O(1) line lookup.
    line_starts: Vec<usize>,
}

impl Rope {
    /// Create a new rope from the given text.
    ///
    /// Builds the `line_starts` index by scanning at byte level for newline
    /// bytes (0x0A) while tracking the current character offset.  This
    /// avoids UTF-8 decode overhead on every code point and runs
    /// significantly faster than `text.chars()` for ASCII-dominated source.
    pub fn new(text: &str) -> Self {
        let start_time = std::time::Instant::now();
        let bytes = text.as_bytes();

        let mut char_count = 0usize;
        let mut line_count = 1usize;

        // First pass: count chars and lines so we can pre-allocate
        // line_starts to the exact capacity.
        for &b in bytes {
            if b & 0xC0 != 0x80 {
                char_count += 1;
            }
            if b == b'\n' {
                line_count += 1;
            }
        }

        let mut line_starts = Vec::with_capacity(line_count);
        line_starts.push(0);
        let mut ci = 0usize;
        for &b in bytes {
            if b & 0xC0 != 0x80 {
                ci += 1;
            }
            if b == b'\n' {
                line_starts.push(ci);
            }
        }

        if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
            let elapsed_us = start_time.elapsed().as_micros();
            eprintln!(
                "ZAROXI_DEBUG_LARGE_FILE: Rope::new chars={} lines={} duration_us={}",
                char_count, line_count, elapsed_us,
            );
        }
        let piece = Piece { source: None, byte_range: 0..text.len(), char_count, line_count };
        Self {
            original: text.to_string(),
            inserts: Vec::new(),
            pieces: vec![piece],
            total_chars: char_count,
            total_lines: line_count,
            line_starts,
        }
    }

    /// Create an empty rope.
    pub fn empty() -> Self {
        Self::new("")
    }

    /// Return the total number of characters in the rope.
    pub fn char_count(&self) -> usize {
        self.total_chars
    }

    /// Return the number of newline-delimited lines (at least 1).
    pub fn line_count(&self) -> usize {
        self.total_lines
    }

    /// Return the text content of the rope as a `String`.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut out = String::with_capacity(
            self.original.len() + self.inserts.iter().map(|s| s.len()).sum::<usize>(),
        );
        self.append_to_string(&mut out);
        out
    }

    fn append_to_string(&self, out: &mut String) {
        for piece in &self.pieces {
            let src: &str = match piece.source {
                None => &self.original,
                Some(idx) => &self.inserts[idx],
            };
            out.push_str(&src[piece.byte_range.clone()]);
        }
    }

    /// Substring of the rope from `start` to `end` character indices.
    /// Used for efficient line extraction with the line-starts index.
    pub fn substring_range(&self, start: usize, end: usize) -> String {
        let start = start.min(self.total_chars);
        let end = end.min(self.total_chars);
        if start >= end {
            return String::new();
        }
        let mut chars_seen = 0usize;
        let cap = end - start;
        let mut out = String::with_capacity(cap);
        for piece in &self.pieces {
            let piece_len = piece.char_count;
            if chars_seen + piece_len <= start {
                chars_seen += piece_len;
                continue;
            }
            if chars_seen >= end {
                break;
            }
            let src: &str = match piece.source {
                None => &self.original,
                Some(idx) => &self.inserts[idx],
            };
            let sub = &src[piece.byte_range.clone()];
            for c in sub.chars() {
                if chars_seen >= end {
                    break;
                }
                if chars_seen >= start {
                    out.push(c);
                }
                chars_seen += 1;
            }
        }
        out
    }

    /// Return characters in `[start_char..start_char+count)`.
    pub fn extract_chars(&self, start_char: usize, count: usize) -> String {
        self.substring_range(start_char, start_char + count)
    }

    /// Return joined text of lines `[line_start_idx..line_end_idx)`.
    pub fn visible_lines(&self, line_start_idx: usize, line_end_idx: usize) -> String {
        let start = line_start_idx.min(self.total_lines);
        let end = line_end_idx.min(self.total_lines);
        if start >= end {
            return String::new();
        }
        let char_start = self.line_starts.get(start).copied().unwrap_or(self.total_chars);
        let char_end = self.line_starts.get(end).copied().unwrap_or(self.total_chars);
        let out = self.substring_range(char_start, char_end);
        if out.ends_with('\n') {
            let mut s = out;
            s.pop();
            s
        } else {
            out
        }
    }

    /// Return the iterable count of visible lines (for tab-expanded display).
    pub fn lines_expanded_iter(&self) -> Vec<String> {
        (0..self.total_lines).filter_map(|li| self.line(li)).collect()
    }

    /// Build line_starts from scratch by scanning each piece's characters
    /// in order without materializing the whole document as a String.
    /// Complexity: O(total_chars) — unavoidable for structural edits that
    /// change newline count — but avoids the O(total_chars) allocation
    /// of `to_string()` that the old implementation performed.
    #[allow(dead_code)]
    fn rebuild_line_starts(&mut self) {
        let start_time = if env_diag_enabled() { Some(std::time::Instant::now()) } else { None };
        self.line_starts.clear();
        self.line_starts.push(0);
        let mut ci = 0usize;
        for pi in 0..self.pieces.len() {
            let piece = &self.pieces[pi];
            let src: &str = match piece.source {
                None => &self.original,
                Some(idx) => &self.inserts[idx],
            };
            for c in src[piece.byte_range.clone()].chars() {
                ci += 1;
                if c == '\n' && ci < self.total_chars + 1 {
                    self.line_starts.push(ci);
                }
            }
        }
        if let Some(start) = start_time {
            let elapsed_us = start.elapsed().as_micros();
            eprintln!(
                "ZAROXI_DEBUG_LARGE_FILE: rebuild_line_starts chars={} lines={} duration_us={}",
                self.total_chars,
                self.line_starts.len(),
                elapsed_us,
            );
        }
    }

    /// Insert `text` at the given character index.
    ///
    /// If `char_index` is beyond the end of the rope, text is appended.
    pub fn insert(&mut self, char_index: usize, text: &str) {
        let idx = self.piece_index_for_insert(char_index);
        let offset = self.char_offset_in_piece(idx, char_index);

        let insert_chars = text.chars().count();
        let insert_lines = text.chars().filter(|&c| c == '\n').count();
        let insert_idx = self.inserts.len();
        self.inserts.push(text.to_string());

        let new_piece = Piece {
            source: Some(insert_idx),
            byte_range: 0..text.len(),
            char_count: insert_chars,
            line_count: insert_lines,
        };

        if offset == 0 {
            self.pieces.insert(idx, new_piece);
        } else if offset >= self.pieces[idx].char_count {
            self.pieces.insert(idx + 1, new_piece);
        } else {
            let left = self.split_piece_at(idx, offset);
            self.pieces.insert(left + 1, new_piece);
        }

        self.total_chars += insert_chars;
        self.total_lines += insert_lines;

        if insert_lines == 0 {
            // Content-only edit: shift line_starts entries after insertion point.
            let ins_line = match self.line_starts.binary_search(&char_index) {
                Ok(l) => l,
                Err(l) => l.saturating_sub(1),
            };
            for ls in &mut self.line_starts[ins_line + 1..] {
                *ls += insert_chars;
            }
        } else {
            // Structural edit: incremental line_starts patching.
            // Find the line containing the insertion point.
            let ins_line = match self.line_starts.binary_search(&char_index) {
                Ok(l) => l,
                Err(l) => l.saturating_sub(1),
            };
            // Compute new line start positions from the inserted text.
            // Each newline in text creates a new line starting at
            //   char_index + (position of newline within text) + 1
            // where +1 skips past the newline character.
            let mut new_starts: Vec<usize> = Vec::with_capacity(insert_lines);
            let mut rel_ci = 0usize;
            for c in text.chars() {
                if c == '\n' {
                    new_starts.push(char_index + rel_ci + 1);
                }
                rel_ci += 1;
            }
            // Splice new line starts after the line that was split.
            let splice_at = ins_line + 1;
            let n_new = new_starts.len();
            self.line_starts.splice(splice_at..splice_at, new_starts.into_iter());
            // Shift all subsequent entries by the total inserted character count.
            for ls in &mut self.line_starts[splice_at + n_new..] {
                *ls += insert_chars;
            }
        }
    }

    /// Delete characters in the range `[start, end)` (character indices).
    pub fn delete(&mut self, start: usize, end: usize) {
        if start >= end || start >= self.total_chars {
            return;
        }
        let end = end.min(self.total_chars);

        let first = self.piece_index_for_char(start);
        let first_offset = self.char_offset_in_piece(first, start);

        // Split left side if not at a piece boundary
        let remove_start = if first_offset > 0 {
            self.split_piece_at(first, first_offset);
            first + 1
        } else {
            first
        };

        // Re-find last piece (may have shifted due to left split)
        let end_idx = end.saturating_sub(1);
        let last = self.piece_index_for_char(end_idx);
        let last_offset = self.char_offset_in_piece(last, end_idx);

        // Split right side if not at a piece boundary
        let remove_end = if last_offset + 1 < self.pieces[last].char_count {
            self.split_piece_at(last, last_offset + 1);
            last + 1
        } else {
            last + 1
        };

        let mut removed_chars = 0;
        let mut removed_lines = 0;
        for i in (remove_start..remove_end).rev() {
            removed_chars += self.pieces[i].char_count;
            removed_lines += self.pieces[i].line_count;
            self.pieces.remove(i);
        }

        self.total_chars -= removed_chars;
        self.total_lines -= removed_lines;
        let remove_len = removed_chars;

        if removed_lines == 0 {
            // Content-only delete: shift line_starts entries after deletion point.
            let del_line = match self.line_starts.binary_search(&start) {
                Ok(l) => l,
                Err(l) => l.saturating_sub(1),
            };
            for ls in &mut self.line_starts[del_line + 1..] {
                *ls = ls.saturating_sub(remove_len);
            }
        } else {
            // Structural delete: incremental line_starts patching.
            // Find lines containing start and end-1.
            let del_start_line = match self.line_starts.binary_search(&start) {
                Ok(l) => l,
                Err(l) => l.saturating_sub(1),
            };
            // Remove line_starts entries that fall within the deleted range.
            // These are the entries between del_start_line+1 and
            // del_start_line+1+removed_lines.
            let rm_start = del_start_line + 1;
            let rm_end = (rm_start + removed_lines).min(self.line_starts.len());
            if rm_start < rm_end {
                self.line_starts.drain(rm_start..rm_end);
            }
            // Shift all remaining entries after the deletion zone.
            for ls in &mut self.line_starts[rm_start..] {
                *ls = ls.saturating_sub(remove_len);
            }
        }
    }

    /// Replace characters in `[start, end)` with `text` (equivalent to delete + insert).
    pub fn replace(&mut self, start: usize, end: usize, text: &str) {
        self.delete(start, end);
        self.insert(start, text);
    }

    /// Get the character at the given character index, or None if out of bounds.
    pub fn char_at(&self, char_index: usize) -> Option<char> {
        if char_index >= self.total_chars {
            return None;
        }
        let pi = self.piece_index_for_char(char_index);
        let offset = self.char_offset_in_piece(pi, char_index);
        let piece = &self.pieces[pi];
        let src: &str = match piece.source {
            None => &self.original,
            Some(idx) => &self.inserts[idx],
        };
        for (ci, (_bi, c)) in src[piece.byte_range.clone()].char_indices().enumerate() {
            if ci == offset {
                return Some(c);
            }
        }
        None
    }

    /// Get a line by 0-based index using the line_starts index.  O(line_length).
    pub fn line(&self, line_index: usize) -> Option<String> {
        if line_index >= self.total_lines {
            return None;
        }
        let start = self.line_starts.get(line_index).copied().unwrap_or(self.total_chars);
        let end = if line_index + 1 < self.total_lines {
            self.line_starts[line_index + 1]
        } else {
            self.total_chars
        };
        let mut s = self.substring_range(start, end);
        if s.ends_with('\n') {
            s.pop();
        }
        Some(s)
    }

    /// Get the start character index of the given line.  O(1).
    pub fn line_start(&self, line_index: usize) -> Option<usize> {
        self.line_starts.get(line_index).copied()
    }

    /// Get the end character index of the given line.  O(1).
    pub fn line_end(&self, line_index: usize) -> Option<usize> {
        if line_index >= self.total_lines {
            return None;
        }
        if line_index + 1 < self.total_lines {
            Some(self.line_starts[line_index + 1])
        } else {
            Some(self.total_chars)
        }
    }

    /// Get the length (in characters) of line `line_index`.  O(1).
    pub fn line_length(&self, line_index: usize) -> usize {
        let start = self.line_starts.get(line_index).copied().unwrap_or(self.total_chars);
        let end = if line_index + 1 < self.total_lines {
            self.line_starts[line_index + 1]
        } else {
            self.total_chars
        };
        let len = end.saturating_sub(start);
        if end > start && end >= 1 && len > 0 {
            let text = self.substring_range(end.saturating_sub(1), end);
            if text == "\n" { len.saturating_sub(1) } else { len }
        } else {
            len
        }
    }

    /// Convert a character index to (line, column).  O(log L) via binary search.
    pub fn char_index_to_line_col(&self, char_index: usize) -> (usize, usize) {
        let idx = char_index.min(self.total_chars);
        let line = match self.line_starts.binary_search(&idx) {
            Ok(l) => l,
            Err(l) => l.saturating_sub(1),
        };
        let col = idx.saturating_sub(self.line_starts[line]);
        (line, col)
    }

    /// Convert (line, column) to a character index.  O(1).
    /// Line/column are clamped to valid ranges.
    pub fn line_col_to_char_index(&self, line: usize, col: usize) -> usize {
        let line = line.min(self.total_lines.saturating_sub(1));
        let start = self.line_starts.get(line).copied().unwrap_or(0);
        let line_len = self.line_length(line);
        let col = col.min(line_len);
        start + col
    }

    /// Return an iterator over all lines.
    pub fn lines(&self) -> RopeLines<'_> {
        RopeLines { rope: self, current_line: 0 }
    }

    // ── private helpers ──

    /// Find the piece index containing the given character index.
    fn piece_index_for_char(&self, char_index: usize) -> usize {
        let mut remaining = char_index;
        for (i, piece) in self.pieces.iter().enumerate() {
            if remaining < piece.char_count {
                return i;
            }
            remaining -= piece.char_count;
        }
        self.pieces.len().saturating_sub(1)
    }

    /// Find the piece index and char offset for an insertion or split point.
    /// After the final piece with remaining==0, clamps to the last piece.
    fn piece_index_for_insert(&self, char_index: usize) -> usize {
        let mut remaining = char_index;
        for (i, piece) in self.pieces.iter().enumerate() {
            if remaining <= piece.char_count {
                return i;
            }
            remaining -= piece.char_count;
        }
        self.pieces.len().saturating_sub(1)
    }

    /// Return the character offset within piece at `piece_idx` for char_index.
    fn char_offset_in_piece(&self, piece_idx: usize, char_index: usize) -> usize {
        let mut total = 0usize;
        for i in 0..piece_idx {
            total += self.pieces[i].char_count;
        }
        char_index.saturating_sub(total)
    }

    /// Split piece at `piece_idx` at the given char offset, inserting a new piece
    /// after it. Returns the index of the left piece (which is `piece_idx`).
    fn split_piece_at(&mut self, piece_idx: usize, char_offset: usize) -> usize {
        if char_offset == 0 || char_offset >= self.pieces[piece_idx].char_count {
            return piece_idx;
        }
        let piece = &self.pieces[piece_idx];
        let src: &str = match piece.source {
            None => &self.original,
            Some(idx) => &self.inserts[idx],
        };
        let sub = &src[piece.byte_range.clone()];

        // Walk characters to find byte offset
        let mut byte_offset = 0usize;
        for (ci, (bi, _)) in sub.char_indices().enumerate() {
            if ci == char_offset {
                byte_offset = bi;
                break;
            }
        }

        let left_lines = sub[..byte_offset].chars().filter(|&c| c == '\n').count();
        let right_lines = sub[byte_offset..].chars().filter(|&c| c == '\n').count();

        let left = Piece {
            source: piece.source,
            byte_range: piece.byte_range.start..piece.byte_range.start + byte_offset,
            char_count: char_offset,
            line_count: left_lines,
        };
        let right = Piece {
            source: piece.source,
            byte_range: piece.byte_range.start + byte_offset..piece.byte_range.end,
            char_count: piece.char_count - char_offset,
            line_count: right_lines,
        };
        self.pieces[piece_idx] = left;
        self.pieces.insert(piece_idx + 1, right);
        piece_idx
    }
}

impl fmt::Debug for Rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rope")
            .field("text", &self.to_string())
            .field("chars", &self.total_chars)
            .field("lines", &self.total_lines)
            .finish()
    }
}

// ── Iterator types ──

pub struct RopeLines<'a> {
    rope: &'a Rope,
    current_line: usize,
}

impl<'a> Iterator for RopeLines<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.rope.line(self.current_line)?;
        self.current_line += 1;
        Some(line)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.rope.total_lines.saturating_sub(self.current_line);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for RopeLines<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rope() {
        let r = Rope::empty();
        assert_eq!(r.char_count(), 0);
        assert_eq!(r.line_count(), 1);
        assert_eq!(r.to_string(), "");
    }

    #[test]
    fn new_with_content() {
        let r = Rope::new("hello world");
        assert_eq!(r.char_count(), 11);
        assert_eq!(r.to_string(), "hello world");
    }

    #[test]
    fn insert_at_start() {
        let mut r = Rope::new("world");
        r.insert(0, "hello ");
        assert_eq!(r.to_string(), "hello world");
        assert_eq!(r.char_count(), 11);
    }

    #[test]
    fn insert_at_end() {
        let mut r = Rope::new("hello");
        r.insert(5, " world");
        assert_eq!(r.to_string(), "hello world");
    }

    #[test]
    fn insert_in_middle() {
        let mut r = Rope::new("h world");
        r.insert(1, "ello");
        assert_eq!(r.to_string(), "hello world");
    }

    #[test]
    fn delete_range() {
        let mut r = Rope::new("hello world");
        r.delete(5, 6);
        assert_eq!(r.to_string(), "helloworld");
    }

    #[test]
    fn delete_multiple_chars() {
        let mut r = Rope::new("abcde");
        r.delete(1, 4);
        assert_eq!(r.to_string(), "ae");
    }

    #[test]
    fn replace_text() {
        let mut r = Rope::new("hello world");
        r.replace(6, 11, "rust");
        assert_eq!(r.to_string(), "hello rust");
    }

    #[test]
    fn multiline_rope() {
        let r = Rope::new("line 1\nline 2\nline 3");
        assert_eq!(r.line_count(), 3);
        assert_eq!(r.line(0).unwrap(), "line 1");
        assert_eq!(r.line(1).unwrap(), "line 2");
        assert_eq!(r.line(2).unwrap(), "line 3");
    }

    #[test]
    fn insert_newline() {
        let mut r = Rope::new("ab");
        r.insert(1, "\n");
        assert_eq!(r.line_count(), 2);
        assert_eq!(r.line(0).unwrap(), "a");
        assert_eq!(r.line(1).unwrap(), "b");
    }

    #[test]
    fn delete_newline() {
        let mut r = Rope::new("a\nb");
        r.delete(1, 2);
        assert_eq!(r.to_string(), "ab");
        assert_eq!(r.line_count(), 1);
    }

    #[test]
    fn line_start_index() {
        let r = Rope::new("ab\ncd\nef");
        assert_eq!(r.line_start(0), Some(0));
        assert_eq!(r.line_start(1), Some(3));
        assert_eq!(r.line_start(2), Some(6));
    }

    #[test]
    fn line_lengths() {
        let r = Rope::new("abc\nd\nefgh");
        assert_eq!(r.line_length(0), 3);
        assert_eq!(r.line_length(1), 1);
        assert_eq!(r.line_length(2), 4);
    }

    #[test]
    fn char_index_to_line_col() {
        let r = Rope::new("ab\ncd");
        assert_eq!(r.char_index_to_line_col(0), (0, 0));
        assert_eq!(r.char_index_to_line_col(2), (0, 2));
        assert_eq!(r.char_index_to_line_col(3), (1, 0));
        assert_eq!(r.char_index_to_line_col(5), (1, 2));
    }

    #[test]
    fn line_col_to_char_index() {
        let r = Rope::new("ab\ncd");
        assert_eq!(r.line_col_to_char_index(0, 0), 0);
        assert_eq!(r.line_col_to_char_index(0, 2), 2);
        assert_eq!(r.line_col_to_char_index(1, 0), 3);
        assert_eq!(r.line_col_to_char_index(1, 2), 5);
    }

    #[test]
    fn utf8_multibyte() {
        let r = Rope::new("héllo wörld");
        assert_eq!(r.char_count(), 11);
        assert_eq!(r.char_at(1), Some('é'));
        assert_eq!(r.char_at(7), Some('ö'));
        assert_eq!(r.to_string(), "héllo wörld");
    }

    #[test]
    fn insert_utf8() {
        let mut r = Rope::new("hllo");
        r.insert(1, "é");
        assert_eq!(r.to_string(), "héllo");
        assert_eq!(r.char_count(), 5);
    }

    #[test]
    fn insert_into_utf8_string() {
        let mut r = Rope::new("héllo");
        r.insert(1, "i");
        assert_eq!(r.to_string(), "hiéllo");
        assert_eq!(r.char_count(), 6);
    }

    #[test]
    fn insert_into_utf8_via_set_caret() {
        let mut r = Rope::new("héllo");
        let caret = 1.min(r.char_count());
        r.insert(caret, "i");
        assert_eq!(r.to_string(), "hiéllo");
        assert_eq!(r.char_count(), 6);
    }

    #[test]
    fn delete_utf8() {
        let mut r = Rope::new("héllo");
        r.delete(1, 2);
        assert_eq!(r.to_string(), "hllo");
    }

    #[test]
    fn line_iteration() {
        let r = Rope::new("a\nb\nc");
        let lines: Vec<String> = r.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "a");
        assert_eq!(lines[1], "b");
        assert_eq!(lines[2], "c");
    }

    #[test]
    fn empty_line_iteration() {
        let r = Rope::new("");
        let lines: Vec<String> = r.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "");
    }

    #[test]
    fn trailing_newline() {
        let r = Rope::new("hello\n");
        assert_eq!(r.line_count(), 2);
        assert_eq!(r.line(0).unwrap(), "hello");
        assert_eq!(r.line(1).unwrap(), "");
    }

    #[test]
    fn complex_edit_sequence() {
        let mut r = Rope::new("the quick brown fox");
        r.delete(4, 10); // remove "quick "
        r.insert(4, "slow "); // insert "slow "
        assert_eq!(r.to_string(), "the slow brown fox");
        r.replace(9, 14, "red"); // replace "brown" with "red"
        assert_eq!(r.to_string(), "the slow red fox");
        assert_eq!(r.char_count(), 16);
    }

    #[test]
    fn negative_delete_clamped() {
        let mut r = Rope::new("abc");
        r.delete(5, 10); // start beyond end, should be no-op
        assert_eq!(r.to_string(), "abc");
    }

    #[test]
    fn line_col_clamp() {
        let r = Rope::new("ab\ncd");
        let idx = r.line_col_to_char_index(5, 50); // overshoot
        assert_eq!(idx, r.char_count());
    }
}
