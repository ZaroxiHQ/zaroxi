use super::*;

impl EditorService {
    /// Snapshot for presenter consumption (adapter in interface layer will map 0-based -> 1-based).
    /// If no active buffer exists this returns an empty/none snapshot.
    pub fn snapshot(&self) -> EditorSnapshot {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let b = buf_arc.lock().unwrap();
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
        } else {
            EditorSnapshot {
                lines: Vec::new(),
                top_line: 1,
                cursor_line: None,
                cursor_column: None,
                selection: None,
                dirty: false,
            }
        }
    }

    /// Convenience test helper to read full text from active buffer.
    pub fn get_text(&self) -> String {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let b = buf_arc.lock().unwrap();
            b.to_text()
        } else {
            String::new()
        }
    }

    /// Convenience test helper to inspect selection (0-based normalized) from active buffer.
    pub fn get_selection_normalized(&self) -> Option<(usize, usize, usize, usize)> {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let b = buf_arc.lock().unwrap();
            b.selection.as_ref().map(|s| s.normalized())
        } else {
            None
        }
    }
}
