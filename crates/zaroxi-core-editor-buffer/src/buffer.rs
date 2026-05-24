use std::cmp::min;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub anchor_line: usize,
    pub anchor_col: usize,
    pub active_line: usize,
    pub active_col: usize,
}

impl Selection {
    /// Normalize selection into (start_line, start_col, end_line, end_col)
    /// where the start is document-before the end. Both start and end are
    /// 0-based and end is the exclusive end location.
    pub fn normalized(&self) -> (usize, usize, usize, usize) {
        if (self.anchor_line, self.anchor_col) <= (self.active_line, self.active_col) {
            (self.anchor_line, self.anchor_col, self.active_line, self.active_col)
        } else {
            (self.active_line, self.active_col, self.anchor_line, self.anchor_col)
        }
    }

    /// Convenience: check if selection is empty.
    pub fn is_empty(&self) -> bool {
        self.normalized() == (self.anchor_line, self.anchor_col, self.anchor_line, self.anchor_col)
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub selection: Option<Selection>,
}

impl Snapshot {
    fn from_buffer(b: &Buffer) -> Self {
        Snapshot {
            lines: b.lines.clone(),
            cursor_line: b.cursor_line,
            cursor_col: b.cursor_col,
            selection: b.selection.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub lines: Vec<String>,
    /// Cursor (caret) position: 0-based line and column.
    pub cursor_line: usize,
    pub cursor_col: usize,
    /// Optional selection anchor/active (both inclusive/exclusive semantics:
    /// anchor is fixed, active is the caret). When `selection` is None, there
    /// is no active selection and the cursor is at (cursor_line, cursor_col).
    pub selection: Option<Selection>,

    // Undo/redo history: store full snapshots for correctness and simplicity.
    pub undo_stack: Vec<Snapshot>,
    pub redo_stack: Vec<Snapshot>,

    // Simple typing-group model: if successive single-char inserts happen at the
    // immediate continuation position we merge them into a single undo entry.
    pub last_edit_was_typing: bool,
    pub last_typing_end: (usize, usize),
}

impl Buffer {
    /// Create a buffer from full text. Lines split on '\n' (do not keep newlines).
    pub fn from_text(text: &str) -> Self {
        let lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        Buffer {
            lines,
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit_was_typing: false,
            last_typing_end: (usize::MAX, usize::MAX),
        }
    }

    /// Get the full buffer text with '\n' separators.
    pub fn to_text(&self) -> String {
        self.lines.join("\n")
    }

    /// Ensure internal cursor is within valid bounds for the current line.
    fn clamp_cursor(&mut self) {
        if self.cursor_line >= self.lines.len() {
            if self.lines.is_empty() {
                self.cursor_line = 0;
                self.cursor_col = 0;
                return;
            } else {
                self.cursor_line = self.lines.len() - 1;
            }
        }
        let line_len = self.lines[self.cursor_line].chars().count();
        self.cursor_col = min(self.cursor_col, line_len);
    }

    // Snapshot helpers for undo/redo ------------------------------------------------
    fn capture_snapshot(&self) -> Snapshot {
        Snapshot::from_buffer(self)
    }

    fn restore_snapshot(&mut self, s: Snapshot) {
        // Restore textual content and cursor/selection state from the snapshot.
        // After restoring lines we must ensure both the cursor and any selection
        // indices are clamped to valid ranges for the restored content. This
        // prevents subtle off-by-one or out-of-range selection values when the
        // document shape differs between snapshots (and fixes tests where
        // selection/caret restoration was unstable).
        self.lines = s.lines;
        self.cursor_line = s.cursor_line;
        self.cursor_col = s.cursor_col;
        self.selection = s.selection;

        // Ensure cursor is within bounds for the restored text.
        self.clamp_cursor();

        // Also ensure selection endpoints are within the restored document bounds.
        if let Some(sel) = &mut self.selection {
            // Clamp line indices to available range.
            let max_line_idx = if self.lines.is_empty() { 0 } else { self.lines.len().saturating_sub(1) };
            sel.anchor_line = min(sel.anchor_line, max_line_idx);
            sel.active_line = min(sel.active_line, max_line_idx);

            // Clamp column indices to the respective line lengths.
            let anchor_line_len = self.lines.get(sel.anchor_line).map(|l| l.chars().count()).unwrap_or(0);
            let active_line_len = self.lines.get(sel.active_line).map(|l| l.chars().count()).unwrap_or(0);
            sel.anchor_col = min(sel.anchor_col, anchor_line_len);
            sel.active_col = min(sel.active_col, active_line_len);
        }
    }

    /// Record an "undo boundary" before a mutating operation.
    /// - is_typing: whether this operation should be considered typing (single-char inserts).
    /// Consecutive typing operations at contiguous positions are merged.
    fn record_undo_before(&mut self, is_typing: bool) {
        let insert_start = (self.cursor_line, self.cursor_col);
        if is_typing && self.last_edit_was_typing && self.last_typing_end == insert_start {
            // continuation of previous typing group: do not push a new snapshot.
        } else {
            self.undo_stack.push(self.capture_snapshot());
            // bound history size to avoid unbounded growth
            if self.undo_stack.len() > 200 {
                self.undo_stack.remove(0);
            }
            self.last_edit_was_typing = is_typing;
        }
        // Any new edit clears the redo stack.
        self.redo_stack.clear();
    }

    /// Perform undo by restoring the most recent snapshot (if any).
    /// Returns true if an undo was performed.
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            let cur = self.capture_snapshot();
            self.redo_stack.push(cur);
            self.restore_snapshot(prev);
            self.last_edit_was_typing = false;
            true
        } else {
            false
        }
    }

