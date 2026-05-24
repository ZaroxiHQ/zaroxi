/*!
Tiny action seam: refresh desktop composition.

Architectural rationale (Phase 14 - minimal desktop action flow):
- Provide a tiny, explicit action in the interface layer that composes existing
  seams (WorkspaceView, Presenter, DesktopComposition) to refresh the active
  desktop composition snapshot.
- Keep this strictly orchestration-only: do not duplicate any editor logic,
  do not modify application ports, and avoid introducing broader controller
  abstractions or event buses.
- The action delegates to DesktopComposition::refresh which already uses the
  Presenter + adapter seam (view_adapter) to obtain the renderable window.
- This lets external harnesses or potential future UI shells call a single
  intent-focused function to update presenter/composition state.

Public API:
- A tiny ActionResult returned by interface-facing actions:
    - `success`: true when action completed semantically
    - `message`: optional human-facing message (on failure or informative)
    - `refreshed`: whether the DesktopComposition was refreshed by this action

This file implements two tiny actions that return the normalized ActionResult.
*/

use std::sync::Arc;

use std::path::PathBuf;
use zaroxi_application_workspace::ports::{WorkspaceView, SessionId, WorkspaceService, GetActiveBufferRequest, SetEditorCursorRequest, ApplyTextTransactionRequest, EditorCursor, TextEdit, OpenBufferRequest, SaveCheckpointRequest};
use zaroxi_kernel_types::Id;

use crate::desktop::{DesktopComposition, RefreshReason};

/// Normalized, tiny action result returned by interface-desktop actions.
///
/// Purpose:
/// - Simple, shell-oriented status for UI actions.
/// - Avoid duplicating application/domain error types.
/// - Communicate whether a composition refresh occurred.
#[derive(Clone, Debug)]
pub struct ActionResult {
    pub success: bool,
    pub message: Option<String>,
    pub refreshed: bool,
}

/// Refresh the given DesktopComposition by delegating to its async `refresh` method.
///
/// Parameters:
/// - `comp`: mutable reference to an existing DesktopComposition instance (presenter state).
/// - `view`: an Arc'd WorkspaceView (application-provided).
/// - `session_id`: typed session id.
/// - `workspace_id`: optional workspace id recorded in the composition.
///
/// Returns an ActionResult wrapped in `Result` to allow mapping unexpected internal errors
/// (strings) while keeping the common success/failure represented by `ActionResult`.
///
/// Mapping policy:
/// - If `DesktopComposition::refresh` returns Ok(()) => success=true, refreshed=true
/// - If it returns Err(e) => success=false, message=Some(e), refreshed=false
pub async fn refresh_desktop(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
    service: Option<Arc<dyn WorkspaceService>>,
) -> Result<ActionResult, String> {
    // Delegate to the richer refresh variant which can optionally use a WorkspaceService
    // to populate the opened-buffer list for the shell. When `service` is None the
    // implementation falls back to the conservative projection.
    //
    // Important change:
    // - Do NOT preempt more specific shell-facing detections (like ActiveBufferChanged)
    //   when a WorkspaceService is provided. If a service is supplied, the composition
    //   can observe authoritative opened-buffer changes; setting a pending generic
    //   RefreshAction would incorrectly override those detections.
    // - Preserve previous behavior for the "no-service" path: when service is None
    //   we still mark the explicit action as RefreshAction so tests and harnesses
    //   that expect a generic refresh reason keep working.
    if !comp.has_pending_refresh_reason() {
        if service.is_none() {
            comp.set_pending_refresh_reason(RefreshReason::RefreshAction);
        }
    }

    match comp.refresh_with_service(view, session_id, workspace_id, service).await {
        Ok(()) => Ok(ActionResult { success: true, message: None, refreshed: true }),
        Err(e) => Ok(ActionResult { success: false, message: Some(e), refreshed: false }),
    }
}

