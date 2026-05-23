use zaroxi_core_editor_buffer::buffer::{Buffer, Selection};
use std::sync::{Arc, Mutex};

/// Public snapshot type that the interface can consume to build presenter editor layout.
///
/// Lines are 0-based internally; the presenter EditorLayoutSpec uses 1-based
/// document lines, so the interface layer will map these fields accordingly.
#[derive(Debug, Clone)]
pub struct EditorSnapshot {
    pub lines: Vec<String>,
    /// top visible document line (1-based for presenter convenience). For our
    /// snapshot 1-based means 1 = first line.
    pub top_line: u32,
    pub cursor_line: Option<u32>,
    pub cursor_column: Option<u32>,
    /// Optional selection as (start_line, start_col, end_line, end_col) all 1-based
    pub selection: Option<(u32, u32, u32, u32)>,
}

pub struct EditorService {
    pub buffer: Arc<Mutex<Buffer>>,
}

impl EditorService {
    pub fn new_with_text(text: &str) -> Self {
        let buf = Buffer::from_text(text);
        Self {
            buffer: Arc::new(Mutex::new(buf)),
        }
    }

    /// Move arrow with optional shift (expand selection).
    pub fn arrow_left(&self, shift: bool) {
        let mut b = self.buffer.lock().unwrap();
        b.move_left(shift);
    }
    pub fn arrow_right(&self, shift: bool) {
        let mut b = self.buffer.lock().unwrap();
        b.move_right(shift);
    }
    pub fn arrow_up(&self, shift: bool) {
        let mut b = self.buffer.lock().unwrap();
        b.move_up(shift);
    }
    pub fn arrow_down(&self, shift: bool) {
        let mut b = self.buffer.lock().unwrap();
        b.move_down(shift);
    }

    pub fn home(&self, shift: bool) {
        let mut b = self.buffer.lock().unwrap();
        b.home(shift);
    }

    pub fn end(&self, shift: bool) {
        let mut b = self.buffer.lock().unwrap();
        b.end(shift);
    }

    /// Type a string (inserts or replaces selection).
    pub fn type_text(&self, text: &str) {
        let mut b = self.buffer.lock().unwrap();
        b.replace_selection_or_insert(text);
    }

    pub fn backspace(&self) {
        let mut b = self.buffer.lock().unwrap();
        b.backspace();
    }

    pub fn delete(&self) {
        let mut b = self.buffer.lock().unwrap();
        b.delete();
    }

    pub fn enter(&self) {
        let mut b = self.buffer.lock().unwrap();
        b.enter();
    }

    /// Copy selection into a String (application-layer returns the text; the interface
    /// layer owns the clipboard seam).
    pub fn copy_selection(&self) -> Option<String> {
        let b = self.buffer.lock().unwrap();
        b.selection_text()
    }

    /// Delete selection content (cut should call copy_selection first).
    pub fn delete_selection(&self) -> bool {
        let mut b = self.buffer.lock().unwrap();
        b.delete_selection_and_return_cursor_at_start()
    }

    /// Paste: replace selection or insert text at caret.
    pub fn paste_text(&self, text: &str) {
        let mut b = self.buffer.lock().unwrap();
        b.replace_selection_or_insert(text);
    }

    /// Snapshot for presenter consumption (adapter in interface layer will map 0-based -> 1-based).
    pub fn snapshot(&self) -> EditorSnapshot {
        let b = self.buffer.lock().unwrap();
        let cursor_line = Some(b.cursor_line as u32 + 1);
        let cursor_column = Some(b.cursor_col as u32);
        let selection = b.selection.as_ref().map(|s| {
            let (sl, sc, el, ec) = s.normalized();
            // convert to 1-based line indices for convenience in presenters
            (sl as u32 + 1, sc as u32, el as u32 + 1, ec as u32)
        });
        EditorSnapshot {
            lines: b.lines.clone(),
            top_line: 1,
            cursor_line,
            cursor_column,
            selection,
        }
    }

    /// Convenience test helper to read full text.
    pub fn get_text(&self) -> String {
        let b = self.buffer.lock().unwrap();
        b.to_text()
    }

    /// Convenience test helper to inspect selection (0-based normalized).
    pub fn get_selection_normalized(&self) -> Option<(usize, usize, usize, usize)> {
        let b = self.buffer.lock().unwrap();
        b.selection.as_ref().map(|s| s.normalized())
    }
}
