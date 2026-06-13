/// Canonical rope-backed editor buffer state.
///
/// This module provides the single source of truth for editor text,
/// caret position, and selection. It uses `zaroxi_core_editor_rope::Rope`
/// for efficient character-indexed insert/delete operations.
///
/// All positions in this module are **character indices** (not byte offsets),
/// matching the workspace service's `TextEdit` semantics.
use zaroxi_core_editor_rope::Rope;

/// The canonical caret/selection model for the editor.
#[derive(Clone, Debug)]
pub struct EditorBufferState {
    /// Rope-backed text buffer.
    rope: Rope,
    /// Caret position as a character index into the rope.
    caret: usize,
    /// Optional selection anchor (character index). When present,
    /// the range [caret, anchor] (or [anchor, caret]) is selected.
    selection_anchor: Option<usize>,
    /// Preferred column for vertical caret movement (up/down).
    preferred_column: usize,
    /// Whether selection is currently active (drag in progress).
    pub selection_active: bool,
}

impl EditorBufferState {
    /// Create an empty editor buffer.
    pub fn empty() -> Self {
        Self {
            rope: Rope::empty(),
            caret: 0,
            selection_anchor: None,
            preferred_column: 0,
            selection_active: false,
        }
    }

    /// Create a buffer from an existing string.
    pub fn from_text(text: &str) -> Self {
        let rope = Rope::new(text);
        Self {
            rope,
            caret: 0,
            selection_anchor: None,
            preferred_column: 0,
            selection_active: false,
        }
    }

    /// Return the total character count.
    pub fn char_count(&self) -> usize {
        self.rope.char_count()
    }

    /// Return the line count (at least 1 for an empty buffer).
    pub fn line_count(&self) -> usize {
        self.rope.line_count()
    }

    /// Return the full text content as a String.
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Return a Vec of all lines.
    pub fn lines(&self) -> Vec<String> {
        self.rope.lines().collect()
    }

    /// Get an immutable reference to the rope.
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    // ── Caret / line-col access ──

    /// Currently active caret char index.
    pub fn caret(&self) -> usize {
        self.caret.min(self.rope.char_count())
    }

    /// Current caret line (0-based).
    pub fn caret_line(&self) -> usize {
        let (line, _) = self.rope.char_index_to_line_col(self.caret());
        line
    }

    /// Current caret column (0-based).
    pub fn caret_col(&self) -> usize {
        let (_, col) = self.rope.char_index_to_line_col(self.caret());
        col
    }

    /// Set the caret to a specific character index. Clears selection.
    pub fn set_caret(&mut self, char_index: usize) {
        self.caret = char_index.min(self.rope.char_count());
        self.selection_anchor = None;
        self.selection_active = false;
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
    }

