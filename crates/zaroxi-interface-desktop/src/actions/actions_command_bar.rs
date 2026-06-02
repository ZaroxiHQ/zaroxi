/// Command-bar action handlers — thin desktop delegates to shared
/// orchestration in `zaroxi_application_workspace::workspace_view`.
/// AI commands (review/apply/reject) are desktop-specific because they
/// mutate `DesktopComposition.ai_projection`; the shared function returns
/// a "delegate" sentinel which is caught here.
use crate::desktop::DesktopComposition;
use std::sync::Arc;
use zaroxi_application_workspace::ports::{SessionId, WorkspaceView};
use zaroxi_application_workspace::workspace_view as ws;
use zaroxi_application_workspace::workspace_view::ActionResult;

use crate::desktop::composition::{
    apply_ai_edit_active, cancel_ai_edit_active, request_ai_edit_active,
};

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
    let res = ws::execute_command_by_index(
        comp,
        view.clone(),
        service.clone(),
        session_id.clone(),
        workspace_id,
        index,
    )
    .await?;

    if res.message.as_deref() == Some("delegate") {
        let label = ws::command_bar_labels().get(index).cloned().unwrap_or_default();
        return execute_ai_command(comp, view, service, session_id, &label).await;
    }

    Ok(res)
}

async fn execute_ai_command(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
    label: &str,
) -> Result<ActionResult, String> {
    match label {
        "AI review active buffer" => {
            request_ai_edit_active(comp, view, session_id, service).await?;
            Ok(ActionResult {
                success: true,
                message: Some("AI review requested".to_string()),
                refreshed: true,
            })
        }
        "Apply AI proposal" => {
            if let Some(svc) = service {
                apply_ai_edit_active(comp, view, session_id, None, svc).await?;
                Ok(ActionResult {
                    success: true,
                    message: Some("AI proposal applied".to_string()),
                    refreshed: true,
                })
            } else {
                Ok(ActionResult {
                    success: false,
                    message: Some("apply requires WorkspaceService".to_string()),
                    refreshed: false,
                })
            }
        }
        "Reject AI proposal" => {
            cancel_ai_edit_active(comp, service, Some(session_id));
            Ok(ActionResult {
                success: true,
                message: Some("AI proposal rejected".to_string()),
                refreshed: false,
            })
        }
        _ => Ok(ActionResult {
            success: false,
            message: Some(format!("unsupported AI command: {}", label)),
            refreshed: false,
        }),
    }
}
