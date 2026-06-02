/// Cursor/insert action handlers — thin desktop delegates to shared
/// orchestration in `zaroxi_application_workspace::workspace_view`.
use std::sync::Arc;
use zaroxi_application_workspace::ports::{SessionId, WorkspaceView};
use zaroxi_application_workspace::workspace_view as ws;
use zaroxi_application_workspace::workspace_view::ActionResult;
use zaroxi_kernel_types::Id;

pub async fn move_cursor_to_start_and_refresh(
    comp: &mut crate::desktop::DesktopComposition,
    service: Arc<dyn crate::ports::WorkspaceService>,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
) -> Result<ActionResult, String> {
    ws::move_cursor_to_start_and_refresh(comp, service, view, session_id, workspace_id).await
}

pub async fn insert_line_at_start_and_refresh(
    comp: &mut crate::desktop::DesktopComposition,
    service: Arc<dyn crate::ports::WorkspaceService>,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
) -> Result<ActionResult, String> {
    ws::insert_line_at_start_and_refresh(comp, service, view, session_id, workspace_id).await
}
