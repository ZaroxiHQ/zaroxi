use std::path::PathBuf;

use zaroxi_core_engine_ui::ExplorerPanelItem;

use crate::DesktopComposition;

/// Panel-level view model for the Explorer sidebar.
///
/// Built from `DesktopComposition` state before each render frame.
/// Carries enough information for the widget builder to construct
/// the full explorer section without further composition queries.
#[derive(Debug, Clone)]
pub struct ExplorerPanelViewModel {
    /// Panel header title (e.g. workspace name or "EXPLORER").
    pub title: Option<String>,
    /// Visible tree rows with depth, expand state, and active highlights.
    pub items: Vec<ExplorerPanelItem>,
    /// Label for the primary action button. When set and `items` is empty,
    /// the panel renders this button instead of an empty-state message.
    pub primary_action_label: Option<String>,
    /// Quiet empty-state message shown when no items and no primary action.
    pub empty_message: Option<String>,
    /// Workspace root path (set when a workspace is open).
    pub workspace_root: Option<PathBuf>,
}

impl ExplorerPanelViewModel {
    pub fn build(comp: &DesktopComposition) -> Self {
        let title = comp
            .workspace_root_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string());

        let items: Vec<ExplorerPanelItem> = comp
            .cached_explorer_items
            .iter()
            .map(|ev| {
                // The label is the plain filename. Disclosure chevron and the
                // per-type icon are composed at render time (see `rail.rs` +
                // `explorer_panel::icons`) into separate fixed-x columns so a
                // double-width Nerd Font icon can't misalign the name column.
                ExplorerPanelItem {
                    id: ev.id.clone(),
                    label: ev.name.clone(),
                    depth: ev.depth,
                    is_dir: ev.is_dir,
                    expanded: ev.expanded,
                    is_active: ev.is_active,
                }
            })
            .collect();

        let has_workspace = comp.workspace_root_path.is_some();

        let (primary_action_label, empty_message) = if has_workspace {
            (None, Some("No files in workspace".to_string()))
        } else {
            (Some("Open Workspace".to_string()), None)
        };

        Self {
            title,
            items,
            primary_action_label,
            empty_message,
            workspace_root: comp.workspace_root_path.clone(),
        }
    }
}
