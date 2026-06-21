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
    /// After an edit, records the (first_changed_line, last_changed_line_exclusive)
    /// so incremental sync can avoid rebuilding the entire file list.
    last_edit_line_range: Option<(usize, usize)>,
    /// Monotonically increasing buffer version. Incremented on every edit.
    /// Used by the background parse pipeline to detect stale results.
    pub buffer_version: u64,
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
            last_edit_line_range: None,
            buffer_version: 0,
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
            last_edit_line_range: None,
            buffer_version: 0,
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

    /// Return joined text of lines `[start..end)` from the rope via O(1) index.
    pub fn visible_lines(&self, start: usize, end: usize) -> String {
        self.rope.visible_lines(start, end)
    }

    /// Return up to `max_chars` characters from the start of the buffer, verbatim
    /// (original `\r`/`\n` bytes preserved). Used for cheap indentation and
    /// line-ending detection without materializing the whole document.
    ///
    /// Uses a single O(max_chars) piece-table walk via `extract_chars`. (The
    /// previous `char_at(i)` loop was O(n²) — each `char_at` re-walks from the
    /// start — and dominated `app_update` every frame: ~520ms on large files.)
    pub fn raw_head(&self, max_chars: usize) -> String {
        let n = self.rope.char_count().min(max_chars);
        self.rope.extract_chars(0, n)
    }

    /// Return the total document line count (from rope, always correct).
    pub fn total_lines(&self) -> usize {
        self.rope.line_count()
    }

    /// Return all lines.
    pub fn lines(&self) -> Vec<String> {
        self.rope.lines().collect()
    }

    /// Get an immutable reference to the rope.
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    /// After an edit, returns the range of logical lines that were affected
    /// (first_changed_line, last_changed_line_exclusive). None means
    /// no edit was tracked or a full rebuild is needed.
    pub fn last_edit_line_range(&self) -> Option<(usize, usize)> {
        self.last_edit_line_range
    }

    /// Clear the tracked edit range after it has been consumed by sync logic.
    pub fn clear_edit_line_range(&mut self) {
        self.last_edit_line_range = None;
    }

    // ── Tab expansion ──

    pub const TAB_WIDTH: usize = 4;

    /// Convert a line from raw text (with `\t`) to display text (tabs → spaces).
    pub fn expand_tabs(raw: &str, tab_width: usize) -> String {
        let mut out = String::with_capacity(raw.len());
        for c in raw.chars() {
            if c == '\t' {
                let spaces = tab_width - (out.len() % tab_width);
                for _ in 0..spaces {
                    out.push(' ');
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Map a visual column (tab-expanded) to a raw character index within `raw`.
    pub fn vis_to_raw_col(raw: &str, vis_col: usize, tab_width: usize) -> usize {
        if vis_col == 0 {
            return 0;
        }
        let mut vis = 0usize;
        for (raw_idx, c) in raw.char_indices() {
            if c == '\t' {
                let spaces = tab_width - (vis % tab_width);
                for _ in 0..spaces {
                    if vis >= vis_col {
                        return raw_idx;
                    }
                    vis += 1;
                }
            } else {
                if vis >= vis_col {
                    return raw_idx;
                }
                vis += 1;
            }
        }
        raw.chars().count() // beyond end → clamp to end
    }

    /// Map a raw character index within `raw` to a visual column (tab-expanded).
    pub fn raw_to_vis_col(raw: &str, raw_idx: usize, tab_width: usize) -> usize {
        let mut vis = 0usize;
        for (ri, c) in raw.char_indices() {
            if ri >= raw_idx {
                break;
            }
            if c == '\t' {
                let spaces = tab_width - (vis % tab_width);
                vis += spaces;
            } else {
                vis += 1;
            }
        }
        vis
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

    /// Current caret column as raw character index (0-based).
    /// For rendering, use `caret_vis_col()` which accounts for tab expansion.
    pub fn caret_col(&self) -> usize {
        let (_, col) = self.rope.char_index_to_line_col(self.caret());
        col
    }

    /// Current caret visual column, with tabs expanded to spaces.
    /// This matches what the renderer displays after tab expansion.
    pub fn caret_vis_col(&self) -> usize {
        let raw_col = self.caret_col();
        let line_str = self.rope.line(self.caret_line()).unwrap_or_default();
        Self::raw_to_vis_col(&line_str, raw_col, Self::TAB_WIDTH)
    }

    /// Return all lines with tabs expanded for display/rendering.
    pub fn lines_expanded(&self) -> Vec<String> {
        self.rope.lines().map(|line| Self::expand_tabs(&line, Self::TAB_WIDTH)).collect()
    }

    /// Set the caret to a specific character index. Clears selection.
    pub fn set_caret(&mut self, char_index: usize) {
        self.caret = char_index.min(self.rope.char_count());
        self.selection_anchor = None;
        self.selection_active = false;
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
        self.last_edit_line_range = None;
    }

    /// Set caret from (line, raw character column). Updates preferred_column.
    pub fn set_caret_line_col(&mut self, line: usize, col: usize) {
        self.caret = self.rope.line_col_to_char_index(line, col);
        self.selection_anchor = None;
        self.selection_active = false;
        let (_, actual_col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = actual_col;
        self.last_edit_line_range = None;
    }

    /// Set caret from a mouse hit-test visual column.
    /// Converts visual column → raw char index accounting for tab stops.
    pub fn set_caret_line_vis_col(&mut self, line: usize, vis_col: usize) {
        let line_str = self.rope.line(line).unwrap_or_default();
        let raw_col = Self::vis_to_raw_col(&line_str, vis_col, Self::TAB_WIDTH);
        self.set_caret_line_col(line, raw_col);
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
        self.buffer_version += 1;
        let mut selection_range: Option<(usize, usize)> = None;
        // Replace selection if present
        if let Some((start, end)) = self.sorted_selection() {
            let (sl, _) = self.rope.char_index_to_line_col(start);
            let (el, _) = self.rope.char_index_to_line_col(end);
            selection_range = Some((sl, el + 1));
            self.rope.delete(start, end);
            self.caret = start;
            self.selection_anchor = None;
            self.selection_active = false;
        }

        let insert_pos = self.caret;
        let (insert_line, _) = self.rope.char_index_to_line_col(insert_pos);
        self.rope.insert(insert_pos, text);
        let text_len = text.chars().count();
        let new_caret = insert_pos + text_len;
        self.caret = new_caret;
        let (new_line, col) = self.rope.char_index_to_line_col(new_caret);
        self.preferred_column = col;
        self.selection_anchor = None;

        let insert_end = if text.contains('\n') { new_line + 1 } else { insert_line + 1 };

        // Union of selection range and insert range
        self.last_edit_line_range = match selection_range {
            Some((sl, el)) => Some((sl.min(insert_line), el.max(insert_end))),
            None => Some((insert_line, insert_end)),
        };

        Some((insert_pos, text.to_string()))
    }

    /// Insert a newline at the current caret (replace selection first).
    pub fn insert_newline(&mut self) -> Option<(usize, String)> {
        self.insert_text("\n")
    }

    /// Delete one character before the caret, or the current selection.
    /// Returns the delete range and removed text.
    pub fn backspace(&mut self) -> Option<(usize, usize)> {
        self.buffer_version += 1;
        if let Some((start, end)) = self.sorted_selection() {
            let (sl, _) = self.rope.char_index_to_line_col(start);
            let (el, _) = self.rope.char_index_to_line_col(end);
            self.rope.delete(start, end);
            self.caret = start;
            self.selection_anchor = None;
            self.selection_active = false;
            self.preferred_column = self.rope.char_index_to_line_col(start).1;
            self.last_edit_line_range = Some((sl, el + 1));
            return Some((start, end));
        }

        if self.caret == 0 {
            return None;
        }
        let start = self.caret - 1;
        let end = self.caret;
        let (line, _) = self.rope.char_index_to_line_col(start);
        let is_line_start = start == self.rope.line_start(line).unwrap_or(start);
        let affected_end = if is_line_start && line > 0 { line + 1 } else { line + 1 };
        let affected_start = if is_line_start && line > 0 { line - 1 } else { line };
        self.rope.delete(start, end);
        self.caret = start;
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
        self.last_edit_line_range = Some((affected_start, affected_end));
        Some((start, end))
    }

    /// Delete one character after the caret, or the current selection.
    /// Returns the delete range.
    pub fn delete_forward(&mut self) -> Option<(usize, usize)> {
        self.buffer_version += 1;
        if let Some((start, end)) = self.sorted_selection() {
            let (sl, _) = self.rope.char_index_to_line_col(start);
            let (el, _) = self.rope.char_index_to_line_col(end);
            self.rope.delete(start, end);
            self.caret = start;
            self.selection_anchor = None;
            self.selection_active = false;
            self.preferred_column = self.rope.char_index_to_line_col(start).1;
            self.last_edit_line_range = Some((sl, el + 1));
            return Some((start, end));
        }

        if self.caret >= self.rope.char_count() {
            return None;
        }
        let start = self.caret;
        let end = self.caret + 1;
        let (line, _) = self.rope.char_index_to_line_col(start);
        let is_line_end = end >= self.rope.line_start(line + 1).unwrap_or(self.rope.char_count());
        let affected_start = line;
        let affected_end =
            if is_line_end && line + 1 < self.rope.line_count() { line + 2 } else { line + 1 };
        self.rope.delete(start, end);
        self.preferred_column = self.rope.char_index_to_line_col(self.caret).1;
        self.last_edit_line_range = Some((affected_start, affected_end));
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
        self.buffer_version += 1;
        let old_line = self.caret_line();
        let old_col = self.caret_col();
        self.rope = Rope::new(text);
        self.selection_anchor = None;
        self.selection_active = false;
        // Try to preserve cursor position
        let new_caret = self.rope.line_col_to_char_index(old_line, old_col);
        self.caret = new_caret;
        self.preferred_column = old_col;
        self.last_edit_line_range = None; // full rebuild needed
    }

    /// Populate the buffer from ContentView data (when a file is opened).
    /// Sets the cursor to the ContentView cursor position if available.
    pub fn populate_from_lines(&mut self, lines: &[String], cursor_line: usize, cursor_col: usize) {
        self.buffer_version += 1;
        // Build the rope in a single fused pass (join + index) — no intermediate
        // joined String and no second scan. Roughly halves open cost on huge files.
        self.rope = Rope::from_lines(lines);
        self.caret = self.rope.line_col_to_char_index(cursor_line, cursor_col);
        let (_, col) = self.rope.char_index_to_line_col(self.caret);
        self.preferred_column = col;
        self.selection_anchor = None;
        self.selection_active = false;
        self.last_edit_line_range = None; // full replacement
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
    fn raw_head_returns_verbatim_capped_head() {
        let buf = EditorBufferState::from_text("abc\ndef\nghi");
        // Full content when the cap exceeds length.
        assert_eq!(buf.raw_head(1000), "abc\ndef\nghi");
        // Capped to the first N chars.
        assert_eq!(buf.raw_head(3), "abc");
        // Newlines are preserved verbatim.
        assert_eq!(buf.raw_head(4), "abc\n");
        // Empty buffer yields empty head regardless of cap.
        assert_eq!(EditorBufferState::empty().raw_head(10), "");
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

    #[test]
    fn tab_expansion_replaces_tabs() {
        let expanded = EditorBufferState::expand_tabs("a\tb", 4);
        assert_eq!(expanded, "a   b");
    }

    #[test]
    fn tab_expansion_at_start_of_line() {
        let expanded = EditorBufferState::expand_tabs("\tx", 4);
        assert_eq!(expanded, "    x");
    }

    #[test]
    fn tab_expansion_at_tab_stop() {
        let expanded = EditorBufferState::expand_tabs("ab\tc", 4);
        assert_eq!(expanded, "ab  c");
    }

    #[test]
    fn vis_to_raw_col_tab_boundaries() {
        // "a\tb" → raw: a(0), \t(1), b(2)
        // visual cols: a at 0, tab fills 1..4 (3 spaces), b at 4
        assert_eq!(EditorBufferState::vis_to_raw_col("a\tb", 0, 4), 0); // 'a'
        assert_eq!(EditorBufferState::vis_to_raw_col("a\tb", 1, 4), 1); // start of tab visual space
        assert_eq!(EditorBufferState::vis_to_raw_col("a\tb", 3, 4), 1); // inside tab → tab char
        assert_eq!(EditorBufferState::vis_to_raw_col("a\tb", 4, 4), 2); // 'b'
        assert_eq!(EditorBufferState::vis_to_raw_col("a\tb", 5, 4), 3); // beyond 'b' → end of raw line
    }

    #[test]
    fn raw_to_vis_col_tab_expansion() {
        // "a\tb" → raw: a(0), \t(1), b(2)
        // visual: a at 0, tab pushes to next stop at 4, b at 4
        assert_eq!(EditorBufferState::raw_to_vis_col("a\tb", 0, 4), 0);
        assert_eq!(EditorBufferState::raw_to_vis_col("a\tb", 1, 4), 1);
        assert_eq!(EditorBufferState::raw_to_vis_col("a\tb", 2, 4), 4);
    }

    #[test]
    fn caret_vis_col_with_tabs() {
        let mut buf = EditorBufferState::from_text("a\tb");
        buf.set_caret(1); // raw caret at the tab character
        assert_eq!(buf.caret_col(), 1);
        assert_eq!(buf.caret_vis_col(), 1); // visual column at start of tab
        buf.set_caret(2); // raw caret at 'b'
        assert_eq!(buf.caret_col(), 2);
        assert_eq!(buf.caret_vis_col(), 4); // visual column: a(0) + tab→4, b at 4
    }

    #[test]
    fn lines_expanded_strips_tabs() {
        let buf = EditorBufferState::from_text("a\tb\n\tc");
        let expanded = buf.lines_expanded();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0], "a   b");
        assert_eq!(expanded[1], "    c");
    }

    #[test]
    fn set_caret_line_vis_col_resolves_to_tab() {
        let mut buf = EditorBufferState::from_text("a\tb");
        buf.set_caret_line_vis_col(0, 2); // click in middle of tab visual space
        assert_eq!(buf.caret(), 1); // raw index → tab character
        assert_eq!(buf.caret_col(), 1); // raw column at tab
    }
}
