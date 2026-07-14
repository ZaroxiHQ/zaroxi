/// Command-bar action handlers — thin desktop delegates to shared
/// orchestration in `zaroxi_application_workspace::workspace_view`.
/// AI commands (review/apply/reject) are desktop-specific because they
/// mutate `DesktopComposition.ai_projection`; the shared function returns
/// a "delegate" sentinel which is caught here.
/// "Open workspace by path" is also a delegate because it needs a
/// folder picker and mutable session/workspace state.
use crate::desktop::DesktopComposition;
use std::path::PathBuf;
use std::sync::Arc;
use zaroxi_application_workspace::ports::{SessionId, WorkspaceBootRequest, WorkspaceView};
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
        if label == "Open workspace by path" {
            return execute_open_workspace_by_path(comp, service, session_id).await;
        }
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
        "Explain selection" | "Refactor selection" | "Generate tests" | "Fix diagnostics" => {
            // Phase 2: these actions use the existing AI review pipeline.
            // Future phases will add action-specific prompt routing via ActionService.
            request_ai_edit_active(comp, view, session_id, service).await?;
            Ok(ActionResult {
                success: true,
                message: Some(format!("{label} requested")),
                refreshed: true,
            })
        }
        _ => Ok(ActionResult {
            success: false,
            message: Some(format!("unsupported AI command: {}", label)),
            refreshed: false,
        }),
    }
}

async fn execute_open_workspace_by_path(
    comp: &mut DesktopComposition,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    _session_id: SessionId,
) -> Result<ActionResult, String> {
    let path = match std::env::var("ZAROXI_WORKSPACE_PATH") {
        Ok(raw) if !raw.is_empty() => PathBuf::from(raw),
        _ => {
            let msg = concat!(
                "ZAROXI_WORKSPACE_PATH env var not set. ",
                "Set it to a workspace directory path, or use the Open Workspace button."
            );
            comp.set_status_message(msg.to_string());
            return Ok(ActionResult {
                success: false,
                message: Some(msg.to_string()),
                refreshed: true,
            });
        }
    };

    if !path.is_dir() {
        let msg =
            format!("Workspace path does not exist or is not a directory: {}", path.display());
        comp.set_status_message(msg.clone());
        return Ok(ActionResult { success: false, message: Some(msg), refreshed: true });
    }

    let service = match service {
        Some(s) => s,
        None => {
            let msg = "Open workspace requires WorkspaceService".to_string();
            comp.set_status_message(msg.clone());
            return Ok(ActionResult { success: false, message: Some(msg), refreshed: false });
        }
    };

    let boot_req = WorkspaceBootRequest { path: path.clone() };
    match service.boot_workspace(boot_req).await {
        Ok(boot_res) => {
            comp.session_id = Some(boot_res.session.session_id);
            comp.workspace_id = Some(boot_res.session.workspace_id);
            comp.workspace_root_path = Some(path.clone());
            comp.load_or_refresh_explorer();

            let msg = format!("Workspace opened: {}", path.display());
            comp.set_status_message(msg.clone());

            Ok(ActionResult { success: true, message: Some(msg), refreshed: true })
        }
        Err(e) => {
            let msg = format!("Failed to boot workspace: {}", e);
            comp.set_status_message(msg.clone());
            Ok(ActionResult { success: false, message: Some(msg), refreshed: true })
        }
    }
}