/// Request to close the active tab (desktop-level). Behavior:
/// - If there is an active buffer, present the pending-close UI so the user can
///   choose Save / Discard / Cancel. This action sets DesktopComposition.pending_close.
///
/// Note:
/// - For simplicity this helper does not synchronously perform saves/closes; it
///   only sets the pending-close model so the UI can prompt the user. Resolution
///   helpers below perform the simple resolution semantics (clear or report failure).
pub async fn request_close_active(
    comp: &mut crate::desktop::DesktopComposition,
    _view: std::sync::Arc<dyn WorkspaceView>,
    _session_id: SessionId,
) -> Result<ActionResult, String> {
    if let Some(details) = comp.latest_active_buffer_details() {
        let pending = crate::desktop::PendingClose::BufferClose {
            buffer_id: details.buffer_id.clone(),
            display: details.display.clone(),
            // We do not have authoritative dirty info in the composition; be conservative.
            dirty: true,
        };
        comp.set_pending_close(pending);
        Ok(ActionResult { success: true, message: None, refreshed: false })
    } else {
        Ok(ActionResult { success: false, message: Some("no active buffer".to_string()), refreshed: false })
    }
}

/// Request to close the current session/window. Behavior:
/// - If the composition/service indicates the session can close immediately, perform a local
///   session close (UI-facing) and return success.
/// - If there are unsaved buffers, set a PendingClose::SessionClose so the UI can prompt
///   Save all / Discard all / Cancel. If a service is provided, prefer its `attempt_close_session` semantics.
pub async fn request_close_session(
    comp: &mut crate::desktop::DesktopComposition,
    _view: std::sync::Arc<dyn WorkspaceView>,
    session_id: SessionId,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
) -> Result<ActionResult, String> {
    // If service can determine close safety, ask it first.
    if let Some(s) = service {
        // We reuse the existing get_session_snapshot request shape as a light-weight attempt.
        let req = crate::ports::GetSessionSnapshotRequest { session_id: session_id.clone(), recent_limit: 0 };
        match s.attempt_close_session(req).await {
            Ok(snapshot) => {
                // Heuristic: if there are no buffer snapshots, proceed to close immediately.
                if snapshot.snapshot.opened_buffers.is_empty() {
                    comp.perform_session_close();
                    return Ok(ActionResult { success: true, message: None, refreshed: true });
                } else {
                    // Build a pending list of buffer ids to show in the UI; callers can resolve.
                    let dirty_ids = snapshot.snapshot.opened_buffers.clone();
                    let summary = format!("{} buffers may have unsaved changes", dirty_ids.len());
                    let pending = crate::desktop::PendingClose::SessionClose { dirty_buffers: dirty_ids, summary };
                    comp.set_pending_close(pending);
                    return Ok(ActionResult { success: true, message: None, refreshed: false });
                }
            }
            Err(_) => {
                // Fall back to conservative UI behavior below.
            }
        }
    }

    // No service or service failed to decide: use composition projection.
    let obs = comp.latest_opened_buffers_summary();
    if obs.count == 0 {
        // nothing open => safe to close
        comp.perform_session_close();
        Ok(ActionResult { success: true, message: None, refreshed: true })
    } else {
        // Conservatively assume there may be unsaved work and prompt the user.
        let ids: Vec<crate::ports::BufferId> = obs.items.iter().map(|i| i.buffer_id.clone()).collect();
        let summary = format!("{} open buffers", ids.len());
        let pending = crate::desktop::PendingClose::SessionClose { dirty_buffers: ids, summary };
        comp.set_pending_close(pending);
        Ok(ActionResult { success: true, message: None, refreshed: false })
    }
}

