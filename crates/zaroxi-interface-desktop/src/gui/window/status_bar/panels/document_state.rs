//! Left-zone panel: document identity + state indicators.
//!
//! Always surfaces the active document name (or a clean "No file" when none is
//! open) so the status bar is never blank-looking, then quietly appends state
//! labels only when actually true: read-only, unsaved edits, or an in-flight
//! background parse.

use super::super::model::{DocumentState, StatusModel};

/// Build the document identity + state segments.
pub fn segments(model: &StatusModel) -> Vec<String> {
    // Document identity is always present — this is the primary guarantee that
    // the bar shows meaningful, stable content even with no workspace/cockpit data.
    let mut out = vec![match &model.file_name {
        Some(name) => name.clone(),
        None => "No file".to_string(),
    }];

    if model.has_file {
        if model.readonly {
            out.push("Read-only".to_string());
        }
        if model.modified {
            out.push("Modified".to_string());
        }
        if model.document_state == DocumentState::Parsing {
            out.push("Parsing\u{2026}".to_string());
        }
    }
    out
}
