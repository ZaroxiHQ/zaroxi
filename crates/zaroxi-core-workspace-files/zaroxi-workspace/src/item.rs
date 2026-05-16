use serde::{Deserialize, Serialize};

/// A simple placeholder workspace item representing a file or folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceItem {
    /// Display name shown in the sidebar.
    pub name: String,
    /// Path placeholder (v1).
    pub path: Option<String>,
    /// Whether this item is selected in the UI.
    pub selected: bool,
}

impl WorkspaceItem {
    pub fn file(name: impl Into<String>, path: Option<String>) -> Self {
        Self {
            name: name.into(),
            path,
            selected: false,
        }
    }
}
