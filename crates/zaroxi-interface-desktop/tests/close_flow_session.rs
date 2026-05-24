mod close_flow_common;
use std::sync::Arc;
use close_flow_common::CloseFlowViewStub;

use zaroxi_application_workspace::ports;
use zaroxi_application_workspace::ports::{GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, WorkspaceView};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop::{DesktopComposition, actions, refresh_desktop};

/// Minimal service stub that reports one opened buffer (dirty).
struct DirtyService;

impl ports::WorkspaceService for DirtyService {
    fn boot_workspace(
        &self,
        _req: ports::WorkspaceBootRequest,
    ) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) })
    }
    fn open_buffer(
        &self,
        _req: ports::OpenBufferRequest,
    ) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn list_open_buffers(
        &self,
        _req: ports::ListBuffersRequest,
    ) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>> {
        let buf = BufferId::from("buf:dirty");
        Box::pin(async move {
            Ok(ports::ListBuffersResponse {
                buffer_ids: vec![buf.clone()],
                active_buffer: Some(buf),
            })
        })
    }
    fn set_active_buffer(
        &self,
        _req: ports::SetActiveBufferRequest,
    ) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn get_active_buffer(
        &self,
        _req: ports::GetActiveBufferRequest,
    ) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn set_editor_cursor(
        &self,
        _req: ports::SetEditorCursorRequest,
    ) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn set_editor_selection(
        &self,
        _req: ports::SetSelectionRequest,
    ) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn clear_editor_selection(
        &self,
        _req: ports::ClearSelectionRequest,
    ) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn get_editor_state(
        &self,
        _req: ports::GetEditorStateRequest,
    ) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn set_viewport_state(
        &self,
        _req: ports::SetViewportRequest,
    ) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn scroll_viewport(
        &self,
        _req: ports::ScrollViewportRequest,
    ) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn explain_active_buffer(
        &self,
        _req: ports::GetActiveBufferRequest,
    ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) })
    }
    fn dispatch_command(
        &self,
        _req: ports::DispatchCommandRequest,
    ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn update_buffer(
        &self,
        _req: ports::UpdateBufferRequest,
    ) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn apply_text_transaction(
        &self,
        _req: ports::ApplyTextTransactionRequest,
    ) -> ports::BoxFuture<'static, Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>> {
        Box::pin(async {
            Ok(ports::ApplyTextTransactionResponse {
                ok: true,
                state: ports::EditorState {
                    cursor: ports::EditorCursor::zero(),
                    selection: None,
                },
                content: None,
            })
        })
    }
    fn get_recent_commands(
        &self,
        _req: ports::GetRecentCommandsRequest,
    ) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>> {
        Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) })
    }
    fn get_recent_events(
        &self,
        _req: ports::GetRecentEventsRequest,
    ) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>> {
        Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) })
    }
    fn get_session_snapshot(
        &self,
        _req: ports::GetSessionSnapshotRequest,
    ) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>> {
        let snap = ports::SessionSnapshot {
            session_id: SessionId(zaroxi_kernel_types::Id::new()),
            workspace_id: zaroxi_kernel_types::Id::new(),
            opened_buffers: vec![BufferId::from("buf:dirty")],
            active_buffer: Some(BufferId::from("buf:dirty")),
            buffers: vec![],
            recent_commands: vec![],
            recent_events: vec![],
        };
        Box::pin(async move { Ok(ports::GetSessionSnapshotResponse { snapshot: snap }) })
    }
    fn create_checkpoint(
        &self,
        _req: ports::CreateCheckpointRequest,
    ) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn save_checkpoint(
        &self,
        _req: ports::SaveCheckpointRequest,
    ) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn load_checkpoint(
        &self,
        _req: ports::LoadCheckpointRequest,
    ) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
    fn restore_checkpoint(
        &self,
        _req: ports::RestoreCheckpointRequest,
    ) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
}

#[tokio::test]
async fn request_close_session_enters_pending_close_when_dirty() {
    let view = Arc::new(CloseFlowViewStub::new()) as Arc<dyn WorkspaceView>;
    let service = Arc::new(DirtyService) as Arc<dyn ports::WorkspaceService>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(service.clone()))
        .await
        .expect("refresh ok");

    // Request session close: service reports opened buffers so UI should enter pending session-close.
    let _ =
        actions::request_close_session(&mut comp, view.clone(), sid.clone(), Some(service.clone()))
            .await
            .expect("request close session ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_session");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    assert!(
        bar.text.contains("Close session") || bar.text.contains("buffers"),
        "status should reflect session close pending"
    );
}
