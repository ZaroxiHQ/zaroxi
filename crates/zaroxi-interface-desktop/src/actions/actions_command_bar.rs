// Focused command-bar helper implementations (split from former large actions.rs).

use std::sync::Arc;
use std::path::PathBuf;
use zaroxi_application_workspace::ports::{WorkspaceView, SessionId, OpenBufferRequest, GetActiveBufferRequest};
use crate::desktop::{DesktopComposition};

use super::actions_refresh::{ActionResult, refresh_desktop};
use super::actions_buffer::set_active_buffer_and_get_shell_context;
use super::actions_close_flow::{request_close_active, confirm_save_and_close, confirm_discard_and_close, confirm_cancel_close};

/// Open the command bar with a deterministic set of commands.
pub async fn open_command_bar(
    comp: &mut DesktopComposition,
) -> Result<ActionResult, String> {
    comp.open_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn close_command_bar(
    comp: &mut DesktopComposition,
) -> Result<ActionResult, String> {
    comp.close_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn navigate_command_bar_next(
    comp: &mut DesktopComposition,
) -> Result<ActionResult, String> {
    comp.select_next_command();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn navigate_command_bar_prev(
    comp: &mut DesktopComposition,
) -> Result<ActionResult, String> {
    comp.select_prev_command();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn confirm_selected_command(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
) -> Result<ActionResult, String> {
    let cb = match comp.latest_command_bar() {
        Some(cb) => cb,
        None => {
            return Ok(ActionResult {
                success: false,
                message: Some("command bar is not open".to_string()),
                refreshed: false,
            })
        }
    };

    let idx = cb.selected;
    let res = execute_command_by_index(comp, view, service, session_id.clone(), workspace_id, idx).await?;
    if res.success {
        comp.close_command_bar();
    }
    Ok(res)
}

pub async fn cancel_command_bar(
    comp: &mut DesktopComposition,
) -> Result<ActionResult, String> {
    comp.close_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn execute_command_by_index(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    index: usize,
) -> Result<ActionResult, String> {
    let label: String = match comp.latest_command_bar().and_then(|cb| cb.commands.get(index).cloned()) {
        Some(l) => l,
        None => {
            return Ok(ActionResult { success: false, message: Some("no command at index".to_string()), refreshed: false })
        }
    };

    match label.as_str() {
        "Refresh" => {
            let res = refresh_desktop(comp, view, session_id, workspace_id, service).await?;
            Ok(res)
        }
        "Open buffer" => {
            if let Some(s) = service {
                let open_req = OpenBufferRequest { session_id: session_id.clone(), path: PathBuf::from("new_buffer.rs") };
                match s.open_buffer(open_req).await {
                    Ok(_) => {
                        comp.set_status_message("Opened buffer: new_buffer.rs".to_string());
                        let _ = refresh_desktop(comp, view, session_id, workspace_id, Some(s)).await?;
                        Ok(ActionResult { success: true, message: Some("opened buffer".to_string()), refreshed: true })
                    }
                    Err(e) => Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false }),
                }
            } else {
                Ok(ActionResult { success: false, message: Some("open-buffer requires WorkspaceService".to_string()), refreshed: false })
            }
        }
        "Set active buffer" => {
            if let Some(s) = service {
                let obs = comp.latest_opened_buffers_summary();
                if let Some(item) = obs.items.get(0) {
                    let buf = item.buffer_id.clone();
                    let res = set_active_buffer_and_get_shell_context(comp, s, view, session_id, workspace_id, buf).await?;
                    Ok(res.action)
                } else {
                    Ok(ActionResult { success: false, message: Some("no opened buffers to activate".to_string()), refreshed: false })
                }
            } else {
                Ok(ActionResult { success: false, message: Some("set-active requires WorkspaceService".to_string()), refreshed: false })
            }
        }
        "Explain active buffer" => {
            if let Some(s) = service {
                match s.explain_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() }).await {
                    Ok(resp) => {
                        comp.set_status_message(format!("Explain dispatched: {:?}", resp));
                        let _ = refresh_desktop(comp, view, session_id, workspace_id, Some(s)).await?;
                        Ok(ActionResult { success: true, message: Some("explain dispatched".to_string()), refreshed: true })
                    }
                    Err(e) => Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false }),
                }
            } else {
                Ok(ActionResult { success: false, message: Some("explain requires WorkspaceService".to_string()), refreshed: false })
            }
        }
        "Request close active" => {
            let ar = request_close_active(comp, view, session_id).await?;
            Ok(ar)
        }
        "Confirm close: save" => {
            let ar = confirm_save_and_close(comp).await?;
            Ok(ar)
        }
        "Confirm close: discard" => {
            let ar = confirm_discard_and_close(comp).await?;
            Ok(ar)
        }
        "Confirm close: cancel" => {
            let ar = confirm_cancel_close(comp).await?;
            Ok(ar)
        }
        _ => Ok(ActionResult { success: false, message: Some(format!("unsupported command: {}", label)), refreshed: false }),
    }
}
