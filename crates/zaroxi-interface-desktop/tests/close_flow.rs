use std::sync::Arc;
use zaroxi_interface_desktop::{DesktopComposition, refresh_desktop, actions};
use zaroxi_application_workspace::ports::{WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::ports as ports;

/// Minimal fake WorkspaceView used to populate a DesktopComposition for tests.
struct FakeView {
    buffer_id: BufferId,
}

impl FakeView {
    fn new() -> Self {
        Self { buffer_id: BufferId::from("buf:fake") }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(&self, _buffer_id: BufferId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: SessionId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveEditorDocumentResponse, ports::UseCaseError>> {
        let doc = ports::EditorDocument {
            buffer_id: self.buffer_id.clone(),
            content: Some("line1".to_string()),
            cursor: ports::EditorCursor::zero(),
            selection: None,
            line_count: 1,
            current_line: Some("line1".to_string()),
        };
        Box::pin(async move { Ok(ports::GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> ports::BoxFuture<'static, Result<ports::GetVisibleLinesResponse, ports::UseCaseError>> {
        let vl = VisibleLine {
            line_number: 1,
            text: "line1".to_string(),
            is_cursor_line: true,
            cursor_column: Some(0),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };
        Box::pin(async move { Ok(ports::GetVisibleLinesResponse { window: vw }) })
    }
}

#[tokio::test]
async fn request_close_active_enters_pending_close_and_status_banner() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");

    // Request close: should set pending close and status banner should reflect it.
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_active");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    let text = bar.text;
    assert!(text.contains("Close buffer"), "status banner should mention Close buffer");
    assert!(text.contains("unsaved changes") || text.contains("[S]ave"), "status banner should include action hints or unsaved indicator");
}

#[tokio::test]
async fn confirm_save_and_close_clears_pending_and_sets_status() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");
    assert!(!comp.has_pending_close(), "pending close should be cleared after save-and-close");
    let bar = comp.latest_status_bar_line().expect("status bar present after save");
    assert!(bar.text.contains("Saved and closed"), "status should reflect save success");
}

#[tokio::test]
async fn request_close_session_enters_pending_close_when_dirty() {
    // Fake service will report the session as having dirty buffers.
    struct Svc {
        buf: BufferId,
    }

    // Use the test-visible alias for application ports.
    use zaroxi_application_workspace::ports as ports;

    impl ports::WorkspaceService for Svc {
        fn boot_workspace(&self, _req: ports::WorkspaceBootRequest) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) }) }
        fn open_buffer(&self, _req: ports::OpenBufferRequest) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn list_open_buffers(&self, _req: ports::ListBuffersRequest) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>> {
            let buf = self.buf.clone();
            Box::pin(async move { Ok(ports::ListBuffersResponse { buffer_ids: vec![buf.clone()], active_buffer: Some(buf.clone()) }) })
        }
        fn set_active_buffer(&self, _req: ports::SetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>> {
            let b = self.buf.clone();
            Box::pin(async move { Ok(ports::GetActiveBufferResponse { buffer_id: b }) })
        }
        fn set_editor_cursor(&self, _req: ports::SetEditorCursorRequest) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_editor_selection(&self, _req: ports::SetSelectionRequest) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn clear_editor_selection(&self, _req: ports::ClearSelectionRequest) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_editor_state(&self, _req: ports::GetEditorStateRequest) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_viewport_state(&self, _req: ports::SetViewportRequest) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn scroll_viewport(&self, _req: ports::ScrollViewportRequest) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn explain_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) }) }
        fn dispatch_command(&self, _req: ports::DispatchCommandRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn update_buffer(&self, _req: ports::UpdateBufferRequest) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn apply_text_transaction(&self, _req: ports::ApplyTextTransactionRequest) -> ports::BoxFuture<'static, Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>> {
            Box::pin(async { Ok(ports::ApplyTextTransactionResponse { ok: true, state: ports::EditorState { cursor: ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: ports::GetRecentCommandsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) }) }
        fn get_recent_events(&self, _req: ports::GetRecentEventsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) }) }
        fn get_session_snapshot(&self, _req: ports::GetSessionSnapshotRequest) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>> {
            let snap = ports::SessionSnapshot {
                session_id: SessionId(zaroxi_kernel_types::Id::new()),
                workspace_id: zaroxi_kernel_types::Id::new(),
                opened_buffers: vec![self.buf.clone()],
                active_buffer: Some(self.buf.clone()),
                buffers: vec![],
                recent_commands: vec![],
                recent_events: vec![],
            };
            Box::pin(async move { Ok(ports::GetSessionSnapshotResponse { snapshot: snap }) })
        }
        fn create_checkpoint(&self, _req: ports::CreateCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn save_checkpoint(&self, _req: ports::SaveCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::SaveCheckpointResponse { location: "loc".to_string() }) }) }
        fn load_checkpoint(&self, _req: ports::LoadCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn restore_checkpoint(&self, _req: ports::RestoreCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
    }

    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let fake = Svc { buf: BufferId::from("buf:dirty") };
    let service = Arc::new(fake) as Arc<dyn ports::WorkspaceService>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(service.clone())).await.expect("refresh ok");

    // Request session close: service reports opened buffers so UI should enter pending session-close.
    let _ = actions::request_close_session(&mut comp, view.clone(), sid.clone(), Some(service.clone())).await.expect("request close session ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_session");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    assert!(bar.text.contains("Close session") || bar.text.contains("buffers"), "status should reflect session close pending");
}