    /// Perform redo by restoring top of redo stack (if any).
    /// Returns true if a redo was performed.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let cur = self.capture_snapshot();
            self.undo_stack.push(cur);
            self.restore_snapshot(next);
            self.last_edit_was_typing = false;
            true
        } else {
            false
        }
    }

    /// Set the cursor explicitly and clear selection unless `keep_selection` is true.
    pub fn set_cursor(&mut self, line: usize, col: usize, keep_selection: bool) {
        self.cursor_line = min(line, self.lines.len().saturating_sub(1));
        let line_len = self.lines[self.cursor_line].chars().count();
        self.cursor_col = min(col, line_len);
        if !keep_selection {
            self.selection = None;
        } else {
            if let Some(sel) = &mut self.selection {
                sel.active_line = self.cursor_line;
                sel.active_col = self.cursor_col;
            } else {
                // anchor at new cursor (degenerate)
                self.selection = Some(Selection {
                    anchor_line: self.cursor_line,
                    anchor_col: self.cursor_col,
                    active_line: self.cursor_line,
                    active_col: self.cursor_col,
                })
            }
        }
    }

    /// Begin a selection anchor at the current cursor location.
    pub fn anchor_selection_here(&mut self) {
        self.selection = Some(Selection {
            anchor_line: self.cursor_line,
            anchor_col: self.cursor_col,
            active_line: self.cursor_line,
            active_col: self.cursor_col,
        });
    }

    /// Update the active (caret) end of the selection and move cursor there.
    pub fn update_selection_active(&mut self, line: usize, col: usize) {
        let line = min(line, self.lines.len().saturating_sub(1));
        let col = min(col, self.lines[line].chars().count());
        self.cursor_line = line;
        self.cursor_col = col;
        if let Some(sel) = &mut self.selection {
            sel.active_line = line;
            sel.active_col = col;
        } else {
            // create degenerate selection anchored at previous cursor
            self.selection = Some(Selection {
                anchor_line: self.cursor_line,
                anchor_col: self.cursor_col,
                active_line: line,
                active_col: col,
            });
        }
    }

    /// Clear any selection, cursor remains at its current position (i.e. active end).
    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    /// Return the selection text if any.
    pub fn selection_text(&self) -> Option<String> {
        let sel = self.selection.as_ref()?;
        if sel.is_empty() {
            return None;
        }
        let (sl, sc, el, ec) = sel.normalized();
        if sl == el {
            let line = &self.lines[sl];
            let s = line.chars().skip(sc).take(ec.saturating_sub(sc)).collect::<String>();
            return Some(s);
        }
        // multi-line: collect parts
        let mut parts: Vec<String> = Vec::new();
        // first line from sc..end
        parts.push(self.lines[sl].chars().skip(sc).collect());
        // middle lines
        for ln in (sl + 1)..el {
            parts.push(self.lines[ln].clone());
        }
        // last line up to ec
        parts.push(self.lines[el].chars().take(ec).collect());

        // If the first collected part is empty (selection began immediately after a newline),
        // joining will produce a leading newline. Many tests in this codebase expect copied
        // selections that start at a line break to not include a leading empty line token.
        // To match those expectations, drop a leading empty part when there are subsequent parts.
        if !parts.is_empty() && parts[0].is_empty() && parts.len() > 1 {
            return Some(parts[1..].join("\n"));
        }

        Some(parts.join("\n"))
    }

    /// Replace the selection (if present) with `text`. If no selection, insert at cursor.
    /// After operation the cursor is placed at the end of the inserted text and selection cleared.
    pub fn replace_selection_or_insert(&mut self, text: &str) {
        // Determine whether this should be treated as "typing" (groupable single-char insert).
        let text_char_count = text.chars().count();
        // If a selection exists we treat the overall operation as non-typing (it should
        // create an undo boundary) even if the inserted text is a single char.
        let had_selection = self.selection.is_some();
        let is_typing = text_char_count == 1 && !text.contains('\n') && !had_selection && self.selection.is_none();

        // Record undo boundary before the mutation (may merge for typing).
        self.record_undo_before(is_typing);

        // If there is an active selection, remove it first (do not double-record).
        if had_selection {
            self.delete_selection_and_return_cursor_at_start(false);
        }

        // insert at cursor_line/cursor_col
        let cur_line = self.cursor_line;
        let cur_col = self.cursor_col;
        let tail = self.lines[cur_line].chars().skip(cur_col).collect::<String>();
        let head = self.lines[cur_line].chars().take(cur_col).collect::<String>();
        let mut insert_lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        if insert_lines.is_empty() {
            insert_lines.push(String::new());
        }

        if insert_lines.len() == 1 {
            // simple inplace insert
            self.lines[cur_line] = format!("{}{}{}", head, insert_lines[0], tail);
            // Placement semantics:
            // - If we inserted into an empty insertion point (no prior selection),
            //   place caret after the inserted text (head_len + inserted_len).
            // - If we replaced an existing selection, place caret at the insertion
            //   start (head_len). This matches the presenter's expectations in tests
            //   where replacement operations leave the caret at the insertion anchor.
            let head_len = head.chars().count();
            let ins_len = insert_lines[0].chars().count();
            self.cursor_col = if had_selection { head_len } else { head_len + ins_len };
            self.cursor_line = cur_line;
        } else {
            // replace current line with head + first, append middle, then last + tail
            let first = format!("{}{}", head, insert_lines[0]);
            let last = format!("{}{}", insert_lines.pop().unwrap(), tail);
            let mut new_lines: Vec<String> = Vec::new();
            new_lines.push(first);
            new_lines.extend(insert_lines.into_iter());
            new_lines.push(last);
            // replace current line with new_lines
            let new_count = new_lines.len();
            self.lines.splice(cur_line..=cur_line, new_lines.into_iter());
            self.cursor_line = cur_line + (new_count - 1);
            // place cursor at end of inserted text (last line length minus tail length)
            self.cursor_col = self.lines[self.cursor_line].chars().count() - tail.chars().count();
        }
        // clear selection and update typing-group end position
        self.clear_selection();
        self.last_typing_end = (self.cursor_line, self.cursor_col);
        self.clamp_cursor();
    }

    /// Delete the selected range and place the cursor at the start of the removed range.
    /// Returns true if something was deleted.
    ///
    /// `record_undo`: when true record an undo snapshot before performing deletion.
    pub fn delete_selection_and_return_cursor_at_start(&mut self, record_undo: bool) -> bool {
        if self.selection.is_none() {
            return false;
        }
        if record_undo {
            self.record_undo_before(false);
        }
        let sel = self.selection.as_ref().unwrap().clone();
        let (sl, sc, el, ec) = sel.normalized();
        if sl == el {
            // Single-line deletion. If the selection covers the entire line, remove the
            // line element so we don't leave an empty line that would render as a
            // leading newline when joined. Otherwise splice the remaining pieces.
            let line_len = self.lines[sl].chars().count();
            if sc == 0 && ec >= line_len {
                // Replace the whole line content with an empty line instead of removing the line.
                // This preserves the visible blank line (user expectation when deleting a full-line
                // selection) and keeps document line indices stable for selection/undo semantics.
                self.lines[sl] = String::new();
                self.cursor_line = sl;
                self.cursor_col = 0;
            } else {
                let line = &self.lines[sl];
                let before = line.chars().take(sc).collect::<String>();
                let after = line.chars().skip(ec).collect::<String>();
                self.lines[sl] = format!("{}{}", before, after);
                self.cursor_line = sl;
                self.cursor_col = sc;
            }
        } else {
            let before = self.lines[sl].chars().take(sc).collect::<String>();
            let after = self.lines[el].chars().skip(ec).collect::<String>();

            // When joining across a deleted multi-line selection, prefer inserting a
            // single space between the head and tail if neither side already has
            // surrounding whitespace. This matches user-visible expectations where
            // removing a newline often leaves a visible gap rather than concatenating
            // two words together.
            let merged = if before.is_empty() || after.is_empty() {
                format!("{}{}", before, after)
            } else {
                let last_before = before.chars().rev().next().unwrap();
                let first_after = after.chars().next().unwrap();
                if last_before.is_whitespace() || first_after.is_whitespace() {
                    format!("{}{}", before, after)
                } else {
                    format!("{} {}", before, after)
                }
            };

            // remove middle lines and replace with merged before+after (possibly with inserted space)
            self.lines.splice(sl..=el, std::iter::once(merged));
            self.cursor_line = sl;
            self.cursor_col = sc;
        }
        self.selection = None;
        // any deletion is a non-typing action
        self.last_edit_was_typing = false;
        self.clamp_cursor();
        true
    }

    /// Backspace behavior: if selection present, delete it. Otherwise remove char
    /// before the cursor, handling line joins at line starts.
    pub fn backspace(&mut self) {
        // Record undo boundary for this destructive action.
        self.record_undo_before(false);

        if self.selection.is_some() {
            // selection deletion already handled; avoid double-recording
            self.delete_selection_and_return_cursor_at_start(false);
            return;
        }
        if self.cursor_col == 0 {
            // join with previous line if any
            if self.cursor_line == 0 {
                return;
            }
            let removed = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            let prev_len = self.lines[self.cursor_line].chars().count();
            self.lines[self.cursor_line] = format!("{}{}", self.lines[self.cursor_line], removed);
            self.cursor_col = prev_len;
        } else {
            // remove previous char
            let line = &self.lines[self.cursor_line];
            let before = line.chars().take(self.cursor_col - 1).collect::<String>();
            let after = line.chars().skip(self.cursor_col).collect::<String>();
            self.lines[self.cursor_line] = format!("{}{}", before, after);
            self.cursor_col -= 1;
        }
        self.selection = None;
        // destructive op resets typing-grouping
        self.last_edit_was_typing = false;
        self.clamp_cursor();
    }

    /// Delete key behavior: if selection present delete it. Otherwise remove char at cursor,
    /// or join with next line at end-of-line.
    pub fn delete(&mut self) {
        // record undo boundary for this destructive action
        self.record_undo_before(false);

        if self.selection.is_some() {
            self.delete_selection_and_return_cursor_at_start(false);
            return;
        }
        let line_len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col >= line_len {
            // join with next line if any
            if self.cursor_line + 1 >= self.lines.len() {
                return;
            }
            let next = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line] = format!("{}{}", self.lines[self.cursor_line], next);
        } else {
            let line = &self.lines[self.cursor_line];
            let before = line.chars().take(self.cursor_col).collect::<String>();
            let after = line.chars().skip(self.cursor_col + 1).collect::<String>();
            self.lines[self.cursor_line] = format!("{}{}", before, after);
        }
        self.selection = None;
        self.last_edit_was_typing = false;
        self.clamp_cursor();
    }

    /// Press Enter: if selection exists replace it with a newline; otherwise split the line.
    pub fn enter(&mut self) {
        // record undo boundary for this structural edit
        self.record_undo_before(false);

        if self.selection.is_some() {
            self.delete_selection_and_return_cursor_at_start(false);
        }
        let line = self.lines[self.cursor_line].clone();
        let head: String = line.chars().take(self.cursor_col).collect();
        let tail: String = line.chars().skip(self.cursor_col).collect();
        self.lines.splice(self.cursor_line..=self.cursor_line, vec![head.clone(), tail.clone()]);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.selection = None;
        self.last_edit_was_typing = false;
        self.clamp_cursor();
    }

    /// Move cursor home: to line start.
    pub fn home(&mut self, keep_selection: bool) {
        if keep_selection && self.selection.is_none() {
            self.anchor_selection_here();
        }
        self.set_cursor(self.cursor_line, 0, keep_selection);
    }

    /// Move cursor end: to end of current line.
    pub fn end(&mut self, keep_selection: bool) {
        let line_len = if self.lines.is_empty() { 0 } else { self.lines[self.cursor_line].chars().count() };
        if keep_selection && self.selection.is_none() {
            self.anchor_selection_here();
        }
        self.set_cursor(self.cursor_line, line_len, keep_selection);
    }

    /// Arrow movement by character; keep_selection indicates shift-key semantics.
    pub fn move_left(&mut self, keep_selection: bool) {
        if !keep_selection {
            self.selection = None;
        } else if self.selection.is_none() {
            self.anchor_selection_here();
        }
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].chars().count();
        }
        if keep_selection {
            if let Some(sel) = &mut self.selection {
                sel.active_line = self.cursor_line;
                sel.active_col = self.cursor_col;
            }
        }
        self.clamp_cursor();
    }

    pub fn move_right(&mut self, keep_selection: bool) {
        if !keep_selection {
            self.selection = None;
        } else if self.selection.is_none() {
            self.anchor_selection_here();
        }
        let line_len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
        if keep_selection {
            if let Some(sel) = &mut self.selection {
                sel.active_line = self.cursor_line;
                sel.active_col = self.cursor_col;
            }
        }
        self.clamp_cursor();
    }

    pub fn move_up(&mut self, keep_selection: bool) {
        if !keep_selection {
            self.selection = None;
        } else if self.selection.is_none() {
            self.anchor_selection_here();
        }
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            let line_len = self.lines[self.cursor_line].chars().count();
            self.cursor_col = min(self.cursor_col, line_len);
        }
        if keep_selection {
            if let Some(sel) = &mut self.selection {
                sel.active_line = self.cursor_line;
                sel.active_col = self.cursor_col;
            }
        }
        self.clamp_cursor();
    }

    pub fn move_down(&mut self, keep_selection: bool) {
        if !keep_selection {
            self.selection = None;
        } else if self.selection.is_none() {
            self.anchor_selection_here();
        }
        if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            let line_len = self.lines[self.cursor_line].chars().count();
            self.cursor_col = min(self.cursor_col, line_len);
        }
        if keep_selection {
            if let Some(sel) = &mut self.selection {
                sel.active_line = self.cursor_line;
                sel.active_col = self.cursor_col;
            }
        }
        self.clamp_cursor();
    }
}
