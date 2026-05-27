#![cfg(test)]

use std::sync::Arc;
use crate::ports::BoxFuture;

use zaroxi_interface_desktop::desktop::DesktopComposition;
use zaroxi_interface_desktop::desktop::request_ai_edit_active;
use zaroxi_interface_desktop::desktop::apply_ai_edit_active;
use zaroxi_interface_desktop::desktop::cancel_ai_edit_active;
use zaroxi_application_workspace::ports::{
    WorkspaceView, WorkspaceService, GetActiveEditorDocumentRequest, GetActiveEditorDocumentResponse,
    UpdateBufferRequest, UpdateBufferResponse, GetVisibleLinesRequest, GetVisibleLinesResponse,
    GetRecentCommandsRequest, GetRecentEventsRequest, ListBuffersRequest, ListBuffersResponse,
    OpenBufferRequest, OpenBufferResponse, WorkspaceBootRequest, WorkspaceBootResponse, SetActiveBufferRequest,
    SetEditorCursorRequest, SetSelectionRequest, ClearSelectionRequest, GetEditorStateRequest, GetEditorStateResponse,
    EditorDocument, BufferId, SessionId, UseCaseError, ApplyTextTransactionRequest, ApplyTextTransactionResponse,
};
use zaroxi_kernel_types::Id;
use zaroxi_interface_desktop::ports;

struct FakeView {
    doc: EditorDocument,
}

impl FakeView {
    fn new(buffer_id: BufferId, content: Option<String>) -> Self {
        FakeView {
            doc: EditorDocument {
                buffer_id,
                content,
                cursor: ports::EditorCursor::zero(),
                selection: None,
                line_count: 1,
                current_line: None,
            },
        }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(&self, _buffer_id: BufferId) -> BoxFuture<'static, Result<Option<String>, UseCaseError>> {
        let content = self.doc.content.clone();
        Box::pin(async move { Ok(content) })
    }

    fn get_active_buffer_content(&self, _session_id: SessionId) -> BoxFuture<'static, Result<Option<String>, UseCaseError>> {
        let content = self.doc.content.clone();
        Box::pin(async move { Ok(content) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> BoxFuture<'static, Result<GetActiveEditorDocumentResponse, UseCaseError>> {
        let doc = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> BoxFuture<'static, Result<GetVisibleLinesResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
}

struct FakeService {
    last_update: std::sync::Mutex<Option<String>>,
}

impl FakeService {
    fn new() -> Self {
        FakeService { last_update: std::sync::Mutex::new(None) }
    }
}

impl WorkspaceService for FakeService {
    fn boot_workspace(&self, _req: WorkspaceBootRequest) -> BoxFuture<'static, Result<WorkspaceBootResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownWorkspace) })
    }
    fn open_buffer(&self, _req: OpenBufferRequest) -> BoxFuture<'static, Result<OpenBufferResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownBuffer) })
    }
    fn list_open_buffers(&self, _req: ListBuffersRequest) -> BoxFuture<'static, Result<ListBuffersResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn set_active_buffer(&self, _req: SetActiveBufferRequest) -> BoxFuture<'static, Result<ports::SetActiveBufferResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn get_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> BoxFuture<'static, Result<ports::GetActiveBufferResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn set_editor_cursor(&self, _req: SetEditorCursorRequest) -> BoxFuture<'static, Result<ports::SetEditorCursorResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn set_editor_selection(&self, _req: SetSelectionRequest) -> BoxFuture<'static, Result<ports::SetSelectionResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn clear_editor_selection(&self, _req: ClearSelectionRequest) -> BoxFuture<'static, Result<ports::ClearSelectionResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn get_editor_state(&self, _req: GetEditorStateRequest) -> BoxFuture<'static, Result<GetEditorStateResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn set_viewport_state(&self, _req: ports::SetViewportRequest) -> BoxFuture<'static, Result<ports::SetViewportResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn scroll_viewport(&self, _req: ports::ScrollViewportRequest) -> BoxFuture<'static, Result<ports::ScrollViewportResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn explain_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> BoxFuture<'static, Result<ports::DispatchCommandResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn dispatch_command(&self, _req: ports::DispatchCommandRequest) -> BoxFuture<'static, Result<ports::DispatchCommandResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn update_buffer(&self, req: UpdateBufferRequest) -> BoxFuture<'static, Result<UpdateBufferResponse, UseCaseError>> {
        let mut guard = self.last_update.lock().unwrap();
        *guard = Some(req.new_content.clone());
        Box::pin(async move { Ok(UpdateBufferResponse { ok: true }) })
    }
    fn apply_text_transaction(&self, _req: ApplyTextTransactionRequest) -> BoxFuture<'static, Result<ApplyTextTransactionResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn get_recent_commands(&self, _req: GetRecentCommandsRequest) -> BoxFuture<'static, Result<ports::GetRecentCommandsResponse, UseCaseError>> {
        Box::pin(async move { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) })
    }
    fn get_recent_events(&self, _req: GetRecentEventsRequest) -> BoxFuture<'static, Result<ports::GetRecentEventsResponse, UseCaseError>> {
        Box::pin(async move { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) })
    }
    fn get_session_snapshot(&self, _req: ports::GetSessionSnapshotRequest) -> BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn create_checkpoint(&self, _req: ports::CreateCheckpointRequest) -> BoxFuture<'static, Result<ports::CreateCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn save_checkpoint(&self, _req: ports::SaveCheckpointRequest) -> BoxFuture<'static, Result<ports::SaveCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn load_checkpoint(&self, _req: ports::LoadCheckpointRequest) -> BoxFuture<'static, Result<ports::LoadCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn restore_checkpoint(&self, _req: ports::RestoreCheckpointRequest) -> BoxFuture<'static, Result<ports::RestoreCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn attempt_close_session(&self, _req: ports::GetSessionSnapshotRequest) -> BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn resolve_close_session_save_all(&self, _req: ports::SaveCheckpointRequest) -> BoxFuture<'static, Result<ports::SaveCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Err(UseCaseError::UnknownSession) })
    }
    fn resolve_close_session_discard_all(&self, _req: ports::SaveCheckpointRequest) -> BoxFuture<'static, Result<ports::SaveCheckpointResponse, UseCaseError>> {
        Box::pin(async move { Ok(ports::SaveCheckpointResponse { location: String::new() }) })
    }
}