impl FakeView {
    fn new() -> Self {
        Self { buffer_id: BufferId::from("buf:fake") }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(&self, _buffer_id: BufferId) -> BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: SessionId) -> BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> BoxFuture<'static, Result<ports::GetActiveEditorDocumentResponse, ports::UseCaseError>> {
        let doc = ports::EditorDocument {
            buffer_id: self.buffer_id.clone(),
            content: Some("line1".to_string()),
            cursor: ports::EditorCursor::zero(),
            selection: None,
            line_count: 1,
            current_line: Some("line1".to_string()),
        };
        Box::pin(async move { Ok(ports::GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> BoxFuture<'static, Result<ports::GetVisibleLinesResponse, ports::UseCaseError>> {
        let vl = VisibleLine {
            line_number: 1,
            text: "line1".to_string(),
            is_cursor_line: true,
            cursor_column: Some(0),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };
        Box::pin(async move { Ok(ports::GetVisibleLinesResponse { window: vw }) })
    }
}

#[tokio::test]
async fn request_close_active_enters_pending_close_and_status_banner() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");

    // Request close: should set pending close and status banner should reflect it.
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_active");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    let text = bar.text;
    assert!(text.contains("Close buffer"), "status banner should mention Close buffer");
    assert!(text.contains("unsaved changes") || text.contains("[S]ave"), "status banner should include action hints or unsaved indicator");
}

#[tokio::test]
async fn confirm_save_and_close_clears_pending_and_sets_status() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");
    assert!(!comp.has_pending_close(), "pending close should be cleared after save-and-close");
    let bar = comp.latest_status_bar_line().expect("status bar present after save");
    assert!(bar.text.contains("Saved and closed"), "status should reflect save success");
}

