use std::sync::Arc;
use zaroxi_application_workspace::ports::{SaveCheckpointRequest, SessionId, WorkspaceView};

use super::actions_refresh::ActionResult;

/// Request to close the active tab (desktop-level). Behavior:
/// - If there is an active buffer, present the pending-close UI so the user can
///   choose Save / Discard / Cancel. This action sets DesktopComposition.pending_close.
pub async fn request_close_active(
    comp: &mut crate::desktop::DesktopComposition,
    _view: Arc<dyn WorkspaceView>,
    _session_id: SessionId,
) -> Result<ActionResult, String> {
    if let Some(details) = comp.latest_active_buffer_details() {
        let pending = crate::desktop::PendingClose::BufferClose {
            buffer_id: details.buffer_id.clone(),
            display: details.display.clone(),
            dirty: true,
        };
        comp.set_pending_close(pending);
        Ok(ActionResult { success: true, message: None, refreshed: false })
    } else {
        Ok(ActionResult {
            success: false,
            message: Some("no active buffer".to_string()),
            refreshed: false,
        })
    }
}

/// Request to close the current session/window. Behavior:
pub async fn request_close_session(
    comp: &mut crate::desktop::DesktopComposition,
    _view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
) -> Result<ActionResult, String> {
    if let Some(s) = service {
        let req = crate::ports::GetSessionSnapshotRequest {
            session_id: session_id.clone(),
            recent_limit: 0,
        };
        match s.attempt_close_session(req).await {
            Ok(snapshot) => {
                if snapshot.snapshot.opened_buffers.is_empty() {
                    comp.perform_session_close();
                    return Ok(ActionResult { success: true, message: None, refreshed: true });
                } else {
                    let dirty_ids = snapshot.snapshot.opened_buffers.clone();
                    let summary = format!("{} buffers may have unsaved changes", dirty_ids.len());
                    let pending = crate::desktop::PendingClose::SessionClose {
                        dirty_buffers: dirty_ids,
                        summary,
                    };
                    comp.set_pending_close(pending);
                    return Ok(ActionResult { success: true, message: None, refreshed: false });
                }
            }
            Err(_) => {}
        }
    }

    let obs = comp.latest_opened_buffers_summary();
    if obs.count == 0 {
        comp.perform_session_close();
        Ok(ActionResult { success: true, message: None, refreshed: true })
    } else {
        let ids: Vec<crate::ports::BufferId> =
            obs.items.iter().map(|i| i.buffer_id.clone()).collect();
        let summary = format!("{} open buffers", ids.len());
        let pending = crate::desktop::PendingClose::SessionClose { dirty_buffers: ids, summary };
        comp.set_pending_close(pending);
        Ok(ActionResult { success: true, message: None, refreshed: false })
    }
}

/// Confirm "Save all and close" for the currently pending session-close.
pub async fn confirm_save_all_and_close(
    comp: &mut crate::desktop::DesktopComposition,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
) -> Result<ActionResult, String> {
    if let Some(s) = service {
        let save_req = SaveCheckpointRequest { session_id: session_id.clone() };
        match s.save_checkpoint(save_req).await {
            Ok(_) => {
                // perform_session_close clears composition metadata/presenter; set a coherent
                // final status afterwards so harnesses and status helpers report the same text.
                comp.perform_session_close();
                comp.set_close_result_status("Saved and closed session".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            Err(e) => {
                comp.set_pending_close(crate::desktop::PendingClose::ResolutionFailure {
                    message: format!("Save failed: {}", e),
                });
                return Ok(ActionResult {
                    success: false,
                    message: Some("save failed".to_string()),
                    refreshed: false,
                });
            }
        }
    } else {
        comp.perform_session_close();
        comp.set_close_result_status("Closed session (no service)".to_string());
        return Ok(ActionResult { success: true, message: None, refreshed: true });
    }
}

/// Confirm "Discard all and close" for the currently pending session-close.
pub async fn confirm_discard_all_and_close(
    comp: &mut crate::desktop::DesktopComposition,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    session_id: SessionId,
) -> Result<ActionResult, String> {
    if let Some(s) = service {
        let req = crate::ports::SaveCheckpointRequest { session_id: session_id.clone() };
        match s.resolve_close_session_discard_all(req).await {
            Ok(_) => {
                comp.perform_session_close();
                comp.set_close_result_status("Discarded and closed session".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            Err(e) => {
                comp.set_pending_close(crate::desktop::PendingClose::ResolutionFailure {
                    message: format!("Discard failed: {}", e),
                });
                return Ok(ActionResult {
                    success: false,
                    message: Some("discard failed".to_string()),
                    refreshed: false,
                });
            }
        }
    } else {
        comp.clear_pending_close();
        comp.perform_session_close();
        comp.set_status_message("Discarded and closed session (no service)".to_string());
        return Ok(ActionResult { success: true, message: None, refreshed: true });
    }
}

/// Confirm "Save and close" for the currently pending buffer-close.
pub async fn confirm_save_and_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    if let Some(pc) = comp.latest_pending_close() {
        match pc {
            crate::desktop::PendingClose::BufferClose { buffer_id, display, .. } => {
                // Prefer a human-friendly display label when available, otherwise fall back to
                // buffer path or buffer id string. This makes the status message explicit.
                let label = display
                    .clone()
                    .or_else(|| buffer_id.path().map(|p| p.to_string_lossy().to_string()))
                    .unwrap_or_else(|| buffer_id.to_string());

                // Remove from opened buffers and set a final close-result status (this clears pending state).
                let _removed = comp.close_opened_buffer(&buffer_id);
                let id_str = format!("{}", buffer_id);
                comp.set_close_result_status(format!("Saved and closed {} ({})", label, id_str));
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            _ => {
                comp.set_close_result_status("Saved and closed".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
        }
    }

    comp.set_status_message("Saved and closed".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: true })
}

/// Confirm "Discard and close" for the currently pending buffer-close.
pub async fn confirm_discard_and_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    if let Some(pc) = comp.latest_pending_close() {
        match pc {
            crate::desktop::PendingClose::BufferClose { buffer_id, display, .. } => {
                // Prefer human-friendly display when available, otherwise fall back to path or id.
                let label = display
                    .clone()
                    .or_else(|| buffer_id.path().map(|p| p.to_string_lossy().to_string()))
                    .unwrap_or_else(|| buffer_id.to_string());

                // Remove from opened buffers and set a final close-result status (this clears pending state).
                let _removed = comp.close_opened_buffer(&buffer_id);
                let id_str = format!("{}", buffer_id);
                comp.set_close_result_status(format!(
                    "Discarded changes and closed {} ({})",
                    label, id_str
                ));
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            _ => {
                comp.set_close_result_status("Discarded and closed".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
        }
    }

    comp.set_status_message("Discarded and closed".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: true })
}

/// Cancel the pending-close flow and return to normal UI state.
pub async fn confirm_cancel_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    // Ensure any previously preserved explicit close-result status is cleared
    // so a cancelled close does not accidentally display a stale "Saved/Discarded"
    // message.
    comp.clear_close_result_status();
    comp.clear_pending_close();
    comp.set_status_message("Close cancelled".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: false })
}
