/// Buffer activation handlers — thin desktop delegates to shared
/// orchestration in `zaroxi_application_workspace::workspace_view`.
use std::path::PathBuf;
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

pub async fn open_buffer_by_path(
    comp: &mut crate::desktop::DesktopComposition,
    service: std::sync::Arc<dyn crate::ports::WorkspaceService>,
    session_id: SessionId,
    path: PathBuf,
) -> Result<Option<crate::ports::BufferId>, String> {
    use zaroxi_application_workspace::ports::OpenBufferRequest;

    let req = OpenBufferRequest { session_id, path };
    match service.open_buffer(req).await {
        Ok(resp) => {
            comp.set_pending_refresh_reason(
                zaroxi_application_workspace::workspace_view::RefreshReason::ActiveBufferChanged,
            );
            Ok(Some(resp.buffer_id))
        }
        Err(e) => {
            log::warn!("open_buffer_by_path failed: {:?}", e);
            Ok(None)
        }
    }
}
