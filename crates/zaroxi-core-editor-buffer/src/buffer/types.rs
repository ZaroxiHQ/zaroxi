use std::cmp::min;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub anchor_line: usize,
    pub anchor_col: usize,
    pub active_line: usize,
    pub active_col: usize,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub selection: Option<Selection>,
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

    /// Dirty flag: true when current content differs from last saved state.
    pub dirty: bool,
    /// Last known saved text (if any). Used to compute dirty state after undo/redo.
    pub saved_text: Option<String>,

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
            dirty: false,
            saved_text: Some(text.to_string()),
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
    pub(crate) fn clamp_cursor(&mut self) {
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
}
