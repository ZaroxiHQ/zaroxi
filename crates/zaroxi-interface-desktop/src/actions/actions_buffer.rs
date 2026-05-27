use crate::desktop::RefreshReason;
use zaroxi_application_workspace::ports::{SessionId, WorkspaceView};

use super::actions_refresh::{ActionResult, refresh_and_get_shell_context};

/// Tiny convenience shell action:
/// - Set the active buffer via the provided WorkspaceService.
/// - Mark the composition pending reason as ActiveBufferChanged.
/// - Refresh the DesktopComposition (using the service when present) and return the ShellActionResult.
pub async fn set_active_buffer_and_get_shell_context(
    comp: &mut crate::desktop::DesktopComposition,
    service: std::sync::Arc<dyn crate::ports::WorkspaceService>,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    buffer_id: crate::ports::BufferId,
) -> Result<super::actions_refresh::ShellActionResult, String> {
    match service
        .get_active_buffer(crate::ports::GetActiveBufferRequest { session_id: session_id.clone() })
        .await
    {
        Ok(get_res) => {
            if get_res.buffer_id == buffer_id {
                let comp_active = comp.latest_metadata().and_then(|m| m.active_buffer.clone());
                if comp_active != Some(buffer_id.clone()) {
                    comp.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
                } else {
                    comp.set_pending_refresh_reason(RefreshReason::RefreshAction);
                }
            } else {
                if let Err(e) = service
                    .set_active_buffer(crate::ports::SetActiveBufferRequest {
                        session_id: session_id.clone(),
                        buffer_id: buffer_id.clone(),
                    })
                    .await
                {
                    return Ok(super::actions_refresh::ShellActionResult {
                        action: ActionResult {
                            success: false,
                            message: Some(e.to_string()),
                            refreshed: false,
                        },
                        context: None,
                    });
                }
                comp.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
            }
        }
        Err(_e) => {
            if let Err(e) = service
                .set_active_buffer(crate::ports::SetActiveBufferRequest {
                    session_id: session_id.clone(),
                    buffer_id: buffer_id.clone(),
                })
                .await
            {
                return Ok(super::actions_refresh::ShellActionResult {
                    action: ActionResult {
                        success: false,
                        message: Some(e.to_string()),
                        refreshed: false,
                    },
                    context: None,
                });
            }
            comp.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
        }
    }

    let res =
        refresh_and_get_shell_context(comp, view, session_id, workspace_id, Some(service)).await?;
    Ok(res)
}
