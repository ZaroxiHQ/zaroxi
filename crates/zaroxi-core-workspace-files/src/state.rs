use serde::{Deserialize, Serialize};
use crate::item::WorkspaceItem;
use std::path::PathBuf;
use std::fmt;
use std::str::FromStr;
use zaroxi_kernel_id::UuidId;
use uuid;

/// Canonical, semantic workspace identifier (kernel-backed).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub UuidId);

impl WorkspaceId {
    /// Generate a new random workspace id (v4).
    pub fn new_v4() -> Self {
        WorkspaceId(UuidId::new_v4())
    }

    /// Borrow inner UuidId.
    pub fn as_uuid(&self) -> &UuidId {
        &self.0
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for WorkspaceId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(WorkspaceId(s.parse()?))
    }
}

impl From<UuidId> for WorkspaceId {
    fn from(u: UuidId) -> Self {
        WorkspaceId(u)
    }
}

impl From<WorkspaceId> for UuidId {
    fn from(s: WorkspaceId) -> UuidId {
        s.0
    }
}

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