/// Confirm "Save all and close" for the currently pending session-close.
///
/// Behavior:
/// - If a WorkspaceService is provided we attempt to persist a checkpoint via save_checkpoint.
/// - On success the composition performs a local close; on failure the pending-close is converted
///   into a ResolutionFailure so the UI shows the error and keeps pending state.
pub async fn confirm_save_all_and_close(
    comp: &mut crate::desktop::DesktopComposition,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
    session_id: SessionId,
) -> Result<ActionResult, String> {
    // Try to persist via service if available.
    if let Some(s) = service {
        let save_req = SaveCheckpointRequest { session_id: session_id.clone() };
        match s.save_checkpoint(save_req).await {
            Ok(_) => {
                comp.clear_pending_close();
                comp.perform_session_close();
                comp.set_status_message("Saved and closed session".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            Err(e) => {
                // Keep pending state and surface resolution failure.
                comp.set_pending_close(crate::desktop::PendingClose::ResolutionFailure { message: format!("Save failed: {}", e) });
                return Ok(ActionResult { success: false, message: Some("save failed".to_string()), refreshed: false });
            }
        }
    } else {
        // No service: best-effort UI close (no durability), perform UI close immediately.
        comp.clear_pending_close();
        comp.perform_session_close();
        comp.set_status_message("Closed session (no service)".to_string());
        return Ok(ActionResult { success: true, message: None, refreshed: true });
    }
}

/// Confirm "Discard all and close" for the currently pending session-close.
///
/// Behavior:
/// - If a WorkspaceService is present we delegate discard semantics to the service (if implemented).
/// - Regardless, a successful discard performs a local composition close. On service-level failures
///   we present a ResolutionFailure.
pub async fn confirm_discard_all_and_close(
    comp: &mut crate::desktop::DesktopComposition,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
    session_id: SessionId,
) -> Result<ActionResult, String> {
    if let Some(s) = service {
        // The ports do not prescribe a discard-all call; many services may implement a lightweight
        // `resolve_close_session_discard_all` by delegating to internal persistence. Try and fall back.
        let req = crate::ports::SaveCheckpointRequest { session_id: session_id.clone() };
        match s.resolve_close_session_discard_all(req).await {
            Ok(_) => {
                comp.clear_pending_close();
                comp.perform_session_close();
                comp.set_status_message("Discarded and closed session".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            Err(e) => {
                comp.set_pending_close(crate::desktop::PendingClose::ResolutionFailure { message: format!("Discard failed: {}", e) });
                return Ok(ActionResult { success: false, message: Some("discard failed".to_string()), refreshed: false });
            }
        }
    } else {
        // No service: perform in-memory close (UI-level).
        comp.clear_pending_close();
        comp.perform_session_close();
        comp.set_status_message("Discarded and closed session (no service)".to_string());
        return Ok(ActionResult { success: true, message: None, refreshed: true });
    }
}

/// Confirm "Save and close" for the currently pending buffer-close.
///
/// Note:
/// - This interface-layer helper models the UI transition: set a short success
///   message and clear the pending-close UI. In a full integration the adapter
///   would call WorkspaceService to persist the buffer and then close it; if
///   that operation failed the adapter should surface PendingClose::ResolutionFailure
///   so the UI can present the error and keep the pending state.
pub async fn confirm_save_and_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    // If there is a pending buffer-close request, perform a UI-level removal of that buffer.
    if let Some(pc) = comp.latest_pending_close() {
        match pc {
            crate::desktop::PendingClose::BufferClose { buffer_id, .. } => {
                // Attempt to remove the buffer from the opened-buffer projection.
                let _removed = comp.close_opened_buffer(&buffer_id);
                // Clear pending overlay and set transient status message indicating success.
                comp.clear_pending_close();
                comp.set_status_message("Saved and closed".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            _ => {
                // Not a buffer-close (session close or other): fall back to previous behavior.
                comp.clear_pending_close();
                comp.set_status_message("Saved and closed".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
        }
    }

    // No pending close: behave as before (idempotent).
    comp.set_status_message("Saved and closed".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: true })
}

/// Confirm "Discard and close" for the currently pending buffer-close.
///
/// Note:
/// - This helper models the UI resolution. Real discard/close semantics should
///   be performed by an adapter calling into WorkspaceService; UI simply orchestrates.
pub async fn confirm_discard_and_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    // If there is a pending buffer-close, perform the UI-level buffer removal.
    if let Some(pc) = comp.latest_pending_close() {
        match pc {
            crate::desktop::PendingClose::BufferClose { buffer_id, .. } => {
                let _removed = comp.close_opened_buffer(&buffer_id);
                comp.clear_pending_close();
                comp.set_status_message("Discarded and closed".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            _ => {
                comp.clear_pending_close();
                comp.set_status_message("Discarded and closed".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
        }
    }

    // No pending close: behave as before.
    comp.set_status_message("Discarded and closed".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: true })
}

/// Cancel the pending-close flow and return to normal UI state.
pub async fn confirm_cancel_close(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    comp.clear_pending_close();
    comp.set_status_message("Close cancelled".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

/// Small shell action: move the editor cursor for the active buffer to the document start
/// (line 0, column 0) and refresh the desktop composition.
///
/// Behavior:
/// - Resolve active buffer via WorkspaceService::get_active_buffer
/// - Issue set_editor_cursor to move caret to (0,0)
/// - Refresh the DesktopComposition and return the ActionResult from refresh
///
/// Error handling:
/// - If get_active_buffer or set_editor_cursor return an error, return ActionResult with success=false
///   and the mapped message (stringified).
/// - The final result reflects whether the refresh completed (refreshed flag).
pub async fn move_cursor_to_start_and_refresh(
    comp: &mut crate::desktop::DesktopComposition,
    service: Arc<dyn WorkspaceService>,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
) -> Result<ActionResult, String> {
    // Resolve active buffer id from the service (explicit small use-case).
    let active_resp = match service.get_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() }).await {
        Ok(r) => r,
        Err(e) => return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false }),
    };

    let buffer_id = active_resp.buffer_id;

    // Issue set_editor_cursor to move caret to start (0,0).
    let set_req = SetEditorCursorRequest {
        session_id: session_id.clone(),
        buffer_id: buffer_id.clone(),
        cursor: EditorCursor { line: 0, column: 0 },
    };

    if let Err(e) = service.set_editor_cursor(set_req).await {
        return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false });
    }

    // Indicate why we are refreshing (cursor moved) so the composition records the reason.
    comp.set_pending_refresh_reason(RefreshReason::CursorMoved);

    // Refresh composition via existing tiny action (keeps responsibilities separated).
    // Reuse the normalized refresh_desktop so we return a consistent ActionResult.
    let refresh_result = refresh_desktop(comp, view, session_id, workspace_id, Some(service)).await?;
    Ok(refresh_result)
}

/// Small shell action: insert a blank line at the start of the active buffer
/// (line 0) and refresh the desktop composition.
///
/// Behavior:
/// - Resolve active buffer via WorkspaceService::get_active_buffer
/// - Apply a single character-indexed Insert transaction at index 0 using the
///   existing ApplyTextTransaction use-case (reuses application mutation pathway).
/// - Refresh the DesktopComposition and return the ActionResult from refresh.
///
/// Error handling:
/// - If get_active_buffer or apply_text_transaction return an error, return ActionResult with success=false
///   and the mapped message (stringified).
/// - The final result reflects whether the refresh completed (refreshed flag).
pub async fn insert_line_at_start_and_refresh(
    comp: &mut crate::desktop::DesktopComposition,
    service: Arc<dyn WorkspaceService>,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
) -> Result<ActionResult, String> {
    // Resolve active buffer id from the service (explicit small use-case).
    let active_resp = match service.get_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() }).await {
        Ok(r) => r,
        Err(e) => return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false }),
    };

    let buffer_id = active_resp.buffer_id;

    // Build and issue a typed Insert transaction at character index 0.
    let txn_req = ApplyTextTransactionRequest {
        session_id: session_id.clone(),
        buffer_id: buffer_id.clone(),
        transaction: TextEdit::Insert { index: 0, text: "\n".to_string() },
    };

    if let Err(e) = service.apply_text_transaction(txn_req).await {
        return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false });
    }

    // Indicate why we are refreshing (buffer updated) so the composition records the reason.
    comp.set_pending_refresh_reason(RefreshReason::BufferUpdated);

    // Refresh composition via existing tiny action and return its result.
    let refresh_result = refresh_desktop(comp, view, session_id, workspace_id, Some(service)).await?;
    Ok(refresh_result)
}

