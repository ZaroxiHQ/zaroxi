use std::path::PathBuf;
use std::sync::Arc;

use pollster;
use zaroxi_application_workspace::ports::{
    SessionId, WorkspaceBootRequest, WorkspaceService, WorkspaceView,
};
use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_kernel_types::Id;

use crate::DesktopComposition;
use crate::folder_picker::DynFolderPicker;

/// Panel-level action handler for the Explorer sidebar.
///
/// All explorer interactions — toggle expand, open/activate file,
/// open workspace — are routed through methods on this helper.
/// The `GuiApp` owns a single instance and delegates to it from
/// `dispatch_activation`.
pub struct ExplorerPanelActions {
    folder_picker: Option<DynFolderPicker>,
}

impl ExplorerPanelActions {
    pub fn new(folder_picker: Option<DynFolderPicker>) -> Self {
        Self { folder_picker }
    }

    /// Toggle expand/collapse for a directory explorer item.
    /// Returns updated work content, or None on failure.
    pub fn toggle_directory(
        &mut self,
        comp: &mut DesktopComposition,
        explorer_idx: usize,
    ) -> Option<ShellWorkContent> {
        let item_id = comp.get_explorer_item_id_at(explorer_idx)?;
        if let Some(ref mut explorer) = comp.maybe_explorer {
            explorer.toggle_expand(&item_id);
        }
        comp.refresh_cached_explorer_items();
        Some(comp.build_work_content())
    }

    /// Open a file from the explorer (by its item index). Opens the buffer
    /// if not already open, then activates it.
    /// Returns updated work content, or None on failure.
    pub fn open_file(
        &mut self,
        comp: &mut DesktopComposition,
        service: Arc<dyn WorkspaceService>,
        view: Arc<dyn WorkspaceView>,
        session_id: SessionId,
        workspace_id: Option<Id>,
        explorer_idx: usize,
    ) -> Option<ShellWorkContent> {
        let item_id = comp.get_explorer_item_id_at(explorer_idx)?;

        let path = comp.maybe_explorer.as_ref()?.get_entry_path(&item_id)?;

        let buf_id = pollster::block_on(crate::actions::open_buffer_by_path(
            comp,
            service.clone(),
            session_id.clone(),
            path,
        ))
        .ok()??;

        pollster::block_on(crate::actions::set_active_buffer_and_get_shell_context(
            comp,
            service,
            view,
            session_id,
            workspace_id,
            buf_id,
        ))
        .ok()?;

        Some(comp.build_work_content())
    }

    /// Open a workspace from a folder path. Boots the workspace session,
    /// loads the explorer tree, and refreshes composition.
    /// Returns updated work content, or None on failure.
    pub fn open_workspace(
        &mut self,
        comp: &mut DesktopComposition,
        service: Arc<dyn WorkspaceService>,
        view: Arc<dyn WorkspaceView>,
        session_id: &mut Option<SessionId>,
        workspace_id: &mut Option<Id>,
        path: PathBuf,
    ) -> Option<ShellWorkContent> {
        let boot_req = WorkspaceBootRequest { path: path.clone() };
        let boot_res = pollster::block_on(service.boot_workspace(boot_req)).ok()?;

        *session_id = Some(boot_res.session.session_id.clone());
        *workspace_id = Some(boot_res.session.workspace_id);

        comp.workspace_root_path = Some(path);
        comp.load_or_refresh_explorer();

        let _ = pollster::block_on(crate::actions::refresh_desktop(
            comp,
            view,
            boot_res.session.session_id.clone(),
            Some(boot_res.session.workspace_id),
            Some(service),
        ));

        Some(comp.build_work_content())
    }

    /// Trigger folder picker, then if a path is selected, delegate to `open_workspace`.
    /// Returns updated work content if a workspace was successfully opened.
    pub fn open_workspace_from_picker(
        &mut self,
        comp: &mut DesktopComposition,
        service: Arc<dyn WorkspaceService>,
        view: Arc<dyn WorkspaceView>,
        session_id: &mut Option<SessionId>,
        workspace_id: &mut Option<Id>,
    ) -> Option<ShellWorkContent> {
        let path = self.folder_picker.as_ref()?.pick_folder()?;
        self.open_workspace(comp, service, view, session_id, workspace_id, path)
    }
}
