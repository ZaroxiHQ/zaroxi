/*!
Tiny, read-only text view model for the currently visible editor surface.

Purpose:
- Provide a minimal shell-facing projection answering:
  - "What is the currently visible text for the active buffer?"
  - "Where is the cursor (line/column) in that visible text?"
- Compose on top of DesktopComposition / Presenter output (InterfaceRenderableWindow).
- Keep the model purely read-only and presentation-focused: no IO, no mutation, no rendering.

Design:
- TextView is constructed from a DesktopComposition reference.
- It uses the presenter's InterfaceRenderableWindow (if available) and maps
  each InterfaceRenderableLine to a single String line (joining span texts).
- Cursor location is derived by scanning render spans for Cursor / SelectionCursor
  kinds; we report the 1-based line_number (matching visible window numbering)
  and 0-based character column.
*/

use crate::desktop::DesktopComposition;
use crate::view_adapter::{InterfaceRenderableWindow, InterfaceSpanKind};

/// Tiny read-only visible-text view model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextView {
    /// 1-based top line number of the visible window.
    pub top_line: usize,
    /// Total number of lines in the buffer/document.
    pub total_lines: usize,
    /// Lines of text visible in the window (each String is a full line).
    pub lines: Vec<String>,
    /// Cursor line number in 1-based coordinates relative to the document (None when absent).
    pub cursor_line: Option<usize>,
    /// Cursor column as 0-based character index within the line (None when absent).
    pub cursor_column: Option<usize>,
}

impl TextView {
    /// Build a TextView from the current DesktopComposition.
    ///
    /// Returns None when no presenter window is available.
    pub fn from_composition(comp: &DesktopComposition) -> Option<Self> {
        let win = comp.latest_window()?;
        Self::from_window(&win)
    }

    /// Build a TextView from an InterfaceRenderableWindow.
    pub fn from_window(win: &InterfaceRenderableWindow) -> Option<Self> {
        let mut lines: Vec<String> = Vec::with_capacity(win.lines.len());
        let mut cursor_line: Option<usize> = None;
        let mut cursor_column: Option<usize> = None;

        for line in win.lines.iter() {
            // Reconstruct the line text by concatenating span.text in order.
            let mut reconstructed = String::new();
            for sp in line.spans.iter() {
                reconstructed.push_str(&sp.text);
            }
            lines.push(reconstructed);

            // If we haven't found a cursor yet, scan spans on this line.
            if cursor_line.is_none() {
                for sp in line.spans.iter() {
                    match sp.kind {
                        InterfaceSpanKind::Cursor | InterfaceSpanKind::SelectionCursor => {
                            // Record 1-based line_number and 0-based column.
                            cursor_line = Some(line.line_number);
                            cursor_column = Some(sp.start_col);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        Some(TextView {
            top_line: win.top_line,
            total_lines: win.total_lines,
            lines,
            cursor_line,
            cursor_column,
        })
    }

    /// Return lines annotated with a simple cursor marker inserted at the cursor column.
    ///
    /// - `marker`: string to insert at the cursor position (e.g., "|^|").
    /// - When cursor is absent or on a line not present in the window this returns lines unmodified.
    pub fn lines_with_cursor_marker(&self, marker: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::with_capacity(self.lines.len());
        for (idx, line) in self.lines.iter().enumerate() {
            // line_number for this entry (1-based) = top_line + idx
            let line_number = self.top_line.saturating_add(idx);
            if Some(line_number) == self.cursor_line
                && let Some(col) = self.cursor_column
            {
                // Special-case insertion at column 0 to preserve an intentional
                // single separating space between the marker and the visible text.
                // - If the line already starts with a space, do not add another.
                // - Otherwise, insert exactly one space after the marker.
                // This keeps other cursor positions behaviour unchanged.
                if col == 0 {
                    if line.starts_with(' ') {
                        out.push(format!("{}{}", marker, line));
                    } else {
                        out.push(format!("{} {}", marker, line));
                    }
                    continue;
                }

                // Insert marker at character column `col` (0-based). Be Unicode-safe by using char indices.
                let mut acc = String::new();
                let mut char_iter = line.chars();
                let mut i = 0usize;
                while i < col {
                    if let Some(ch) = char_iter.next() {
                        acc.push(ch);
                        i += 1;
                    } else {
                        break;
                    }
                }
                acc.push_str(marker);
                // append remainder
                acc.extend(char_iter);
                out.push(acc);
                continue;
            }
            // default
            out.push(line.clone());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_marker_at_col0_inserts_space() {
        // Cursor at column 0 on a line that does not start with a space should yield
        // "|^| sample file"
        let tv = TextView {
            top_line: 1,
            total_lines: 1,
            lines: vec!["sample file".to_string()],
            cursor_line: Some(1),
            cursor_column: Some(0),
        };
        let out = tv.lines_with_cursor_marker("|^|");
        assert_eq!(out, vec!["|^| sample file".to_string()]);

        // If the visible line already starts with a space, do not insert an extra space.
        let tv2 = TextView {
            top_line: 1,
            total_lines: 1,
            lines: vec![" sample file".to_string()],
            cursor_line: Some(1),
            cursor_column: Some(0),
        };
        let out2 = tv2.lines_with_cursor_marker("|^|");
        assert_eq!(out2, vec!["|^| sample file".to_string()]);
    }
}
