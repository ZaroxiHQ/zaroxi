//! Left-zone panel: document state indicators.
//!
//! Quiet by default — only surfaces a label when something is actually true:
//! read-only, unsaved edits, or an in-flight background parse. Nothing is shown
//! when no file is open.

use super::super::model::{DocumentState, StatusModel};

/// Build the document-state segments (read-only / modified / parsing).
pub fn segments(model: &StatusModel) -> Vec<String> {
    if !model.has_file {
        return Vec::new();
    }

    let mut out = Vec::new();
    if model.readonly {
        out.push("Read-only".to_string());
    }
    if model.modified {
        out.push("Modified".to_string());
    }
    if model.document_state == DocumentState::Parsing {
        out.push("Parsing\u{2026}".to_string());
    }
    out
}
