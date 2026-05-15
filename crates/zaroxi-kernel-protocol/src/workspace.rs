//! Workspace-specific protocol types.

use serde::{Deserialize, Serialize};

/// A directory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    /// Path to the entry.
    pub path: String,
    /// Name of the entry.
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File type, if known.
    pub file_type: Option<String>,
}

/// Request to open a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkspaceRequest {
    /// Path to the workspace root.
    pub path: String,
}

/// Response from opening a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkspaceResponse {
    /// Unique workspace identifier.
    pub workspace_id: String,
    /// Root path of the workspace.
    pub root_path: String,
    /// Number of files in the workspace.
    pub file_count: usize,
}

/// Request to list a directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirectoryRequest {
    /// Path to the directory.
    pub path: String,
}

/// Response from listing a directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirectoryResponse {
    /// Entries in the directory.
    pub entries: Vec<DirectoryEntry>,
}
