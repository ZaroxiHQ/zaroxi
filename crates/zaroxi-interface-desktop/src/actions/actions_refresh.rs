/// Refresh action handlers — thin desktop delegates to shared
/// orchestration in `zaroxi_application_workspace::workspace_view`.
use std::sync::Arc;
use zaroxi_application_workspace::ports::{SessionId, WorkspaceService, WorkspaceView};
use zaroxi_application_workspace::workspace_view as ws;
use zaroxi_kernel_types::Id;

pub use zaroxi_application_workspace::workspace_view::{ActionResult, ShellActionResult};

pub async fn refresh_desktop(
    comp: &mut crate::desktop::DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
    service: Option<Arc<dyn WorkspaceService>>,
) -> Result<ActionResult, String> {
    ws::refresh_desktop(comp, view, session_id, workspace_id, service).await
}

pub async fn refresh_and_get_shell_context(
    comp: &mut crate::desktop::DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
    service: Option<Arc<dyn WorkspaceService>>,
) -> Result<ShellActionResult, String> {
    ws::refresh_and_get_shell_context(comp, view, session_id, workspace_id, service).await
}
