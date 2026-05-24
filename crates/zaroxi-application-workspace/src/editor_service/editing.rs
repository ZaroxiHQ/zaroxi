use super::*;

impl EditorService {
    /// Move arrow with optional shift (expand selection).
    pub fn arrow_left(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.move_left(shift);
        }
    }
    pub fn arrow_right(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.move_right(shift);
        }
    }
    pub fn arrow_up(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.move_up(shift);
        }
    }
    pub fn arrow_down(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.move_down(shift);
        }
    }

    pub fn home(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.home(shift);
        }
    }

    pub fn end(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.end(shift);
        }
    }

    /// Type a string (inserts or replaces selection).
    pub fn type_text(&self, text: &str) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.replace_selection_or_insert(text);
        }
    }

    pub fn backspace(&self) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.backspace();
        }
    }

    pub fn delete(&self) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.delete();
        }
    }

    pub fn enter(&self) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.enter();
        }
    }

    /// Copy selection into a String (application-layer returns the text; the interface
    /// layer owns the clipboard seam).
    pub fn copy_selection(&self) -> Option<String> {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let b = buf_arc.lock().unwrap();
            b.selection_text()
        } else {
            None
        }
    }

    /// Delete selection content (cut should call copy_selection first).
    pub fn delete_selection(&self) -> bool {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.delete_selection_and_return_cursor_at_start(true)
        } else {
            false
        }
    }

    /// Paste: read clipboard and paste into active buffer.
    pub fn paste_text(&self, text: &str) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            // record insertion start
            let start_line = b.cursor_line;
            let start_col = b.cursor_col;
            b.replace_selection_or_insert(text);
            // record insertion end (cursor is placed at end of inserted text)
            let end_line = b.cursor_line;
            let end_col = b.cursor_col;
            b.selection = Some(Selection {
                anchor_line: start_line,
                anchor_col: start_col,
                active_line: end_line,
                active_col: end_col,
            });
        }
    }

    /// Undo last edit (returns true if an undo was performed).
    pub fn undo(&self) -> bool {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            let res = b.undo();
            // After undo, if buffer has saved_text, recompute dirty accordingly
            b.dirty = b
                .saved_text
                .as_ref()
                .map(|s| s != &b.to_text())
                .unwrap_or(true);
            res
        } else {
            false
        }
    }

    /// Redo previously undone edit (returns true if a redo was performed).
    pub fn redo(&self) -> bool {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            let res = b.redo();
            b.dirty = b
                .saved_text
                .as_ref()
                .map(|s| s != &b.to_text())
                .unwrap_or(true);
            res
        } else {
            false
        }
    }
}
