pub mod editor_service;
/// Workspace service orchestration logic for Zaroxi Studio.
///
/// Application-level orchestrators (use-case services) live here. They depend on
/// domain contracts and core ports, but not on infrastructure or interface.
/// For Phase 1 keep implementations minimal and focused on the single slice.
pub mod service;
pub use editor_service::EditorService;
pub mod in_memory_adapters;
pub mod ports;
pub mod usecases;
pub mod view;
pub mod workspace_manager; // small, read-only view seam (Phase 2)

use std::path::PathBuf;
use std::io;
use zaroxi_core_workspace_files::FileStorage;

/// Thin application-level helpers for Phase 9 disk-backed operations.
///
/// These convenience functions are small facades used by integration tests and
/// simple harnesses; richer application commands should live in the ports/usecases.
pub fn save_buffer_to_disk(path: &PathBuf, contents: &str) -> io::Result<()> {
    let storage = zaroxi_core_workspace_files::DiskFileStorage::new();
    storage.write_file(path, contents)
}

pub fn read_file_from_disk(path: &PathBuf) -> io::Result<String> {
    let storage = zaroxi_core_workspace_files::DiskFileStorage::new();
    storage.read_file(path)
}

/// Prelude for convenient imports.
///
/// Be explicit about exported symbols to avoid ambiguous glob re-export warnings.
/// Re-export application-owned types and the orchestrator.
pub mod prelude {
    // Re-export the application-owned port/type surface explicitly.
    pub use crate::ports::{
        AppCommand, CommandResult, DispatchCommandRequest, DispatchCommandResponse,
        DynWorkspaceService, OpenBufferRequest, OpenBufferResponse, WorkspaceBootRequest,
        WorkspaceBootResponse, WorkspaceService, WorkspaceSessionDTO,
    };

    // Re-export the concrete orchestrator type for convenience.
    pub use crate::usecases::WorkspaceOrchestrator;

    // Re-export manager helpers.
    pub use crate::workspace_manager::*;

    // Re-export thin view helpers (Phase 2)
    pub use crate::view::*;

    // Re-export the lightweight explorer surface added in Phase 10.
    pub use crate::WorkspaceExplorer;
}

/// Small, local workspace tree model used by the initial explorer vertical slice.
///
/// NOTE:
/// - This is intentionally implemented inside the application crate for the
///   Phase 10 incremental slice. A future refactor will migrate the model to
///   `zaroxi-domain-workspace` and keep application as an orchestrator only.
#[derive(Clone, Debug)]
pub struct WorkspaceEntry {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<WorkspaceEntry>,
    pub expanded: bool,
}

#[derive(Clone, Debug)]
pub struct WorkspaceTree {
    pub root: WorkspaceEntry,
}

impl WorkspaceTree {
    /// Load a full recursive tree from the filesystem starting at `root_path`.
    pub fn load_from_fs(root_path: &PathBuf) -> io::Result<Self> {
        fn build(path: &PathBuf) -> io::Result<WorkspaceEntry> {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let is_dir = path.is_dir();
            let mut children = Vec::new();

            if is_dir {
                for (p, _is_dir) in zaroxi_core_workspace_files::list_dir_entries(path)? {
                    let child = build(&p)?;
                    children.push(child);
                }
            }

            Ok(WorkspaceEntry {
                id: path.to_string_lossy().to_string(),
                name,
                path: path.clone(),
                is_dir,
                children,
                expanded: false,
            })
        }

        let root = build(root_path)?;
        Ok(WorkspaceTree { root })
    }

    /// Find a mutable reference to an entry by id.
    pub fn find_mut_entry<'a>(
        node: &'a mut WorkspaceEntry,
        id: &str,
    ) -> Option<&'a mut WorkspaceEntry> {
        if node.id == id {
            return Some(node);
        }
        for child in &mut node.children {
            if let Some(found) = WorkspaceTree::find_mut_entry(child, id) {
                return Some(found);
            }
        }
        None
    }

    /// Find an immutable reference to an entry by id.
    pub fn find_entry<'a>(node: &'a WorkspaceEntry, id: &str) -> Option<&'a WorkspaceEntry> {
        if node.id == id {
            return Some(node);
        }
        for child in &node.children {
            if let Some(found) = WorkspaceTree::find_entry(child, id) {
                return Some(found);
            }
        }
        None
    }
}

/// Small application-side explorer surface for Phase 10.
///
/// Responsibilities:
/// - Load a WorkspaceTree from disk
/// - Toggle expand/collapse for directories
/// - Select an entry
/// - Open selected file (returns text via the existing read_file_from_disk helper)
pub struct WorkspaceExplorer {
    pub tree: Option<WorkspaceTree>,
    pub selected: Option<String>,
}

impl WorkspaceExplorer {
    pub fn new() -> Self {
        WorkspaceExplorer {
            tree: None,
            selected: None,
        }
    }

    /// Load a workspace tree rooted at `path`.
    pub fn load_workspace(&mut self, path: &PathBuf) -> io::Result<()> {
        let tree = WorkspaceTree::load_from_fs(path)?;
        self.tree = Some(tree);
        Ok(())
    }

    /// Toggle expand/collapse for a directory entry by id.
    pub fn toggle_expand(&mut self, id: &str) -> bool {
        if let Some(ref mut t) = self.tree {
            if let Some(node) = WorkspaceTree::find_mut_entry(&mut t.root, id) {
                if node.is_dir {
                    node.expanded = !node.expanded;
                    return true;
                }
            }
        }
        false
    }

    /// Select an entry by id. Returns true if selection succeeded.
    pub fn select(&mut self, id: &str) -> bool {
        if let Some(ref t) = self.tree {
            if WorkspaceTree::find_entry(&t.root, id).is_some() {
                self.selected = Some(id.to_string());
                return true;
            }
        }
        false
    }

    /// Open the currently selected entry if it is a file and return its contents.
    pub fn open_selected(&self) -> io::Result<Option<String>> {
        if let Some(ref sel) = self.selected {
            if let Some(ref t) = self.tree {
                if let Some(entry) = WorkspaceTree::find_entry(&t.root, sel) {
                    if !entry.is_dir {
                        return crate::read_file_from_disk(&entry.path).map(Some);
                    }
                }
            }
        }
        Ok(None)
    }

    /// Render a simple textual representation of the currently loaded tree.
    /// This is a tiny presenter used by tests and early UI wiring.
    pub fn render_text(&self) -> String {
        fn render_node(node: &WorkspaceEntry, indent: usize, out: &mut String) {
            let prefix = if node.is_dir { "📂" } else { "📄" };
            let _ = writeln!(
                out,
                "{}{} {}",
                "  ".repeat(indent),
                prefix,
                node.name
            );
            if node.is_dir && node.expanded {
                for c in &node.children {
                    render_node(c, indent + 1, out);
                }
            }
        }

        use std::fmt::Write;
        let mut out = String::new();
        if let Some(ref t) = self.tree {
            render_node(&t.root, 0, &mut out);
        }
        out
    }
}
