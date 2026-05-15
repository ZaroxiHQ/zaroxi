//! Workspace manager.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A managed workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedWorkspace {
    /// Unique identifier for the workspace.
    pub id: Uuid,
    /// The root path.
    pub root_path: String,
    /// Whether the workspace is active.
    pub active: bool,
}

impl ManagedWorkspace {
    /// Create a new managed workspace.
    pub fn new(root_path: String) -> Self {
        Self { id: Uuid::new_v4(), root_path, active: true }
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
