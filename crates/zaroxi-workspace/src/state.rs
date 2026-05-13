use serde::{Deserialize, Serialize};
use crate::item::WorkspaceItem;
use std::path::PathBuf;

/// Minimal workspace state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    /// Root path (None for no workspace opened).
    pub root: Option<PathBuf>,
    /// Items contained in the workspace sidebar (v1 placeholders).
    pub items: Vec<WorkspaceItem>,
    /// Index of the selected item in `items`, if any.
    pub selected: Option<usize>,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self {
            root: None,
            items: Vec::new(),
            selected: None,
        }
    }

    pub fn select(&mut self, idx: Option<usize>) {
        if let Some(i) = self.selected {
            if let Some(prev) = self.items.get_mut(i) {
                prev.selected = false;
            }
        }
        self.selected = idx;
        if let Some(i) = idx {
            if let Some(item) = self.items.get_mut(i) {
                item.selected = true;
            }
        }
    }
}