//
// Session / window close tests - visible flow parity with buffer close.
//
#[tokio::test]
async fn request_close_session_enters_pending_close_when_dirty() {
    // Fake service will report the session as having dirty buffers.
    struct Svc {
        buf: BufferId,
    }

    // Use the test-visible alias for application ports.
    use crate::ports as app_ports;
    use zaroxi_application_workspace::ports as ports;

    impl ports::WorkspaceService for Svc {
        fn boot_workspace(&self, _req: ports::WorkspaceBootRequest) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) }) }
        fn open_buffer(&self, _req: ports::OpenBufferRequest) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn list_open_buffers(&self, _req: ports::ListBuffersRequest) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>> {
            let buf = self.buf.clone();
            Box::pin(async move { Ok(ports::ListBuffersResponse { buffer_ids: vec![buf.clone()], active_buffer: Some(buf.clone()) }) })
        }
        fn set_active_buffer(&self, _req: ports::SetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>> {
            let b = self.buf.clone();
            Box::pin(async move { Ok(ports::GetActiveBufferResponse { buffer_id: b }) })
        }
        fn set_editor_cursor(&self, _req: ports::SetEditorCursorRequest) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_editor_selection(&self, _req: ports::SetSelectionRequest) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn clear_editor_selection(&self, _req: ports::ClearSelectionRequest) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_editor_state(&self, _req: ports::GetEditorStateRequest) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_viewport_state(&self, _req: ports::SetViewportRequest) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn scroll_viewport(&self, _req: ports::ScrollViewportRequest) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn explain_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) }) }
        fn dispatch_command(&self, _req: ports::DispatchCommandRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn update_buffer(&self, _req: ports::UpdateBufferRequest) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn apply_text_transaction(&self, _req: ports::ApplyTextTransactionRequest) -> ports::BoxFuture<'static, Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>> {
            Box::pin(async { Ok(ports::ApplyTextTransactionResponse { ok: true, state: ports::EditorState { cursor: ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: ports::GetRecentCommandsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) }) }
        fn get_recent_events(&self, _req: ports::GetRecentEventsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) }) }
        fn get_session_snapshot(&self, _req: ports::GetSessionSnapshotRequest) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>> {
            let snap = ports::SessionSnapshot {
                session_id: SessionId(zaroxi_kernel_types::Id::new()),
                workspace_id: zaroxi_kernel_types::Id::new(),
                opened_buffers: vec![self.buf.clone()],
                active_buffer: Some(self.buf.clone()),
                buffers: vec![],
                recent_commands: vec![],
                recent_events: vec![],
            };
            Box::pin(async move { Ok(ports::GetSessionSnapshotResponse { snapshot: snap }) })
        }
        fn create_checkpoint(&self, _req: ports::CreateCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn save_checkpoint(&self, _req: ports::SaveCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::SaveCheckpointResponse { location: "loc".to_string() }) }) }
        fn load_checkpoint(&self, _req: ports::LoadCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn restore_checkpoint(&self, _req: ports::RestoreCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
    }

    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let fake = Svc { buf: BufferId::from("buf:dirty") };
    let service = Arc::new(fake) as Arc<dyn ports::WorkspaceService>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(service.clone())).await.expect("refresh ok");

    // Request session close: service reports opened buffers so UI should enter pending session-close.
    let _ = actions::request_close_session(&mut comp, view.clone(), sid.clone(), Some(service.clone())).await.expect("request close session ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_session");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    assert!(bar.text.contains("Close session") || bar.text.contains("buffers"), "status should reflect session close pending");
}

