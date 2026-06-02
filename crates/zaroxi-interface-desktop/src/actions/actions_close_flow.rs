/// Thin desktop delegates: call shared close-flow action functions from
/// `zaroxi_application_workspace::workspace_view` with `DesktopComposition`
/// as the `CloseContext`.
use std::sync::Arc;
use zaroxi_application_workspace::ports::{SessionId, WorkspaceView};
use zaroxi_application_workspace::workspace_view::{self as ws, ActionResult};

pub async fn request_close_active(
    comp: &mut crate::desktop::DesktopComposition,
    _view: Arc<dyn WorkspaceView>,
    _session_id: SessionId,
) -> Result<ActionResult, String> {
    ws::request_close_active(comp).await
}

pub async fn request_close_session(
    comp: &mut crate::desktop::DesktopComposition,
    _view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
) -> Result<ActionResult, String> {
    ws::request_close_session(comp, session_id, service).await
}

pub async fn confirm_save_all_and_close(
    comp: &mut crate::desktop::DesktopComposition,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
) -> Result<ActionResult, String> {
    ws::confirm_save_all_and_close(comp, service, session_id).await
}

pub async fn confirm_discard_all_and_close(
    comp: &mut crate::desktop::DesktopComposition,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
) -> Result<ActionResult, String> {
    ws::confirm_discard_all_and_close(comp, service, session_id).await
}

pub async fn confirm_save_and_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    ws::confirm_save_and_close(comp).await
}

pub async fn confirm_discard_and_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    ws::confirm_discard_and_close(comp).await
}

pub async fn confirm_cancel_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    ws::confirm_cancel_close(comp).await
}
