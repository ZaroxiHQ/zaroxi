//! Right-zone panel: caret position and selection.
//!
//! Line/column is always shown for an open file. A compact selection summary is
//! appended only while a selection is active.

use super::super::model::StatusModel;

/// Build the position segment(s): `Ln L, Col C` and an optional selection summary.
pub fn segments(model: &StatusModel) -> Vec<String> {
    if !model.has_file {
        return Vec::new();
    }

    let mut out = vec![format!("Ln {}, Col {}", model.line + 1, model.column + 1)];

    if let Some(sel) = &model.selection {
        if sel.lines > 1 {
            out.push(format!("Sel {} ({} ln)", sel.chars, sel.lines));
        } else {
            out.push(format!("Sel {}", sel.chars));
        }
    }

    out
}
