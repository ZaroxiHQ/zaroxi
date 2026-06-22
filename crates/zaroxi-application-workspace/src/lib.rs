pub mod editor_service;
pub mod service;
pub use editor_service::EditorService;
pub mod in_memory_adapters;
pub mod ports;
pub mod usecases;
pub mod view;
pub mod workspace_manager;
pub mod workspace_view;

use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use zaroxi_core_workspace_files::FileStorage;
use zaroxi_domain_workspace::file_tree::ExplorerItemView;

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
    pub children_loaded: bool,
}

#[derive(Clone, Debug)]
pub struct WorkspaceTree {
    pub root: WorkspaceEntry,
}

/// Standard file-manager ordering for a set of sibling entries:
/// directories first, then files, each group sorted case-insensitively by name.
///
/// Sorting lives in the model/node builder (not at draw time) so that every
/// displayed tree level inherits the same deterministic ordering. Ties on the
/// case-insensitive key fall back to the raw name for stability.
fn sort_workspace_entries(children: &mut [WorkspaceEntry]) {
    children.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
}

impl WorkspaceTree {
    /// Load a tree from the filesystem, loading only immediate children.
    pub fn load_from_fs(root_path: &PathBuf) -> io::Result<Self> {
        fn build_shallow(path: &PathBuf) -> io::Result<WorkspaceEntry> {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let is_dir = path.is_dir();
            let mut children = Vec::new();
            let mut children_loaded = false;

            if is_dir {
                if let Ok(entries) = zaroxi_core_workspace_files::list_dir_entries(path) {
                    for (p, _is_dir) in entries {
                        let child_name = p
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.to_string_lossy().to_string());
                        children.push(WorkspaceEntry {
                            id: p.to_string_lossy().to_string(),
                            name: child_name,
                            path: p,
                            is_dir: _is_dir,
                            children: Vec::new(),
                            expanded: false,
                            children_loaded: false,
                        });
                    }
                    sort_workspace_entries(&mut children);
                    children_loaded = true;
                }
            }

            Ok(WorkspaceEntry {
                id: path.to_string_lossy().to_string(),
                name,
                path: path.clone(),
                is_dir,
                children,
                expanded: false,
                children_loaded,
            })
        }

        let root = build_shallow(root_path)?;
        Ok(WorkspaceTree { root })
    }

    /// Load children for a directory entry on demand.
    pub fn load_children(node: &mut WorkspaceEntry) -> io::Result<()> {
        if !node.is_dir || node.children_loaded {
            return Ok(());
        }

        let entries = zaroxi_core_workspace_files::list_dir_entries(&node.path)?;
        node.children = entries
            .into_iter()
            .map(|(p, is_dir)| {
                let child_name = p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.to_string_lossy().to_string());
                WorkspaceEntry {
                    id: p.to_string_lossy().to_string(),
                    name: child_name,
                    path: p,
                    is_dir,
                    children: Vec::new(),
                    expanded: false,
                    children_loaded: false,
                }
            })
            .collect();
        sort_workspace_entries(&mut node.children);
        node.children_loaded = true;
        Ok(())
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

    /// Flatten the tree into a depth-first list of `ExplorerItemView` rows
    /// suitable for the sidebar. Only expanded directories expose their children.
    pub fn flatten_explorer_items(
        &self,
        _opened_ids: &HashSet<String>,
        _active_id: Option<&str>,
    ) -> Vec<ExplorerItemView> {
        fn walk(
            node: &WorkspaceEntry,
            depth: usize,
            opened_ids: &HashSet<String>,
            active_id: Option<&str>,
            out: &mut Vec<ExplorerItemView>,
        ) {
            let is_open = opened_ids.contains(&node.id);
            let is_active = active_id.map_or(false, |a| a == node.id);

            out.push(ExplorerItemView {
                id: node.id.clone(),
                name: node.name.clone(),
                depth,
                is_dir: node.is_dir,
                expanded: node.expanded,
                is_open,
                is_active,
            });

            if node.is_dir && node.expanded {
                for child in &node.children {
                    walk(child, depth + 1, opened_ids, active_id, out);
                }
            }
        }

        let mut out = Vec::new();
        let opened_ids = _opened_ids;
        let active_id = _active_id;

        // Skip the root entry itself; start at depth 0 with root's children.
        for child in &self.root.children {
            walk(child, 0, opened_ids, active_id, &mut out);
        }
        out
    }
}

/// Small application-side explorer surface for Phase 10.
///
/// Responsibilities:
/// - Load a WorkspaceTree from disk
/// - Toggle expand/collapse for directories
/// - Select an entry
/// - Open selected file (returns text via the existing read_file_from_disk helper)
#[derive(Clone, Debug)]
pub struct WorkspaceExplorer {
    pub tree: Option<WorkspaceTree>,
    pub selected: Option<String>,
}

impl WorkspaceExplorer {
    pub fn new() -> Self {
        WorkspaceExplorer { tree: None, selected: None }
    }

    /// Load a workspace tree rooted at `path`.
    pub fn load_workspace(&mut self, path: &PathBuf) -> io::Result<()> {
        let tree = WorkspaceTree::load_from_fs(path)?;
        self.tree = Some(tree);
        Ok(())
    }

    /// Toggle expand/collapse for a directory entry by id.
    /// Lazily loads children on first expand.
    pub fn toggle_expand(&mut self, id: &str) -> bool {
        if let Some(ref mut t) = self.tree {
            if let Some(node) = WorkspaceTree::find_mut_entry(&mut t.root, id) {
                if node.is_dir {
                    if !node.expanded && !node.children_loaded {
                        if let Err(_e) = WorkspaceTree::load_children(node) {
                            return false;
                        }
                    }
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
            let _ = writeln!(out, "{}{} {}", "  ".repeat(indent), prefix, node.name);
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

    /// Produce a flat, depth-first list of `ExplorerItemView` rows for the sidebar.
    ///
    /// `opened_paths` is the set of file paths currently opened as buffers.
    /// `active_path` is the currently active buffer path, if any.
    pub fn visible_items(
        &self,
        opened_paths: &HashSet<String>,
        active_path: Option<&str>,
    ) -> Vec<ExplorerItemView> {
        match self.tree.as_ref() {
            Some(t) => t.flatten_explorer_items(opened_paths, active_path),
            None => Vec::new(),
        }
    }

    /// Return the filesystem path for an explorer entry by its id.
    pub fn get_entry_path(&self, id: &str) -> Option<PathBuf> {
        self.tree
            .as_ref()
            .and_then(|t| WorkspaceTree::find_entry(&t.root, id))
            .map(|e| e.path.clone())
    }

    /// Return true if the entry with the given id is a directory.
    pub fn is_dir(&self, id: &str) -> bool {
        self.tree
            .as_ref()
            .and_then(|t| WorkspaceTree::find_entry(&t.root, id))
            .map(|e| e.is_dir)
            .unwrap_or(false)
    }
}