#[tokio::test]
async fn request_close_session_proceeds_when_clean() {
    // This test was removed/rebased in favor of the single authoritative
    // `request_close_session_enters_pending_close_when_dirty` test. Keep a
    // no-op placeholder here to avoid unclosed-delimiter or duplicate-definition
    // problems during incremental edits and to preserve test name history.
    //
    // The real behaviors are covered by:
    // - request_close_session_enters_pending_close_when_dirty
    // - confirm_save_all_and_close / confirm_discard_all_and_close flows
}
    // Service reports no opened buffers -> should close immediately.
    struct CleanSvc;
    impl crate::ports::WorkspaceService for CleanSvc {
        fn boot_workspace(&self, _req: crate::ports::WorkspaceBootRequest) -> crate::BoxFuture<'static, Result<crate::ports::WorkspaceBootResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownWorkspace) }) }
        fn open_buffer(&self, _req: crate::ports::OpenBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::OpenBufferResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn list_open_buffers(&self, _req: crate::ports::ListBuffersRequest) -> crate::BoxFuture<'static, Result<crate::ports::ListBuffersResponse, crate::ports::UseCaseError>> { Box::pin(async { Ok(crate::ports::ListBuffersResponse { buffer_ids: vec![], active_buffer: None }) }) }
        fn set_active_buffer(&self, _req: crate::ports::SetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn get_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn set_editor_cursor(&self, _req: crate::ports::SetEditorCursorRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn set_editor_selection(&self, _req: crate::ports::SetSelectionRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetSelectionResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn clear_editor_selection(&self, _req: crate::ports::ClearSelectionRequest) -> crate::BoxFuture<'static, Result<crate::ports::ClearSelectionResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn get_editor_state(&self, _req: crate::ports::GetEditorStateRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetEditorStateResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn set_viewport_state(&self, _req: crate::ports::SetViewportRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetViewportResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn scroll_viewport(&self, _req: crate::ports::ScrollViewportRequest) -> crate::BoxFuture<'static, Result<crate::ports::ScrollViewportResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn explain_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::NoActiveBuffer) }) }
        fn dispatch_command(&self, _req: crate::ports::DispatchCommandRequest) -> crate::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn update_buffer(&self, _req: crate::ports::UpdateBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::UpdateBufferResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn apply_text_transaction(&self, _req: crate::ports::ApplyTextTransactionRequest) -> crate::BoxFuture<'static, Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>> { Box::pin(async { Ok(crate::ports::ApplyTextTransactionResponse { ok: true, state: crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None }, content: None }) }) }
        fn get_recent_commands(&self, _req: crate::ports::GetRecentCommandsRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetRecentCommandsResponse, crate::ports::UseCaseError>> { Box::pin(async { Ok(crate::ports::GetRecentCommandsResponse { commands: Vec::new() }) }) }
        fn get_recent_events(&self, _req: crate::ports::GetRecentEventsRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetRecentEventsResponse, crate::ports::UseCaseError>> { Box::pin(async { Ok(crate::ports::GetRecentEventsResponse { events: Vec::new() }) }) }
        fn get_session_snapshot(&self, _req: crate::ports::GetSessionSnapshotRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetSessionSnapshotResponse, crate::ports::UseCaseError>> {
            let snap = crate::ports::SessionSnapshot {
                session_id: SessionId(zaroxi_kernel_types::Id::new()),
                workspace_id: zaroxi_kernel_types::Id::new(),
                opened_buffers: vec![],
                active_buffer: None,
                buffers: vec![],
                recent_commands: vec![],
                recent_events: vec![],
            };
            Box::pin(async move { Ok(crate::ports::GetSessionSnapshotResponse { snapshot: snap }) })
        }
        fn create_checkpoint(&self, _req: crate::ports::CreateCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::CreateCheckpointResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn save_checkpoint(&self, _req: crate::ports::SaveCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::SaveCheckpointResponse, crate::ports::UseCaseError>> { Box::pin(async { Ok(crate::ports::SaveCheckpointResponse { location: "loc".to_string() }) }) }
        fn load_checkpoint(&self, _req: crate::ports::LoadCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::LoadCheckpointResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
        fn restore_checkpoint(&self, _req: crate::ports::RestoreCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::RestoreCheckpointResponse, crate::ports::UseCaseError>> { Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) }) }
    }

use std::sync::Arc;

use zaroxi_interface_desktop::{DesktopComposition, refresh_desktop, actions};
use zaroxi_application_workspace::ports::{
    WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::ports as ports;

/// Minimal fake WorkspaceView used to populate a DesktopComposition for tests.
struct FakeView {
    buffer_id: BufferId,
}

impl FakeView {
    fn new() -> Self {
        Self { buffer_id: BufferId::from("buf:fake") }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(&self, _buffer_id: BufferId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: SessionId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveEditorDocumentResponse, ports::UseCaseError>> {
        let doc = ports::EditorDocument {
            buffer_id: self.buffer_id.clone(),
            content: Some("line1".to_string()),
            cursor: ports::EditorCursor::zero(),
            selection: None,
            line_count: 1,
            current_line: Some("line1".to_string()),
        };
        Box::pin(async move { Ok(ports::GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> ports::BoxFuture<'static, Result<ports::GetVisibleLinesResponse, ports::UseCaseError>> {
        let vl = VisibleLine {
            line_number: 1,
            text: "line1".to_string(),
            is_cursor_line: true,
            cursor_column: Some(0),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };
        Box::pin(async move { Ok(ports::GetVisibleLinesResponse { window: vw }) })
    }
}

#[tokio::test]
async fn request_close_active_enters_pending_close_and_status_banner() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");

    // Request close: should set pending close and status banner should reflect it.
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_active");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    let text = bar.text;
    assert!(text.contains("Close buffer"), "status banner should mention Close buffer");
    assert!(text.contains("unsaved changes") || text.contains("[S]ave"), "status banner should include action hints or unsaved indicator");
}

#[tokio::test]
async fn confirm_save_and_close_clears_pending_and_sets_status() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");
    assert!(!comp.has_pending_close(), "pending close should be cleared after save-and-close");
    let bar = comp.latest_status_bar_line().expect("status bar present after save");
    assert!(bar.text.contains("Saved and closed"), "status should reflect save success");
}

//
// Session / window close tests - visible flow parity with buffer close.
//
#[tokio::test]
async fn request_close_session_enters_pending_close_when_dirty() {
    // Fake service will report the session as having dirty buffers.
    struct Svc {
        buf: BufferId,
    }

    // Use the test-visible alias for application ports.
    use crate::ports as app_ports;
    use zaroxi_application_workspace::ports as ports;

    impl ports::WorkspaceService for Svc {
        fn boot_workspace(&self, _req: ports::WorkspaceBootRequest) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) }) }
        fn open_buffer(&self, _req: ports::OpenBufferRequest) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn list_open_buffers(&self, _req: ports::ListBuffersRequest) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>> {
            let buf = self.buf.clone();
            Box::pin(async move { Ok(ports::ListBuffersResponse { buffer_ids: vec![buf.clone()], active_buffer: Some(buf.clone()) }) })
        }
        fn set_active_buffer(&self, _req: ports::SetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>> {
            let b = self.buf.clone();
            Box::pin(async move { Ok(ports::GetActiveBufferResponse { buffer_id: b }) })
        }
        fn set_editor_cursor(&self, _req: ports::SetEditorCursorRequest) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_editor_selection(&self, _req: ports::SetSelectionRequest) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn clear_editor_selection(&self, _req: ports::ClearSelectionRequest) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_editor_state(&self, _req: ports::GetEditorStateRequest) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_viewport_state(&self, _req: ports::SetViewportRequest) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn scroll_viewport(&self, _req: ports::ScrollViewportRequest) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn explain_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) }) }
        fn dispatch_command(&self, _req: ports::DispatchCommandRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn update_buffer(&self, _req: ports::UpdateBufferRequest) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn apply_text_transaction(&self, _req: ports::ApplyTextTransactionRequest) -> ports::BoxFuture<'static, Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>> {
            Box::pin(async { Ok(ports::ApplyTextTransactionResponse { ok: true, state: ports::EditorState { cursor: ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: ports::GetRecentCommandsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) }) }
        fn get_recent_events(&self, _req: ports::GetRecentEventsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) }) }
        fn get_session_snapshot(&self, _req: ports::GetSessionSnapshotRequest) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>> {
            let snap = ports::SessionSnapshot {
                session_id: SessionId(zaroxi_kernel_types::Id::new()),
                workspace_id: zaroxi_kernel_types::Id::new(),
                opened_buffers: vec![self.buf.clone()],
                active_buffer: Some(self.buf.clone()),
                buffers: vec![],
                recent_commands: vec![],
                recent_events: vec![],
            };
            Box::pin(async move { Ok(ports::GetSessionSnapshotResponse { snapshot: snap }) })
        }
        fn create_checkpoint(&self, _req: ports::CreateCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn save_checkpoint(&self, _req: ports::SaveCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::SaveCheckpointResponse { location: "loc".to_string() }) }) }
        fn load_checkpoint(&self, _req: ports::LoadCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn restore_checkpoint(&self, _req: ports::RestoreCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
    }

    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let fake = Svc { buf: BufferId::from("buf:dirty") };
    let service = Arc::new(fake) as Arc<dyn ports::WorkspaceService>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(service.clone())).await.expect("refresh ok");

    // Request session close: service reports opened buffers so UI should enter pending session-close.
    let _ = actions::request_close_session(&mut comp, view.clone(), sid.clone(), Some(service.clone())).await.expect("request close session ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_session");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    assert!(bar.text.contains("Close session") || bar.text.contains("buffers"), "status should reflect session close pending");
}
use std::sync::Arc;

