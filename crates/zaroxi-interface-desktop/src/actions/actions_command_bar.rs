/// Command-bar action handlers — thin desktop delegates to shared
/// orchestration in `zaroxi_application_workspace::workspace_view`.
///
/// Service-heavy commands (Refresh, Open buffer, Set active, Explain)
/// are caught here after the shared function returns `"delegate"`.
use crate::desktop::DesktopComposition;
use std::path::PathBuf;
use std::sync::Arc;
use zaroxi_application_workspace::ports::{
    GetActiveBufferRequest, OpenBufferRequest, SessionId, WorkspaceView,
};
use zaroxi_application_workspace::workspace_view as ws;
use zaroxi_application_workspace::workspace_view::ActionResult;

use super::actions_buffer::set_active_buffer_and_get_shell_context;
use super::actions_refresh::refresh_desktop;

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
    ws::confirm_selected_command(comp, view, service, session_id.clone(), workspace_id).await
}

/// Dispatch then catch service-heavy commands the shared function delegates back.
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
        return execute_delegated(comp, view, service, session_id, workspace_id, &label).await;
    }

    Ok(res)
}

async fn execute_delegated(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    label: &str,
) -> Result<ActionResult, String> {
    match label {
        "Refresh" => refresh_desktop(comp, view, session_id, workspace_id, service).await,
        "Open buffer" => {
            if let Some(s) = service {
                let open_req = OpenBufferRequest {
                    session_id: session_id.clone(),
                    path: PathBuf::from("new_buffer.rs"),
                };
                match s.open_buffer(open_req).await {
                    Ok(_) => {
                        comp.set_status_message("Opened buffer: new_buffer.rs".to_string());
                        let _res =
                            refresh_desktop(comp, view, session_id, workspace_id, Some(s)).await?;
                        Ok(ActionResult {
                            success: true,
                            message: Some("opened buffer".to_string()),
                            refreshed: true,
                        })
                    }
                    Err(e) => Ok(ActionResult {
                        success: false,
                        message: Some(e.to_string()),
                        refreshed: false,
                    }),
                }
            } else {
                Ok(ActionResult {
                    success: false,
                    message: Some("open-buffer requires WorkspaceService".to_string()),
                    refreshed: false,
                })
            }
        }
        "Set active buffer" => {
            if let Some(s) = service {
                let obs = comp.latest_opened_buffers_summary();
                if let Some(item) = obs.items.get(0) {
                    let buf = item.buffer_id.clone();
                    let res = set_active_buffer_and_get_shell_context(
                        comp,
                        s,
                        view,
                        session_id,
                        workspace_id,
                        buf,
                    )
                    .await?;
                    Ok(res.action)
                } else {
                    Ok(ActionResult {
                        success: false,
                        message: Some("no opened buffers to activate".to_string()),
                        refreshed: false,
                    })
                }
            } else {
                Ok(ActionResult {
                    success: false,
                    message: Some("set-active requires WorkspaceService".to_string()),
                    refreshed: false,
                })
            }
        }
        "Explain active buffer" => {
            if let Some(s) = service {
                match s
                    .explain_active_buffer(GetActiveBufferRequest {
                        session_id: session_id.clone(),
                    })
                    .await
                {
                    Ok(resp) => {
                        comp.set_status_message(format!("Explain dispatched: {:?}", resp));
                        let _ =
                            refresh_desktop(comp, view, session_id, workspace_id, Some(s)).await?;
                        Ok(ActionResult {
                            success: true,
                            message: Some("explain dispatched".to_string()),
                            refreshed: true,
                        })
                    }
                    Err(e) => Ok(ActionResult {
                        success: false,
                        message: Some(e.to_string()),
                        refreshed: false,
                    }),
                }
            } else {
                Ok(ActionResult {
                    success: false,
                    message: Some("explain requires WorkspaceService".to_string()),
                    refreshed: false,
                })
            }
        }
        _ => Ok(ActionResult {
            success: false,
            message: Some(format!("unsupported command: {}", label)),
            refreshed: false,
        }),
    }
}