    /// Set caret from line/column. Updates preferred_column.
    pub fn set_caret_line_col(&mut self, line: usize, col: usize) {
        self.caret = self.rope.line_col_to_char_index(line, col);
        self.selection_anchor = None;
        self.selection_active = false;
        let (_, actual_col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = actual_col;
    }

    /// Preferred column for vertical movement.
    pub fn preferred_column(&self) -> usize {
        self.preferred_column
    }

    // ── Selection ──

    /// Start a selection at the current caret.
    pub fn begin_selection(&mut self) {
        self.selection_anchor = Some(self.caret);
        self.selection_active = true;
    }

    /// Extend selection to the given char index.
    pub fn extend_selection_to(&mut self, char_index: usize) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.caret);
        }
        self.caret = char_index.min(self.rope.char_count());
        self.selection_active = true;
    }

    /// End selection (keep caret at current position).
    pub fn end_selection(&mut self) {
        self.selection_active = false;
    }

    /// Clear selection entirely.
    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
        self.selection_active = false;
    }

    /// Return selection range as (start_line, start_col, end_line, end_col) if active.
    pub fn selection_range(&self) -> Option<(usize, usize, usize, usize)> {
        let anchor = self.selection_anchor?;
        let cursor = self.caret;
        if anchor == cursor {
            return None;
        }
        let (start, end) = if anchor < cursor { (anchor, cursor) } else { (cursor, anchor) };
        let (sl, sc) = self.rope.char_index_to_line_col(start);
        let (el, ec) = self.rope.char_index_to_line_col(end);
        Some((sl, sc, el, ec))
    }

    /// Return the selected text, if any.
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.sorted_selection()?;
        if start == end {
            return None;
        }
        let mut text = String::new();
        for ci in start..end {
            if let Some(c) = self.rope.char_at(ci) {
                text.push(c);
            }
        }
        Some(text)
    }

    /// Get the selection start..end as character indices (start < end).
    fn sorted_selection(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        let cursor = self.caret;
        if anchor == cursor {
            return None;
        }
        Some(if anchor < cursor { (anchor, cursor) } else { (cursor, anchor) })
    }

    /// Return the delete-range for the current selection, or None.
    pub fn selection_delete_range(&self) -> Option<(usize, usize)> {
        self.sorted_selection()
    }

    // ── Editing operations ──

    /// Insert text at the current caret, replacing any selection first.
    /// Returns (start_index, inserted_text) for workspace transaction.
    pub fn insert_text(&mut self, text: &str) -> Option<(usize, String)> {
        // Replace selection if present
        if let Some((start, end)) = self.sorted_selection() {
            self.rope.delete(start, end);
            self.caret = start;
            self.selection_anchor = None;
            self.selection_active = false;
        }

        let insert_pos = self.caret;
        self.rope.insert(insert_pos, text);
        let text_len = text.chars().count();
        self.caret = insert_pos + text_len;
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
        self.selection_anchor = None;

        Some((insert_pos, text.to_string()))
    }

    /// Insert a newline at the current caret (replace selection first).
    pub fn insert_newline(&mut self) -> Option<(usize, String)> {
        self.insert_text("\n")
    }

    /// Delete one character before the caret, or the current selection.
    /// Returns the delete range and removed text.
    pub fn backspace(&mut self) -> Option<(usize, usize)> {
        if let Some((start, end)) = self.sorted_selection() {
            self.rope.delete(start, end);
            self.caret = start;
            self.selection_anchor = None;
            self.selection_active = false;
            self.preferred_column = self.rope.char_index_to_line_col(start).1;
            return Some((start, end));
        }

        if self.caret == 0 {
            return None;
        }
        let start = self.caret - 1;
        let end = self.caret;
        self.rope.delete(start, end);
        self.caret = start;
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
        Some((start, end))
    }

    /// Delete one character after the caret, or the current selection.
    /// Returns the delete range.
    pub fn delete_forward(&mut self) -> Option<(usize, usize)> {
        if let Some((start, end)) = self.sorted_selection() {
            self.rope.delete(start, end);
            self.caret = start;
            self.selection_anchor = None;
            self.selection_active = false;
            self.preferred_column = self.rope.char_index_to_line_col(start).1;
            return Some((start, end));
        }

        if self.caret >= self.rope.char_count() {
            return None;
        }
        let start = self.caret;
        let end = self.caret + 1;
        self.rope.delete(start, end);
        self.preferred_column = self.rope.char_index_to_line_col(self.caret).1;
        Some((start, end))
    }

    // ── Cursor movement ──

    /// Move caret left by one character.
    pub fn move_left(&mut self) {
        if self.caret > 0 {
            self.caret -= 1;
        }
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
    }

    /// Move caret right by one character.
    pub fn move_right(&mut self) {
        if self.caret < self.rope.char_count() {
            self.caret += 1;
        }
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
    }

    /// Move caret up by one visual line.
    pub fn move_up(&mut self) {
        let (line, _col) = self.rope.char_index_to_line_col(self.caret);
        if line > 0 {
            let target_line = line.saturating_sub(1);
            self.caret = self.rope.line_col_to_char_index(target_line, self.preferred_column);
        } else {
            self.caret = 0;
        }
    }

    /// Move caret down by one visual line.
    pub fn move_down(&mut self) {
        let (line, _col) = self.rope.char_index_to_line_col(self.caret);
        if line + 1 < self.rope.line_count() {
            let target_line = line + 1;
            self.caret = self.rope.line_col_to_char_index(target_line, self.preferred_column);
        } else {
            // Move to end of document
            self.caret = self.rope.char_count();
        }
    }

    /// Move caret to the start of the current line.
    pub fn move_home(&mut self) {
        let (line, _) = self.rope.char_index_to_line_col(self.caret);
        self.caret = self.rope.line_start(line).unwrap_or(0);
        self.preferred_column = 0;
    }

    /// Move caret to the end of the current line.
    pub fn move_end(&mut self) {
        let (line, _) = self.rope.char_index_to_line_col(self.caret);
        let line_len = self.rope.line_length(line);
        let start = self.rope.line_start(line).unwrap_or(0);
        self.caret = start + line_len;
        self.preferred_column = line_len;
    }

    // ── Content sync ──

    /// Replace the entire buffer content with new text.
    /// Preserves cursor position when possible, clamps otherwise.
    pub fn replace_content(&mut self, text: &str) {
        let old_line = self.caret_line();
        let old_col = self.caret_col();
        self.rope = Rope::new(text);
        self.selection_anchor = None;
        self.selection_active = false;
        // Try to preserve cursor position
        let new_caret = self.rope.line_col_to_char_index(old_line, old_col);
        self.caret = new_caret;
        self.preferred_column = old_col;
    }

    /// Populate the buffer from ContentView data (when a file is opened).
    /// Sets the cursor to the ContentView cursor position if available.
    pub fn populate_from_lines(&mut self, lines: &[String], cursor_line: usize, cursor_col: usize) {
        let text = lines.join("\n");
        self.rope = Rope::new(&text);
        self.caret = self.rope.line_col_to_char_index(cursor_line, cursor_col);
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
        self.selection_anchor = None;
        self.selection_active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer() {
        let buf = EditorBufferState::empty();
        assert_eq!(buf.char_count(), 0);
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.caret(), 0);
    }

    #[test]
    fn from_text_sets_caret_at_zero() {
        let buf = EditorBufferState::from_text("hello");
        assert_eq!(buf.char_count(), 5);
        assert_eq!(buf.caret(), 0);
        assert_eq!(buf.to_string(), "hello");
    }

    #[test]
    fn insert_text_moves_caret() {
        let mut buf = EditorBufferState::from_text("hello");
        buf.set_caret(5); // move to end
        buf.insert_text(" world");
        assert_eq!(buf.to_string(), "hello world");
        assert_eq!(buf.caret(), 11);
    }

    #[test]
    fn backspace_removes_char() {
        let mut buf = EditorBufferState::from_text("hello");
        buf.set_caret(5); // move to end
        buf.backspace();
        assert_eq!(buf.to_string(), "hell");
        assert_eq!(buf.caret(), 4);
    }

    #[test]
    fn delete_forward_removes_char() {
        let mut buf = EditorBufferState::from_text("hello");
        buf.delete_forward();
        assert_eq!(buf.to_string(), "ello");
        assert_eq!(buf.caret(), 0);
    }

    #[test]
    fn insert_newline_splits_text() {
        let mut buf = EditorBufferState::from_text("ab");
        buf.set_caret(1);
        buf.insert_newline();
        assert_eq!(buf.to_string(), "a\nb");
        assert_eq!(buf.line_count(), 2);
    }

    #[test]
    fn move_left_right() {
        let mut buf = EditorBufferState::from_text("abc");
        buf.set_caret(1);
        buf.move_right();
        assert_eq!(buf.caret(), 2);
        buf.move_left();
        assert_eq!(buf.caret(), 1);
        buf.move_left();
        assert_eq!(buf.caret(), 0);
        buf.move_left(); // clamped
        assert_eq!(buf.caret(), 0);
    }

    #[test]
    fn move_up_down() {
        let mut buf = EditorBufferState::from_text("line1\nline2\nline3");
        buf.set_caret_line_col(2, 3); // line 2, col 3
        buf.move_up();
        assert_eq!(buf.caret_line(), 1);
        assert_eq!(buf.caret_col(), 3); // preserved column
        buf.move_down();
        assert_eq!(buf.caret_line(), 2);
        buf.move_down(); // last line
        assert_eq!(buf.caret_line(), 2);
    }

    #[test]
    fn move_home_end() {
        let mut buf = EditorBufferState::from_text("hello world");
        buf.set_caret(6); // at 'w'
        buf.move_home();
        assert_eq!(buf.caret(), 0);
        buf.move_end();
        assert_eq!(buf.caret(), 11);
    }

    #[test]
    fn selection_range() {
        let mut buf = EditorBufferState::from_text("hello world");
        buf.set_caret(6);
        buf.begin_selection();
        buf.extend_selection_to(11); // select "world"
        let range = buf.selection_range().unwrap();
        assert_eq!(range, (0, 6, 0, 11));
    }

    #[test]
    fn insert_replaces_selection() {
        let mut buf = EditorBufferState::from_text("hello world");
        buf.set_caret(6);
        buf.begin_selection();
        buf.extend_selection_to(11); // select "world"
        buf.insert_text("rust"); // replace with "rust"
        assert_eq!(buf.to_string(), "hello rust");
        assert_eq!(buf.caret(), 10);
    }

    #[test]
    fn backspace_deletes_selection() {
        let mut buf = EditorBufferState::from_text("hello world");
        buf.set_caret(6);
        buf.begin_selection();
        buf.extend_selection_to(11);
        buf.backspace();
        assert_eq!(buf.to_string(), "hello ");
        assert_eq!(buf.caret(), 6);
    }

    #[test]
    fn delete_forward_deletes_selection() {
        let mut buf = EditorBufferState::from_text("hello world");
        buf.set_caret(6);
        buf.begin_selection();
        buf.extend_selection_to(11);
        buf.delete_forward();
        assert_eq!(buf.to_string(), "hello ");
        assert_eq!(buf.caret(), 6);
    }

    #[test]
    fn caret_line_col() {
        let buf = EditorBufferState::from_text("ab\ncd\nef");
        let mut b = buf;
        b.set_caret_line_col(1, 1);
        assert_eq!(b.caret_line(), 1);
        assert_eq!(b.caret_col(), 1);
        assert_eq!(b.caret(), 4); // "ab\n" = 3 chars, then "c" = 4th char
    }

    #[test]
    fn utf8_editing() {
        let mut buf = EditorBufferState::from_text("héllo");
        assert_eq!(buf.char_count(), 5, "from_text char_count");
        buf.set_caret(1);
        assert_eq!(buf.caret(), 1, "after set_caret");
        buf.insert_text("i");
        assert_eq!(buf.char_count(), 6, "char_count after insert");
        assert_eq!(buf.to_string().as_bytes(), "hiéllo".as_bytes(), "content after insert");
        buf.move_right();
        buf.backspace();
        assert_eq!(
            buf.to_string().as_bytes(),
            "hillo".as_bytes(),
            "after move_right+backspace deletes é"
        );
        assert_eq!(buf.caret(), 2);
    }

    #[test]
    fn lines_from_buffer() {
        let buf = EditorBufferState::from_text("a\nb\nc");
        let lines = buf.lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "a");
        assert_eq!(lines[1], "b");
        assert_eq!(lines[2], "c");
    }
}
