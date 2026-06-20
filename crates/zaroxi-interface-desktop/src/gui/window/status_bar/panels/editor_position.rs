//! Right-zone panel: caret line/column.
//!
//! Displayed only when a file is open. Columns and lines are 1-based in the UI.

use super::super::model::StatusModel;

/// Build the line/column segment.
pub fn segments(model: &StatusModel) -> Vec<String> {
    if !model.has_file {
        return Vec::new();
    }
    vec![format!("Ln {}, Col {}", model.line + 1, model.column + 1)]
}
