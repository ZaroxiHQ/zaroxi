
//! Workspace manager.

use serde::{Deserialize, Serialize};
use zaroxi_kernel_types::Id;

/// A managed workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedWorkspace {
    /// Unique identifier for the workspace.
    pub id: Id,
    /// The root path.
    pub root_path: String,
    /// Whether the workspace is active.
    pub active: bool,
}

impl ManagedWorkspace {
    /// Create a new managed workspace.
    pub fn new(root_path: String) -> Self {
        Self { id: Id::new(), root_path, active: true }
    }

    /// Deactivate the workspace.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Activate the workspace.
    pub fn activate(&mut self) {
        self.active = true;
    }
}
