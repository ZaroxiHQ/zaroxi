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
    /// Whether the buffer contains unsaved edits.
    pub dirty: bool,
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

    /// Create an EditorService by loading file contents from path.
    pub fn new_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let buf = Buffer::from_text(&content);
        Ok(Self {
            buffer: Arc::new(Mutex::new(buf)),
        })
    }

    /// Save current buffer contents to the given path and mark as saved.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        // copy text under lock to avoid holding lock while writing to disk
        let text = {
            let b = self.buffer.lock().unwrap();
            b.to_text()
        };
        std::fs::write(path, text.as_bytes())?;
        // update buffer saved state
        let mut b = self.buffer.lock().unwrap();
        b.saved_text = Some(b.to_text());
        b.dirty = false;
        Ok(())
    }

    /// Reload buffer contents from disk: replace buffer text and reset history.
    pub fn reload(&self, path: &std::path::Path) -> std::io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let mut b = self.buffer.lock().unwrap();
        b.load_from_text(&content);
        Ok(())
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
        b.delete_selection_and_return_cursor_at_start(true)
    }

    /// Paste: replace selection or insert text at caret.
    /// After pasting, set the selection to the inserted range (anchor at
    /// insertion start, active at insertion end) so presenters can highlight
    /// pasted content in downstream projections/tests.
    pub fn paste_text(&self, text: &str) {
        let mut b = self.buffer.lock().unwrap();
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
            dirty: b.dirty,
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

    /// Undo last edit (returns true if an undo was performed).
    pub fn undo(&self) -> bool {
        let mut b = self.buffer.lock().unwrap();
        b.undo()
    }

    /// Redo previously undone edit (returns true if a redo was performed).
    pub fn redo(&self) -> bool {
        let mut b = self.buffer.lock().unwrap();
        b.redo()
    }
}
