//! File tree representation for workspaces.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A node in the file tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileTreeNode {
    /// A directory containing other nodes.
    Directory {
        /// Path to the directory.
        path: PathBuf,
        /// Name of the directory.
        name: String,
        /// Children nodes.
        children: Vec<FileTreeNode>,
    },
    /// A file.
    File {
        /// Path to the file.
        path: PathBuf,
        /// Name of the file.
        name: String,
        /// File extension, if any.
        extension: Option<String>,
        /// File size in bytes.
        size: u64,
    },
}

impl FileTreeNode {
    /// Create a directory node.
    pub fn directory(path: PathBuf) -> Self {
        let name =
            path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()).unwrap_or_default();
        Self::Directory { path, name, children: Vec::new() }
    }

    /// Create a file node.
    pub fn file(path: PathBuf, size: u64) -> Self {
        let name =
            path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()).unwrap_or_default();
        let extension = path.extension().and_then(|e| e.to_str()).map(|s| s.to_string());
        Self::File { path, name, extension, size }
    }

    /// Get the path of the node.
    pub fn path(&self) -> &Path {
        match self {
            FileTreeNode::Directory { path, .. } => path,
            FileTreeNode::File { path, .. } => path,
        }
    }

    /// Get the name of the node.
    pub fn name(&self) -> &str {
        match self {
            FileTreeNode::Directory { name, .. } => name,
            FileTreeNode::File { name, .. } => name,
        }
    }
}

/// A flat, display-oriented view of a single tree item for the Explorer panel.
///
/// This type carries enough structure for the interface layer to render a
/// visible tree row (name, depth, folder/file glyph) without needing direct
/// access to the underlying file tree. It is app-neutral and serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerItemView {
    /// Stable identifier (the node's path as a string).
    pub id: String,
    /// Display name (file or directory name).
    pub name: String,
    /// Indentation depth (0 = first-level children of workspace root).
    pub depth: usize,
    /// Whether this item is a directory.
    pub is_dir: bool,
    /// Whether this directory is currently expanded (only meaningful for directories).
    pub expanded: bool,
    /// Whether this file/directory corresponds to an opened buffer.
    pub is_open: bool,
    /// Whether this item is the currently active buffer.
    pub is_active: bool,
}

/// A file tree representing the structure of a workspace.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FileTree {
    /// The root node of the tree.
    pub root: Option<FileTreeNode>,
}

impl FileTree {
    /// Create a new empty file tree.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Set the root node.
    pub fn set_root(&mut self, root: FileTreeNode) {
        self.root = Some(root);
    }
}
