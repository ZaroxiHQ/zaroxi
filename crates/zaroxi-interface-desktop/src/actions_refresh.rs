use std::sync::Arc;

use zaroxi_application_workspace::ports::{WorkspaceView, SessionId, WorkspaceService};
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
    comp: &mut DesktopComposition,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
) -> Result<ShellActionResult, String> {
    let action = refresh_desktop(comp, view, session_id.clone(), workspace_id, service).await?;
    let context = comp.latest_shell_context();
    Ok(ShellActionResult { action, context })
}