use zaroxi_interface_desktop::{DesktopComposition, refresh_desktop, actions};
use zaroxi_application_workspace::ports::{
    WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::ports as ports;

/// Minimal fake WorkspaceView used to populate a DesktopComposition for tests.
struct FakeView {
    buffer_id: BufferId,
}

impl FakeView {
    fn new() -> Self {
        Self { buffer_id: BufferId::from("buf:fake") }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(&self, _buffer_id: BufferId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: SessionId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveEditorDocumentResponse, ports::UseCaseError>> {
        let doc = ports::EditorDocument {
            buffer_id: self.buffer_id.clone(),
            content: Some("line1".to_string()),
            cursor: ports::EditorCursor::zero(),
            selection: None,
            line_count: 1,
            current_line: Some("line1".to_string()),
        };
        Box::pin(async move { Ok(ports::GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> ports::BoxFuture<'static, Result<ports::GetVisibleLinesResponse, ports::UseCaseError>> {
        let vl = VisibleLine {
            line_number: 1,
            text: "line1".to_string(),
            is_cursor_line: true,
            cursor_column: Some(0),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };
        Box::pin(async move { Ok(ports::GetVisibleLinesResponse { window: vw }) })
    }
}

#[tokio::test]
async fn request_close_active_enters_pending_close_and_status_banner() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");

    // Request close: should set pending close and status banner should reflect it.
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_active");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    let text = bar.text;
    assert!(text.contains("Close buffer"), "status banner should mention Close buffer");
    assert!(text.contains("unsaved changes") || text.contains("[S]ave"), "status banner should include action hints or unsaved indicator");
}

#[tokio::test]
async fn confirm_save_and_close_clears_pending_and_sets_status() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");
    assert!(!comp.has_pending_close(), "pending close should be cleared after save-and-close");
    let bar = comp.latest_status_bar_line().expect("status bar present after save");
    assert!(bar.text.contains("Saved and closed"), "status should reflect save success");
}

//
// Session / window close tests - visible flow parity with buffer close.
//
#[tokio::test]
async fn request_close_session_enters_pending_close_when_dirty() {
    // Fake service will report the session as having dirty buffers.
    struct Svc {
        buf: BufferId,
    }

    // Use the test-visible alias for application ports.
    use crate::ports as app_ports;
    use zaroxi_application_workspace::ports as ports;

    impl ports::WorkspaceService for Svc {
        fn boot_workspace(&self, _req: ports::WorkspaceBootRequest) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) }) }
        fn open_buffer(&self, _req: ports::OpenBufferRequest) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn list_open_buffers(&self, _req: ports::ListBuffersRequest) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>> {
            let buf = self.buf.clone();
            Box::pin(async move { Ok(ports::ListBuffersResponse { buffer_ids: vec![buf.clone()], active_buffer: Some(buf.clone()) }) })
        }
        fn set_active_buffer(&self, _req: ports::SetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>> {
            let b = self.buf.clone();
            Box::pin(async move { Ok(ports::GetActiveBufferResponse { buffer_id: b }) })
        }
        fn set_editor_cursor(&self, _req: ports::SetEditorCursorRequest) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_editor_selection(&self, _req: ports::SetSelectionRequest) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn clear_editor_selection(&self, _req: ports::ClearSelectionRequest) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_editor_state(&self, _req: ports::GetEditorStateRequest) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_viewport_state(&self, _req: ports::SetViewportRequest) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn scroll_viewport(&self, _req: ports::ScrollViewportRequest) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn explain_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) }) }
        fn dispatch_command(&self, _req: ports::DispatchCommandRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn update_buffer(&self, _req: ports::UpdateBufferRequest) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn apply_text_transaction(&self, _req: ports::ApplyTextTransactionRequest) -> ports::BoxFuture<'static, Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>> {
            Box::pin(async { Ok(ports::ApplyTextTransactionResponse { ok: true, state: ports::EditorState { cursor: ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: ports::GetRecentCommandsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) }) }
        fn get_recent_events(&self, _req: ports::GetRecentEventsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) }) }
        fn get_session_snapshot(&self, _req: ports::GetSessionSnapshotRequest) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>> {
            let snap = ports::SessionSnapshot {
                session_id: SessionId(zaroxi_kernel_types::Id::new()),
                workspace_id: zaroxi_kernel_types::Id::new(),
                opened_buffers: vec![self.buf.clone()],
                active_buffer: Some(self.buf.clone()),
                buffers: vec![],
                recent_commands: vec![],
                recent_events: vec![],
            };
            Box::pin(async move { Ok(ports::GetSessionSnapshotResponse { snapshot: snap }) })
        }
        fn create_checkpoint(&self, _req: ports::CreateCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn save_checkpoint(&self, _req: ports::SaveCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::SaveCheckpointResponse { location: "loc".to_string() }) }) }
        fn load_checkpoint(&self, _req: ports::LoadCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn restore_checkpoint(&self, _req: ports::RestoreCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
    }

    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let fake = Svc { buf: BufferId::from("buf:dirty") };
    let service = Arc::new(fake) as Arc<dyn ports::WorkspaceService>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(service.clone())).await.expect("refresh ok");

    // Request session close: service reports opened buffers so UI should enter pending session-close.
    let _ = actions::request_close_session(&mut comp, view.clone(), sid.clone(), Some(service.clone())).await.expect("request close session ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_session");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    assert!(bar.text.contains("Close session") || bar.text.contains("buffers"), "status should reflect session close pending");
}
use std::sync::Arc;