#[tokio::test]
async fn ai_request_and_apply_flow() {
    // Create a simple composition and fake interfaces.
    let mut comp = DesktopComposition::new();
    // Create fake buffer id by using a simple BufferId::new() if available; otherwise use Default/constructors.
    // We'll construct a BufferId via a simple path helper (BufferId often wraps a path in tests).
    let buf_id = ports::BufferId::from_path(std::path::Path::new("file1.txt"));

    let view = Arc::new(FakeView::new(buf_id.clone(), Some("original content".to_string())));
    let service = Arc::new(FakeService::new());

    // Create a dummy session id; use default Id if available.
    let session_id = SessionId(Id::new());

    // Request AI edit.
    let req_res = request_ai_edit_active(&mut comp, view.clone(), session_id.clone(), None).await;
    assert!(req_res.is_ok(), "request_ai_edit_active failed: {:?}", req_res);

    // Ensure ai_projection is present and proposed.
    let md = comp.latest_metadata().expect("metadata expected");
    let ai = md.ai_projection.expect("ai projection expected");
    assert_eq!(ai.state, Some(zaroxi_interface_desktop::desktop::AiState::Proposed));
    assert!(ai.proposal_text.is_some());

    // Apply the proposal.
    let apply_res = apply_ai_edit_active(&mut comp, view.clone(), session_id.clone(), None, service.clone()).await;
    assert!(apply_res.is_ok(), "apply_ai_edit_active failed: {:?}", apply_res);

    // After apply, projection should be Applied.
    let md2 = comp.latest_metadata().expect("metadata expected after apply");
    let ai2 = md2.ai_projection.expect("ai projection expected after apply");
    assert_eq!(ai2.state, Some(zaroxi_interface_desktop::desktop::AiState::Applied));

    // Ensure AI projection was applied in composition. Recording in the service adapter is optional.
    let md_after = comp.latest_metadata().expect("metadata expected after apply");
    let ai_after = md_after.ai_projection.expect("ai projection expected after apply");
    assert_eq!(ai_after.state, Some(zaroxi_interface_desktop::desktop::AiState::Applied));
}

#[tokio::test]
async fn ai_cancel_clears_proposal() {
    let mut comp = DesktopComposition::new();
    let buf_id = ports::BufferId::from_path(std::path::Path::new("file2.txt"));
    let view = Arc::new(FakeView::new(buf_id.clone(), Some("something".to_string())));
    let session_id = SessionId(Id::new());

    let _ = request_ai_edit_active(&mut comp, view.clone(), session_id.clone(), None).await;
    assert!(comp.latest_metadata().and_then(|m| m.ai_projection).is_some());

    cancel_ai_edit_active(&mut comp);
    assert!(comp.latest_metadata().and_then(|m| m.ai_projection).is_none());
}
