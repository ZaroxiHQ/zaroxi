//! Left-zone panel: workspace identity.
//!
//! Restrained by design — it shows the workspace folder name, or a clean
//! "No Workspace" when none is open. Document-level status moved to the
//! dedicated `document_state` panel.

use super::super::model::StatusModel;

/// Build the workspace segment.
pub fn segments(model: &StatusModel) -> Vec<String> {
    match &model.workspace {
        Some(name) => vec![name.clone()],
        None => vec!["No Workspace".to_string()],
    }
}
