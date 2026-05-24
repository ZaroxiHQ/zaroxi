use std::sync::Arc;
use zaroxi_application_workspace::ports::{WorkspaceView, SessionId, GetActiveBufferRequest, SetEditorCursorRequest, ApplyTextTransactionRequest, EditorCursor, TextEdit};
use zaroxi_kernel_types::Id;
use crate::desktop::RefreshReason;

use super::actions_refresh::{ActionResult, refresh_desktop};

/// Small shell action: move the editor cursor for the active buffer to the document start
/// (line 0, column 0) and refresh the desktop composition.
pub async fn move_cursor_to_start_and_refresh(
    comp: &mut crate::desktop::DesktopComposition,
    service: Arc<dyn crate::ports::WorkspaceService>,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
) -> Result<ActionResult, String> {
    let active_resp = match service.get_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() }).await {
        Ok(r) => r,
        Err(e) => return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false }),
    };

    let buffer_id = active_resp.buffer_id;

    let set_req = SetEditorCursorRequest {
        session_id: session_id.clone(),
        buffer_id: buffer_id.clone(),
        cursor: EditorCursor { line: 0, column: 0 },
    };

    if let Err(e) = service.set_editor_cursor(set_req).await {
        return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false });
    }

    comp.set_pending_refresh_reason(RefreshReason::CursorMoved);

    let refresh_result = refresh_desktop(comp, view, session_id, workspace_id, Some(service)).await?;
    Ok(refresh_result)
}

/// Small shell action: insert a blank line at the start of the active buffer
/// (line 0) and refresh the desktop composition.
pub async fn insert_line_at_start_and_refresh(
    comp: &mut crate::desktop::DesktopComposition,
    service: Arc<dyn crate::ports::WorkspaceService>,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
) -> Result<ActionResult, String> {
    let active_resp = match service.get_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() }).await {
        Ok(r) => r,
        Err(e) => return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false }),
    };

    let buffer_id = active_resp.buffer_id;

    let txn_req = ApplyTextTransactionRequest {
        session_id: session_id.clone(),
        buffer_id: buffer_id.clone(),
        transaction: TextEdit::Insert { index: 0, text: "\n".to_string() },
    };

    if let Err(e) = service.apply_text_transaction(txn_req).await {
        return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false });
    }

    comp.set_pending_refresh_reason(RefreshReason::BufferUpdated);

    let refresh_result = refresh_desktop(comp, view, session_id, workspace_id, Some(service)).await?;
    Ok(refresh_result)
}
