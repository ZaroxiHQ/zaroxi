use super::types::{Buffer, Snapshot};
use std::cmp::min;

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

impl Buffer {
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
            let max_line_idx =
                if self.lines.is_empty() { 0 } else { self.lines.len().saturating_sub(1) };
            sel.anchor_line = min(sel.anchor_line, max_line_idx);
            sel.active_line = min(sel.active_line, max_line_idx);

            // Clamp column indices to the respective line lengths.
            let anchor_line_len =
                self.lines.get(sel.anchor_line).map(|l| l.chars().count()).unwrap_or(0);
            let active_line_len =
                self.lines.get(sel.active_line).map(|l| l.chars().count()).unwrap_or(0);
            sel.anchor_col = min(sel.anchor_col, anchor_line_len);
            sel.active_col = min(sel.active_col, active_line_len);
        }

        // Recompute dirty state relative to last saved text (if known).
        self.dirty = self.saved_text.as_ref().map(|s| s != &self.to_text()).unwrap_or(true);
    }

    /// Record an "undo boundary" before a mutating operation.
    /// - is_typing: whether this operation should be considered typing (single-char inserts).
    /// Consecutive typing operations at contiguous positions are merged.
    pub(crate) fn record_undo_before(&mut self, is_typing: bool) {
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
}