/// Convenience, tiny shell-facing result containing the normalized ActionResult
/// plus the latest ShellContext (when available).
#[derive(Clone, Debug)]
pub struct ShellActionResult {
    pub action: ActionResult,
    pub context: Option<crate::desktop::ShellContext>,
}

/// Tiny convenience action used by shells/harnesses:
/// - Reuse the existing refresh_desktop flow to update the DesktopComposition.
/// - Return both the normalized ActionResult and the latest ShellContext (if any).
///
/// This function intentionally delegates to refresh_desktop and then uses the
/// composition accessor `latest_shell_context()` so no refresh logic is duplicated.
pub async fn refresh_and_get_shell_context(
    comp: &mut crate::desktop::DesktopComposition,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
) -> Result<ShellActionResult, String> {
    // Perform the normalized refresh (reuses existing action semantics).
    let action = refresh_desktop(comp, view, session_id.clone(), workspace_id, service).await?;
    // Read the latest shell context from the composition (read-only accessor).
    let context = comp.latest_shell_context();
    Ok(ShellActionResult { action, context })
}

/// Tiny convenience shell action:
/// - Set the active buffer via the provided WorkspaceService.
/// - Mark the composition pending reason as ActiveBufferChanged.
/// - Refresh the DesktopComposition (using the service when present) and return the ShellActionResult.
///
/// Errors from the service are mapped into a failed ActionResult (returned inside Ok(ShellActionResult))
/// rather than being propagated as Err(String) — keeping parity with other small action semantics.
pub async fn set_active_buffer_and_get_shell_context(
    comp: &mut crate::desktop::DesktopComposition,
    service: std::sync::Arc<dyn WorkspaceService>,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    buffer_id: crate::ports::BufferId,
) -> Result<ShellActionResult, String> {
    // Try to read the current active buffer first. If the service reports the
    // requested buffer is already active we avoid calling set_active_buffer to
    // prevent redundant commands/events. If we cannot read the active buffer we
    // conservatively attempt to set it (preserve existing behavior).
    match service.get_active_buffer(crate::ports::GetActiveBufferRequest { session_id: session_id.clone() }).await {
        Ok(get_res) => {
            if get_res.buffer_id == buffer_id {
                // Already active: previously we always used a generic RefreshAction here to
                // avoid emitting duplicate ActiveBufferChanged events on a noop set. However,
                // there is a distinct and useful case where the underlying service already
                // reports the requested buffer as active but the composition has not yet
                // observed that change (stale composition metadata). In that situation the
                // action should prefer marking ActiveBufferChanged so the subsequent refresh
                // records the change for the shell.
                //
                // Decide:
                // - If the composition already thinks the requested buffer is active => noop -> RefreshAction.
                // - Otherwise the service state indicates a change occurred externally or via a prior command:
                //   mark ActiveBufferChanged so the refresh captures the authoritative change.
                let comp_active = comp.latest_metadata().and_then(|m| m.active_buffer.clone());
                if comp_active != Some(buffer_id.clone()) {
                    comp.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
                } else {
                    comp.set_pending_refresh_reason(RefreshReason::RefreshAction);
                }
            } else {
                // Different buffer: proceed to set active and mark ActiveBufferChanged.
                if let Err(e) = service.set_active_buffer(crate::ports::SetActiveBufferRequest { session_id: session_id.clone(), buffer_id: buffer_id.clone() }).await {
                    return Ok(ShellActionResult {
                        action: ActionResult { success: false, message: Some(e.to_string()), refreshed: false },
                        context: None,
                    });
                }
                comp.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
            }
        }
        Err(_e) => {
            // Could not determine current active buffer (e.g. UnknownSession). Fall back
            // to attempting to set the active buffer to preserve previous semantics.
            if let Err(e) = service.set_active_buffer(crate::ports::SetActiveBufferRequest { session_id: session_id.clone(), buffer_id: buffer_id.clone() }).await {
                return Ok(ShellActionResult {
                    action: ActionResult { success: false, message: Some(e.to_string()), refreshed: false },
                    context: None,
                });
            }
            comp.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
        }
    }

    // Delegate to the existing refresh_and_get_shell_context helper so we reuse projection/consistency logic.
    let res = refresh_and_get_shell_context(comp, view, session_id, workspace_id, Some(service)).await?;
    Ok(res)
}

