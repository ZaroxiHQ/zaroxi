//! Left-zone panel: workspace identity and transient document state.
//!
//! Restrained by design — it shows the workspace name. The `document_state`
//! match is the extension point for transient hints (`Parsing…`/`Saving…`) once
//! those signals exist; idle documents add no noise.

use super::super::model::{DocumentState, StatusModel};

/// Build the left-zone segments (workspace + optional transient state).
pub fn segments(model: &StatusModel) -> Vec<String> {
    let mut out = Vec::new();

    match &model.workspace {
        Some(name) => out.push(name.clone()),
        None => out.push("No Workspace".to_string()),
    }

    match model.document_state {
        // Phase 2 will surface transient states (parsing/saving) here.
        DocumentState::Ready | DocumentState::NoFile => {}
    }

    out
}
