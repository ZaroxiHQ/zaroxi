/// Buffer activation handlers — thin desktop delegates to shared
/// orchestration in `zaroxi_application_workspace::workspace_view`.
use zaroxi_application_workspace::ports::{SessionId, WorkspaceView};
use zaroxi_application_workspace::workspace_view as ws;
use zaroxi_application_workspace::workspace_view::ShellActionResult;

pub async fn set_active_buffer_and_get_shell_context(
    comp: &mut crate::desktop::DesktopComposition,
    service: std::sync::Arc<dyn crate::ports::WorkspaceService>,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    buffer_id: crate::ports::BufferId,
) -> Result<ShellActionResult, String> {
    ws::set_active_buffer_and_get_shell_context(
        comp,
        service,
        view,
        session_id,
        workspace_id,
        buffer_id,
    )
    .await
}