/// Small command-bar actions exposed to shells/harnesses.
/// These reuse existing interface actions where possible and keep orchestration
/// inside the interface layer.
pub async fn open_command_bar(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    comp.open_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn close_command_bar(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    comp.close_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

/// Keyboard-oriented navigation: move selection to the next command.
///
/// This mirrors the existing `select_next_command` composition helper but exposes
/// it as a tiny async action so tests and harnesses may treat keyboard input as
/// a small action with normalized ActionResult.
pub async fn navigate_command_bar_next(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    comp.select_next_command();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

/// Keyboard-oriented navigation: move selection to the previous command.
pub async fn navigate_command_bar_prev(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    comp.select_prev_command();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

/// Confirm the currently-selected command in the open command bar.
///
/// Behavior:
/// - If no command bar is open, returns a failing ActionResult.
/// - Otherwise dispatches the selected command via existing `execute_command_by_index`.
/// - On successful command execution the command bar is closed (typical palette UX).
/// - The returned ActionResult is the normalized result from the underlying command.
pub async fn confirm_selected_command(
    comp: &mut crate::desktop::DesktopComposition,
    view: std::sync::Arc<dyn WorkspaceView>,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
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
    // Delegate to existing command executor. If it returns Ok and the action succeeded,
    // close the command bar to mirror palette UX.
    let res = execute_command_by_index(comp, view, service, session_id.clone(), workspace_id, idx).await?;
    if res.success {
        comp.close_command_bar();
    }
    Ok(res)
}

/// Cancel the open command bar (keyboard Escape).
pub async fn cancel_command_bar(
    comp: &mut crate::desktop::DesktopComposition,
) -> Result<ActionResult, String> {
    comp.close_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

/// Execute the command at `index` from the composition's current command bar list.
///
/// Behavior:
/// - Map short human labels to the existing tiny actions above (refresh, open buffer,
///   set active buffer, explain, request-close, confirm close variants).
/// - If a service is required by the selected command and none is provided, return a failed ActionResult.
pub async fn execute_command_by_index(
    comp: &mut crate::desktop::DesktopComposition,
    view: std::sync::Arc<dyn WorkspaceView>,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    index: usize,
) -> Result<ActionResult, String> {
    // Obtain command label
    let label: String = match comp.latest_command_bar().and_then(|cb| cb.commands.get(index).cloned()) {
        Some(l) => l,
        None => {
            return Ok(ActionResult { success: false, message: Some("no command at index".to_string()), refreshed: false })
        }
    };

    match label.as_str() {
        "Refresh" => {
            // reuse refresh action (service optional)
            let res = refresh_desktop(comp, view, session_id, workspace_id, service).await?;
            Ok(res)
        }
        "Open buffer" => {
            // deterministic fixture path for now
            if let Some(s) = service {
                let open_req = OpenBufferRequest { session_id: session_id.clone(), path: PathBuf::from("new_buffer.rs") };
                match s.open_buffer(open_req).await {
                    Ok(_) => {
                        comp.set_status_message("Opened buffer: new_buffer.rs".to_string());
                        // Trigger a refresh to make opened buffers visible to composition
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
                // Choose a deterministic target: first opened buffer if present.
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
                // Fire explain and update a tiny status message on success/failure.
                match s.explain_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() }).await {
                    Ok(resp) => {
                        comp.set_status_message(format!("Explain dispatched: {:?}", resp));
                        // refresh using service to allow AI projection to be picked up
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use zaroxi_application_workspace::ports::{
        WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, EditorDocument, EditorCursor,
    };
    use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
    use zaroxi_core_editor_buffer::ports::BufferId;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc as StdArc;

    /// Minimal in-test WorkspaceView stub that returns a tiny document and a prebuilt visible window.
    struct FakeView {
        doc: EditorDocument,
        window: VisibleLinesWindow,
    }

    impl FakeView {
        fn new() -> Self {
            // Build a simple document with one line "abcd" and cursor at col 2.
            let content = Some("abcd".to_string());
            let ed = EditorDocument {
                buffer_id: BufferId::from("buf:fake"),
                content: content.clone(),
                cursor: EditorCursor { line: 0, column: 2 },
                selection: None,
                line_count: 1,
                current_line: content.and_then(|c| c.lines().nth(0).map(|s| s.to_string())),
            };

            // Build a VisibleLinesWindow of one line.
            let vl = VisibleLine {
                line_number: 1,
                text: "abcd".to_string(),
                is_cursor_line: true,
                cursor_column: Some(2),
                selection_intersects: false,
                selection_start_column: None,
                selection_end_column: None,
            };
            let vw = VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };

            FakeView { doc: ed, window: vw }
        }
    }

    impl WorkspaceView for FakeView {
        fn get_buffer_content(&self, _buffer_id: crate::ports::BufferId) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
            Box::pin(async move { Ok(Some("".to_string())) })
        }

        fn get_active_buffer_content(&self, _session_id: crate::ports::SessionId) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
            Box::pin(async move { Ok(Some("".to_string())) })
        }

        fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetActiveEditorDocumentResponse, crate::ports::UseCaseError>> {
            let d = self.doc.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveEditorDocumentResponse { document: d }) })
        }

        fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetVisibleLinesResponse, crate::ports::UseCaseError>> {
            let w = self.window.clone();
            Box::pin(async move { Ok(crate::ports::GetVisibleLinesResponse { window: w }) })
        }
    }

    /// Minimal fake WorkspaceService implementing only the small methods we need for this test;
    /// other methods return standard errors. This keeps the test focused and avoids pulling
    /// in application orchestrator boot semantics.
    struct FakeService {
        buffer_id: BufferId,
        set_called: StdArc<AtomicBool>,
        apply_called: StdArc<AtomicBool>,
    }

    impl FakeService {
        fn new(buffer_id: BufferId) -> Self {
            Self { buffer_id, set_called: StdArc::new(AtomicBool::new(false)), apply_called: StdArc::new(AtomicBool::new(false)) }
        }
    }

    impl crate::ports::WorkspaceService for FakeService {
        fn boot_workspace(&self, _req: crate::ports::WorkspaceBootRequest) -> crate::BoxFuture<'static, Result<crate::ports::WorkspaceBootResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(&self, _req: crate::ports::OpenBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::OpenBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(&self, _req: crate::ports::ListBuffersRequest) -> crate::BoxFuture<'static, Result<crate::ports::ListBuffersResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_active_buffer(&self, _req: crate::ports::SetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>> {
            let bid = self.buffer_id.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveBufferResponse { buffer_id: bid }) })
        }

        fn set_editor_cursor(&self, req: crate::ports::SetEditorCursorRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>> {
            let expected = self.buffer_id.clone();
            let set_called = self.set_called.clone();
            Box::pin(async move {
                if req.buffer_id == expected && req.cursor.line == 0 && req.cursor.column == 0 {
                    set_called.store(true, Ordering::SeqCst);
                    Ok(crate::ports::SetEditorCursorResponse { ok: true })
                } else {
                    Err(crate::ports::UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()))
                }
            })
        }

        fn set_editor_selection(&self, _req: crate::ports::SetSelectionRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetSelectionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(&self, _req: crate::ports::ClearSelectionRequest) -> crate::BoxFuture<'static, Result<crate::ports::ClearSelectionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(&self, _req: crate::ports::GetEditorStateRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetEditorStateResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn set_viewport_state(&self, _req: crate::ports::SetViewportRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetViewportResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(&self, _req: crate::ports::ScrollViewportRequest) -> crate::BoxFuture<'static, Result<crate::ports::ScrollViewportResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(&self, _req: crate::ports::DispatchCommandRequest) -> crate::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(&self, _req: crate::ports::UpdateBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::UpdateBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(&self, _req: crate::ports::ApplyTextTransactionRequest) -> crate::BoxFuture<'static, Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>> {
            let apply_called = self.apply_called.clone();
            Box::pin(async move {
                apply_called.store(true, Ordering::SeqCst);
                Ok(crate::ports::ApplyTextTransactionResponse { ok: true, state: crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None }, content: None })
            })
        }

        fn get_recent_commands(&self, _req: crate::ports::GetRecentCommandsRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetRecentCommandsResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Ok(crate::ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }
        fn get_recent_events(&self, _req: crate::ports::GetRecentEventsRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetRecentEventsResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Ok(crate::ports::GetRecentEventsResponse { events: Vec::new() }) })
        }

        fn get_session_snapshot(&self, _req: crate::ports::GetSessionSnapshotRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetSessionSnapshotResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn create_checkpoint(&self, _req: crate::ports::CreateCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::CreateCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn save_checkpoint(&self, _req: crate::ports::SaveCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::SaveCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(&self, _req: crate::ports::LoadCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::LoadCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(&self, _req: crate::ports::RestoreCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::RestoreCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
    }

    #[tokio::test]
    async fn refresh_action_updates_composition() {
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let mut comp = crate::desktop::DesktopComposition::new();
        // Call the tiny action (no service available in this test)
        let ar = refresh_desktop(&mut comp, arc, sid.clone(), None, None).await.expect("refresh ok");
        assert!(ar.success);
        assert!(ar.refreshed);
        assert_eq!(comp.get_session_id().unwrap(), sid);
        let win = comp.latest_window().expect("window present");
        assert_eq!(win.total_lines, 1);
        assert_eq!(win.lines.len(), 1);

        // Composition should record a refresh reason for this explicit refresh action.
        let rr = comp.latest_refresh_reason().expect("reason present");
        assert_eq!(rr, RefreshReason::RefreshAction);

        // Status snapshot should be available for shell consumption.
        let status = comp.latest_status().expect("status present");
        assert!(status.has_render_window);
        assert!(status.has_metadata);
        assert!(status.has_opened_buffers);
        assert!(!status.has_ai_projection);
    }

    #[tokio::test]
    async fn move_cursor_action_moves_and_refreshes() {
        // Set up a fake view and fake service that cooperatively simulate a running orchestrator.
        let v = FakeView::new();
        let view_arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());

        // Fake service uses the same buffer id as the FakeView (buf:fake).
        let fake_service = FakeService::new(BufferId::from("buf:fake"));
        let service_arc: StdArc<dyn crate::ports::WorkspaceService> = StdArc::new(fake_service);

        let mut comp = crate::desktop::DesktopComposition::new();

        // First refresh to populate presenter state
        let _ = refresh_desktop(&mut comp, view_arc.clone(), sid.clone(), None, None).await.expect("initial refresh ok");

        // Execute the move-cursor action which should call set_editor_cursor on the service
        // and then refresh the composition again.
        let res = move_cursor_to_start_and_refresh(&mut comp, service_arc.clone(), view_arc.clone(), sid.clone(), None).await;
        assert!(res.is_ok(), "move cursor action should succeed");
        let ar = res.unwrap();
        assert!(ar.success);
        assert!(ar.refreshed);

        // There is no direct observable cursor state on the composition beyond refresh success,
        // but success indicates the orchestration path executed (get_active_buffer -> set_editor_cursor -> refresh).
    }

    #[tokio::test]
    async fn insert_line_action_inserts_and_refreshes() {
        // Set up a fake view and fake service that cooperatively simulate a running orchestrator.
        let v = FakeView::new();
        let view_arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());

        // Fake service uses the same buffer id as the FakeView (buf:fake).
        let fake_service = FakeService::new(BufferId::from("buf:fake"));
        let service_arc: StdArc<dyn crate::ports::WorkspaceService> = StdArc::new(fake_service);

        let mut comp = crate::desktop::DesktopComposition::new();

        // First refresh to populate presenter state
        let _ = refresh_desktop(&mut comp, view_arc.clone(), sid.clone(), None, None).await.expect("initial refresh ok");

        // Execute the insert-line action which should call apply_text_transaction on the service
        // and then refresh the composition again.
        let res = insert_line_at_start_and_refresh(&mut comp, service_arc.clone(), view_arc.clone(), sid.clone(), None).await;
        assert!(res.is_ok(), "insert-line action should succeed");
        let ar = res.unwrap();
        assert!(ar.success);
        assert!(ar.refreshed);
    }

    #[tokio::test]
    async fn set_active_buffer_detects_external_change() {
        // Scenario:
        // - Composition has not been refreshed (no metadata).
        // - WorkspaceService reports the requested buffer is already active (external change).
        // Expected:
        // - The convenience action should mark ActiveBufferChanged so the upcoming refresh
        //   records the authoritative active-buffer transition for the shell.
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let mut comp = crate::desktop::DesktopComposition::new();

        // Fake service reports buf:two as the currently active buffer.
        let fake_service = std::sync::Arc::new(FakeService::new(BufferId::from("buf:two"))) as std::sync::Arc<dyn crate::ports::WorkspaceService>;

        let res = set_active_buffer_and_get_shell_context(&mut comp, fake_service.clone(), arc.clone(), sid.clone(), None, BufferId::from("buf:two")).await.expect("action ok");
        assert!(res.action.success);

        let rr = comp.latest_refresh_reason().expect("reason present");
        assert_eq!(rr, crate::desktop::RefreshReason::ActiveBufferChanged);
    }
}