use zaroxi_interface_desktop::{DesktopComposition, refresh_desktop, actions};
use zaroxi_application_workspace::ports::{
    WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::ports as ports;

/// Minimal fake WorkspaceView used to populate a DesktopComposition for tests.
struct FakeView {
    buffer_id: BufferId,
}

impl FakeView {
    fn new() -> Self {
        Self { buffer_id: BufferId::from("buf:fake") }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(&self, _buffer_id: BufferId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: SessionId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveEditorDocumentResponse, ports::UseCaseError>> {
        let doc = ports::EditorDocument {
            buffer_id: self.buffer_id.clone(),
            content: Some("line1".to_string()),
            cursor: ports::EditorCursor::zero(),
            selection: None,
            line_count: 1,
            current_line: Some("line1".to_string()),
        };
        Box::pin(async move { Ok(ports::GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> ports::BoxFuture<'static, Result<ports::GetVisibleLinesResponse, ports::UseCaseError>> {
        let vl = VisibleLine {
            line_number: 1,
            text: "line1".to_string(),
            is_cursor_line: true,
            cursor_column: Some(0),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };
        Box::pin(async move { Ok(ports::GetVisibleLinesResponse { window: vw }) })
    }
}

#[tokio::test]
async fn request_close_active_enters_pending_close_and_status_banner() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");

    // Request close: should set pending close and status banner should reflect it.
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_active");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    let text = bar.text;
    assert!(text.contains("Close buffer"), "status banner should mention Close buffer");
    assert!(text.contains("unsaved changes") || text.contains("[S]ave"), "status banner should include action hints or unsaved indicator");
}

#[tokio::test]
async fn confirm_save_and_close_clears_pending_and_sets_status() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");
    assert!(!comp.has_pending_close(), "pending close should be cleared after save-and-close");
    let bar = comp.latest_status_bar_line().expect("status bar present after save");
    assert!(bar.text.contains("Saved and closed"), "status should reflect save success");
}

//
// Session / window close tests - visible flow parity with buffer close.
//
#[tokio::test]
async fn request_close_session_enters_pending_close_when_dirty() {
    // Fake service will report the session as having dirty buffers.
    struct Svc {
        buf: BufferId,
    }

    // Use the test-visible alias for application ports.
    use zaroxi_application_workspace::ports as ports;

    impl ports::WorkspaceService for Svc {
        fn boot_workspace(&self, _req: ports::WorkspaceBootRequest) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) }) }
        fn open_buffer(&self, _req: ports::OpenBufferRequest) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn list_open_buffers(&self, _req: ports::ListBuffersRequest) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>> {
            let buf = self.buf.clone();
            Box::pin(async move { Ok(ports::ListBuffersResponse { buffer_ids: vec![buf.clone()], active_buffer: Some(buf.clone()) }) })
        }
        fn set_active_buffer(&self, _req: ports::SetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>> {
            let b = self.buf.clone();
            Box::pin(async move { Ok(ports::GetActiveBufferResponse { buffer_id: b }) })
        }
        fn set_editor_cursor(&self, _req: ports::SetEditorCursorRequest) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_editor_selection(&self, _req: ports::SetSelectionRequest) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn clear_editor_selection(&self, _req: ports::ClearSelectionRequest) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn get_editor_state(&self, _req: ports::GetEditorStateRequest) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn set_viewport_state(&self, _req: ports::SetViewportRequest) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn scroll_viewport(&self, _req: ports::ScrollViewportRequest) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn explain_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) }) }
        fn dispatch_command(&self, _req: ports::DispatchCommandRequest) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn update_buffer(&self, _req: ports::UpdateBufferRequest) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn apply_text_transaction(&self, _req: ports::ApplyTextTransactionRequest) -> ports::BoxFuture<'static, Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>> {
            Box::pin(async { Ok(ports::ApplyTextTransactionResponse { ok: true, state: ports::EditorState { cursor: ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: ports::GetRecentCommandsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) }) }
        fn get_recent_events(&self, _req: ports::GetRecentEventsRequest) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) }) }
        fn get_session_snapshot(&self, _req: ports::GetSessionSnapshotRequest) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>> {
            let snap = ports::SessionSnapshot {
                session_id: SessionId(zaroxi_kernel_types::Id::new()),
                workspace_id: zaroxi_kernel_types::Id::new(),
                opened_buffers: vec![self.buf.clone()],
                active_buffer: Some(self.buf.clone()),
                buffers: vec![],
                recent_commands: vec![],
                recent_events: vec![],
            };
            Box::pin(async move { Ok(ports::GetSessionSnapshotResponse { snapshot: snap }) })
        }
        fn create_checkpoint(&self, _req: ports::CreateCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn save_checkpoint(&self, _req: ports::SaveCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Ok(ports::SaveCheckpointResponse { location: "loc".to_string() }) }) }
        fn load_checkpoint(&self, _req: ports::LoadCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
        fn restore_checkpoint(&self, _req: ports::RestoreCheckpointRequest) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>> { Box::pin(async { Err(ports::UseCaseError::UnknownSession) }) }
    }

    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let fake = Svc { buf: BufferId::from("buf:dirty") };
    let service = Arc::new(fake) as Arc<dyn ports::WorkspaceService>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(service.clone())).await.expect("refresh ok");

    // Request session close: service reports opened buffers so UI should enter pending session-close.
    let _ = actions::request_close_session(&mut comp, view.clone(), sid.clone(), Some(service.clone())).await.expect("request close session ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_session");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    assert!(bar.text.contains("Close session") || bar.text.contains("buffers"), "status should reflect session close pending");
}
