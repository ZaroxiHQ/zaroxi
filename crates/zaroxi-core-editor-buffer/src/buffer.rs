use std::cmp::min;

/// Simple in-memory text buffer with stable line/column addressing and a basic
/// selection model suitable for use by the application and interface layers.
/// This intentionally stays small and dependency-free to keep the Phase-5
/// implementation focused on ergonomics (no undo/redo yet).
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
}

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

impl Buffer {
    /// Create a buffer from full text. Lines split on '\n' (do not keep newlines).
    pub fn from_text(text: &str) -> Self {
        let lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
        Buffer {
            lines,
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
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
        Some(parts.join("\n"))
    }

    /// Replace the selection (if present) with `text`. If no selection, insert at cursor.
    /// After operation the cursor is placed at the end of the inserted text and selection cleared.
    pub fn replace_selection_or_insert(&mut self, text: &str) {
        if self.selection.is_some() {
            self.delete_selection_and_return_cursor_at_start();
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
            self.cursor_col = head.chars().count() + insert_lines[0].chars().count();
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
        self.clear_selection();
        self.clamp_cursor();
    }

    /// Delete the selected range and place the cursor at the start of the removed range.
    /// Returns true if something was deleted.
    pub fn delete_selection_and_return_cursor_at_start(&mut self) -> bool {
        if self.selection.is_none() {
            return false;
        }
        let sel = self.selection.as_ref().unwrap().clone();
        let (sl, sc, el, ec) = sel.normalized();
        if sl == el {
            let line = &self.lines[sl];
            let before = line.chars().take(sc).collect::<String>();
            let after = line.chars().skip(ec).collect::<String>();
            self.lines[sl] = format!("{}{}", before, after);
            self.cursor_line = sl;
            self.cursor_col = sc;
        } else {
            let before = self.lines[sl].chars().take(sc).collect::<String>();
            let after = self.lines[el].chars().skip(ec).collect::<String>();
            // remove middle lines and replace with merged before+after
            self.lines.splice(sl..=el, std::iter::once(format!("{}{}", before, after)));
            self.cursor_line = sl;
            self.cursor_col = sc;
        }
        self.selection = None;
        self.clamp_cursor();
        true
    }

    /// Backspace behavior: if selection present, delete it. Otherwise remove char
    /// before the cursor, handling line joins at line starts.
    pub fn backspace(&mut self) {
        if self.selection.is_some() {
            self.delete_selection_and_return_cursor_at_start();
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
        self.clamp_cursor();
    }

    /// Delete key behavior: if selection present delete it. Otherwise remove char at cursor,
    /// or join with next line at end-of-line.
    pub fn delete(&mut self) {
        if self.selection.is_some() {
            self.delete_selection_and_return_cursor_at_start();
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
        self.clamp_cursor();
    }

    /// Press Enter: if selection exists replace it with a newline; otherwise split the line.
    pub fn enter(&mut self) {
        if self.selection.is_some() {
            self.delete_selection_and_return_cursor_at_start();
        }
        let line = self.lines[self.cursor_line].clone();
        let head: String = line.chars().take(self.cursor_col).collect();
        let tail: String = line.chars().skip(self.cursor_col).collect();
        self.lines.splice(self.cursor_line..=self.cursor_line, vec![head.clone(), tail.clone()]);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.selection = None;
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
