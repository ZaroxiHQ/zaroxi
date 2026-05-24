use super::types::Buffer;

impl Buffer {
    /// Mark current buffer state as saved (update saved_text and clear dirty).
    pub fn set_saved_state(&mut self) {
        self.saved_text = Some(self.to_text());
        self.dirty = false;
        // reset typing group state
        self.last_edit_was_typing = false;
    }

    /// Replace entire buffer content from provided text and clear history.
    pub fn load_from_text(&mut self, text: &str) {
        self.lines = text.split('\n').map(|s| s.to_string()).collect();
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.selection = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.last_edit_was_typing = false;
        self.last_typing_end = (usize::MAX, usize::MAX);
        self.saved_text = Some(self.to_text());
        self.dirty = false;
        self.clamp_cursor();
    }
}
