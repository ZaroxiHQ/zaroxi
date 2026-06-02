/// Command-bar action handlers — thin desktop delegates to shared
/// orchestration in `zaroxi_application_workspace::workspace_view`.
use crate::desktop::DesktopComposition;
use std::sync::Arc;
use zaroxi_application_workspace::ports::{SessionId, WorkspaceView};
use zaroxi_application_workspace::workspace_view as ws;
use zaroxi_application_workspace::workspace_view::ActionResult;

pub async fn open_command_bar(comp: &mut DesktopComposition) -> Result<ActionResult, String> {
    ws::open_command_bar(comp).await
}
pub async fn close_command_bar(comp: &mut DesktopComposition) -> Result<ActionResult, String> {
    ws::close_command_bar(comp).await
}
pub async fn navigate_command_bar_next(
    comp: &mut DesktopComposition,
) -> Result<ActionResult, String> {
    ws::navigate_command_bar_next(comp).await
}
pub async fn navigate_command_bar_prev(
    comp: &mut DesktopComposition,
) -> Result<ActionResult, String> {
    ws::navigate_command_bar_prev(comp).await
}
pub async fn cancel_command_bar(comp: &mut DesktopComposition) -> Result<ActionResult, String> {
    ws::cancel_command_bar(comp).await
}
pub async fn confirm_selected_command(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
) -> Result<ActionResult, String> {
    ws::confirm_selected_command(comp, view, service, session_id, workspace_id).await
}
pub async fn execute_command_by_index(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    index: usize,
) -> Result<ActionResult, String> {
    ws::execute_command_by_index(comp, view, service, session_id, workspace_id, index).await
}
