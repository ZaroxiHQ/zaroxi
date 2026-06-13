//! A piece-table-backed rope data structure for efficient text editing.
//!
//! The `Rope` supports:
//! - O(1) construction from a `String`
//! - O(log n) insert, delete, replace at character indices
//! - Line/column conversion
//! - Iteration over lines or characters
//!
//! All positions are in **characters** (UTF-32 code points), not bytes.
//! This matches the `TextEdit` semantics used by the workspace service.

use std::fmt;
use std::ops::Range;

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
}

impl Rope {
    /// Create a new rope from the given text.
    pub fn new(text: &str) -> Self {
        let char_count = text.chars().count();
        let line_count = text.chars().filter(|&c| c == '\n').count() + 1;
        let piece = Piece { source: None, byte_range: 0..text.len(), char_count, line_count };
        Self {
            original: text.to_string(),
            inserts: Vec::new(),
            pieces: vec![piece],
            total_chars: char_count,
            total_lines: line_count,
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
        let mut ci = 0;
        for (_bi, c) in src[piece.byte_range.clone()].char_indices() {
            if ci == offset {
                return Some(c);
            }
            ci += 1;
        }
        None
    }

    /// Get a line by 0-based index. Returns None if the line index is out of range.
    pub fn line(&self, line_index: usize) -> Option<String> {
        if line_index >= self.total_lines {
            return None;
        }
        let mut current_line = 0usize;
        let mut buf = String::new();
        let total = self.total_chars;
        for ci in 0..=total {
            let c = if ci == total { Some('\n') } else { self.char_at(ci) };
            match c {
                Some('\n') => {
                    if current_line == line_index {
                        return Some(buf);
                    }
                    current_line += 1;
                    buf.clear();
                }
                Some(ch) => {
                    if current_line == line_index {
                        buf.push(ch);
                    }
                }
                None => break,
            }
        }
        if current_line == line_index {
            return Some(buf);
        }
        None
    }

    /// Get the start character index of the given line.
    pub fn line_start(&self, line_index: usize) -> Option<usize> {
        if line_index == 0 {
            return Some(0);
        }
        if line_index >= self.total_lines {
            return None;
        }
        let mut li = 0usize;
        for ci in 0..self.total_chars {
            if li == line_index {
                return Some(ci);
            }
            if let Some('\n') = self.char_at(ci) {
                li += 1;
            }
        }
        if li == line_index {
            return Some(self.total_chars);
        }
        None
    }

    /// Get the length (in characters) of line `line_index`.
    pub fn line_length(&self, line_index: usize) -> usize {
        let start = self.line_start(line_index).unwrap_or(0);
        let next = self.line_start(line_index + 1).unwrap_or(self.total_chars);
        if next > start && line_index + 1 <= self.total_lines {
            // Exclude the trailing newline
            if next > start && self.char_at(next.saturating_sub(1)) == Some('\n') {
                next.saturating_sub(1).saturating_sub(start)
            } else {
                next.saturating_sub(start)
            }
        } else {
            0
        }
    }

    /// Convert a character index to (line, column).
    pub fn char_index_to_line_col(&self, char_index: usize) -> (usize, usize) {
        let idx = char_index.min(self.total_chars);
        let mut line = 0usize;
        let mut col = 0usize;
        for ci in 0..idx {
            match self.char_at(ci) {
                Some('\n') => {
                    line += 1;
                    col = 0;
                }
                Some(_) => {
                    col += 1;
                }
                None => break,
            }
        }
        (line, col)
    }

    /// Convert (line, column) to a character index.
    /// Line/column are clamped to valid ranges.
    pub fn line_col_to_char_index(&self, line: usize, col: usize) -> usize {
        let line = line.min(self.total_lines.saturating_sub(1));
        let start = self.line_start(line).unwrap_or(0);
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
        let mut ci = 0;
        for (bi, _) in sub.char_indices() {
            if ci == char_offset {
                byte_offset = bi;
                break;
            }
            ci += 1;
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
