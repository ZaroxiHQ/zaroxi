use super::*;
use std::sync::Arc;
use zaroxi_application_workspace::ports::{
    WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, GetActiveEditorDocumentResponse, GetVisibleLinesResponse, EditorDocument, EditorCursor,
};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};

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

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> crate::ports::BoxFuture<'static, Result<GetActiveEditorDocumentResponse, crate::ports::UseCaseError>> {
        let d = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> crate::ports::BoxFuture<'static, Result<GetVisibleLinesResponse, crate::ports::UseCaseError>> {
        let w = self.window.clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

#[tokio::test]
async fn desktop_composition_refreshes_and_stores_metadata() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let wid = zaroxi_kernel_types::Id::new();

    let mut comp = DesktopComposition::new();
    comp.refresh(arc.clone(), sid.clone(), Some(wid.clone())).await.expect("refresh ok");

    assert_eq!(comp.get_session_id().unwrap(), sid);
    assert_eq!(comp.get_workspace_id().unwrap(), wid);
    let win = comp.latest_window().expect("window present");
    assert_eq!(win.total_lines, 1);
    assert_eq!(win.lines.len(), 1);

    // Revision should have advanced from initial 0 to 1 after the first refresh.
    assert_eq!(comp.latest_revision(), 1);

    // A subsequent refresh should advance the revision again.
    comp.refresh(arc.clone(), sid.clone(), Some(wid.clone())).await.expect("second refresh ok");
    assert_eq!(comp.latest_revision(), 2);

    // Verify tiny metadata projection populated from the application read-path.
    let meta = comp.latest_metadata().expect("metadata present");
    assert_eq!(meta.session_id.unwrap(), sid);
    assert_eq!(meta.workspace_id.unwrap(), wid);
    assert_eq!(meta.active_buffer.unwrap(), crate::ports::BufferId::from("buf:fake"));
    assert_eq!(meta.opened_buffer_count, 1);

    // New: verify active-buffer details projection is populated and consistent
    let abd = comp.latest_active_buffer_details().expect("active buffer details present");
    assert_eq!(abd.buffer_id, crate::ports::BufferId::from("buf:fake"));
    assert_eq!(abd.line_count, 1);
    assert_eq!(abd.display.unwrap(), "fake".to_string());

    // Status snapshot must be present and reflect available projections.
    let status = comp.latest_status().expect("status present");
    assert!(status.has_render_window, "presenter window should be available after refresh");
    assert!(status.has_metadata, "metadata should be present after refresh");
    assert!(status.has_active_buffer_details, "active buffer details should be present");
    assert!(status.has_opened_buffers, "opened buffers projection should be non-empty");
    assert!(!status.has_ai_projection, "AI projection should not be present in this path");
}

#[tokio::test]
async fn desktop_composition_ai_projection_refreshes() {
    use std::sync::Arc;
    use uuid::Uuid;
    use chrono::Utc;

    // Build a fake view (re-use test helper above)
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let wid = zaroxi_kernel_types::Id::new();

    // Minimal fake service that returns a single opened buffer and a single ExplainExecuted event.
    struct FakeSvc {
        buf: crate::ports::BufferId,
        wid: zaroxi_kernel_types::Id,
    }

    impl FakeSvc {
        fn new(buf: crate::ports::BufferId, wid: zaroxi_kernel_types::Id) -> Self {
            Self { buf, wid }
        }
    }

    impl crate::ports::WorkspaceService for FakeSvc {
        fn boot_workspace(&self, _req: crate::ports::WorkspaceBootRequest) -> crate::BoxFuture<'static, Result<crate::ports::WorkspaceBootResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(&self, _req: crate::ports::OpenBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::OpenBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(&self, _req: crate::ports::ListBuffersRequest) -> crate::BoxFuture<'static, Result<crate::ports::ListBuffersResponse, crate::ports::UseCaseError>> {
            let b = self.buf.clone();
            Box::pin(async move { Ok(crate::ports::ListBuffersResponse { buffer_ids: vec![b], active_buffer: Some(crate::ports::BufferId::from("buf:fake")) }) })
        }
        fn set_active_buffer(&self, _req: crate::ports::SetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>> {
            let bid = self.buf.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveBufferResponse { buffer_id: bid }) })
        }
        fn set_editor_cursor(&self, _req: crate::ports::SetEditorCursorRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(&self, _req: crate::ports::SetSelectionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetSelectionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(&self, _req: crate::ports::ClearSelectionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ClearSelectionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(&self, _req: crate::ports::GetEditorStateRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetEditorStateResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(&self, _req: crate::ports::SetViewportRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetViewportResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(&self, _req: crate::ports::ScrollViewportRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ScrollViewportResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(&self, _req: crate::ports::DispatchCommandRequest) -> crate::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(&self, _req: crate::ports::UpdateBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::UpdateBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(&self, _req: crate::ports::ApplyTextTransactionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Ok(crate::ports::ApplyTextTransactionResponse { ok: true, state: crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: crate::ports::GetRecentCommandsRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetRecentCommandsResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Ok(crate::ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }

        fn get_recent_events(&self, req: crate::ports::GetRecentEventsRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetRecentEventsResponse, crate::ports::UseCaseError>> {
            let buf = self.buf.clone();
            let wid = self.wid.clone();
            Box::pin(async move {
                let ev = crate::ports::WorkspaceEvent {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    session_id: req.session_id.clone(),
                    workspace_id: wid,
                    kind: crate::ports::WorkspaceEventKind::ExplainExecuted { buffer_id: buf.clone(), result: "mocked explain".to_string() },
                };
                Ok(crate::ports::GetRecentEventsResponse { events: vec![ev] })
            })
        }

        fn get_session_snapshot(&self, _req: crate::ports::GetSessionSnapshotRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetSessionSnapshotResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn create_checkpoint(&self, _req: crate::ports::CreateCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::CreateCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn save_checkpoint(&self, _req: crate::ports::SaveCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SaveCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(&self, _req: crate::ports::LoadCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::LoadCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(&self, _req: crate::ports::RestoreCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::RestoreCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
    }

    let fake_service = std::sync::Arc::new(FakeSvc::new(crate::ports::BufferId::from("buf:fake"), wid.clone())) as std::sync::Arc<dyn crate::ports::WorkspaceService>;

    let mut comp = DesktopComposition::new();
    // Use refresh_with_service so the composition will consult the fake service and recent events.
    comp.refresh_with_service(arc, sid.clone(), Some(wid.clone()), Some(fake_service)).await.expect("refresh ok");

    // Revision should have advanced from initial 0 to 1 after the refresh with service.
    assert_eq!(comp.latest_revision(), 1);

    let meta = comp.latest_metadata().expect("metadata present");
    assert!(meta.ai_projection.is_some(), "ai projection should be present from recent events");
    let ai = meta.ai_projection.unwrap();
    assert_eq!(ai.result.unwrap(), "mocked explain".to_string());
    assert_eq!(ai.target_buffer.unwrap(), crate::ports::BufferId::from("buf:fake"));

    // Ensure the composition recorded that the AI projection was updated.
    let rr = comp.latest_refresh_reason().expect("reason present");
    assert_eq!(rr, RefreshReason::AiProjectionUpdated);

    // Status snapshot must be present and reflect AI projection availability.
    let status = comp.latest_status().expect("status present");
    assert!(status.has_render_window, "presenter window should be available after refresh");
    assert!(status.has_metadata, "metadata should be present after refresh");
    // active buffer details available in this test too
    assert!(status.has_active_buffer_details, "active buffer details should be present");
    assert!(status.has_opened_buffers, "opened buffers projection should be non-empty");
    assert!(status.has_ai_projection, "AI projection should be reported present");
}

#[tokio::test]
async fn latest_summary_reflects_composition_state() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let wid = zaroxi_kernel_types::Id::new();

    let mut comp = DesktopComposition::new();
    comp.refresh(arc.clone(), sid.clone(), Some(wid.clone())).await.expect("refresh ok");

    let summary = comp.latest_summary().expect("summary present");
    assert_eq!(summary.revision, comp.latest_revision());
    assert_eq!(summary.refresh_reason, comp.latest_refresh_reason());
    let status = comp.latest_status().expect("status present");
    assert!(summary.status.is_some());
    assert_eq!(summary.status.unwrap().has_render_window, status.has_render_window);
    assert_eq!(summary.active_buffer, comp.latest_metadata().and_then(|m| m.active_buffer));
}

#[tokio::test]
async fn desktop_composition_consistency_report_is_valid() {
    use std::sync::Arc;
    use uuid::Uuid;
    use chrono::Utc;

    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let wid = zaroxi_kernel_types::Id::new();

    // Minimal fake service that returns a single opened buffer and a single ExplainExecuted event.
    struct FakeSvc {
        buf: crate::ports::BufferId,
        wid: zaroxi_kernel_types::Id,
    }

    impl FakeSvc {
        fn new(buf: crate::ports::BufferId, wid: zaroxi_kernel_types::Id) -> Self {
            Self { buf, wid }
        }
    }

    impl crate::ports::WorkspaceService for FakeSvc {
        fn boot_workspace(&self, _req: crate::ports::WorkspaceBootRequest) -> crate::BoxFuture<'static, Result<crate::ports::WorkspaceBootResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(&self, _req: crate::ports::OpenBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::OpenBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(&self, _req: crate::ports::ListBuffersRequest) -> crate::BoxFuture<'static, Result<crate::ports::ListBuffersResponse, crate::ports::UseCaseError>> {
            let b = self.buf.clone();
            Box::pin(async move { Ok(crate::ports::ListBuffersResponse { buffer_ids: vec![b], active_buffer: Some(crate::ports::BufferId::from("buf:fake")) }) })
        }
        fn set_active_buffer(&self, _req: crate::ports::SetActiveBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>> {
            let bid = self.buf.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveBufferResponse { buffer_id: bid }) })
        }
        fn set_editor_cursor(&self, _req: crate::ports::SetEditorCursorRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(&self, _req: crate::ports::SetSelectionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetSelectionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(&self, _req: crate::ports::ClearSelectionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ClearSelectionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(&self, _req: crate::ports::GetEditorStateRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetEditorStateResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(&self, _req: crate::ports::SetViewportRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetViewportResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(&self, _req: crate::ports::ScrollViewportRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ScrollViewportResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(&self, _req: crate::ports::DispatchCommandRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(&self, _req: crate::ports::UpdateBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::UpdateBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(&self, _req: crate::ports::ApplyTextTransactionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Ok(crate::ports::ApplyTextTransactionResponse { ok: true, state: crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: crate::ports::GetRecentCommandsRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetRecentCommandsResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Ok(crate::ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }

        fn get_recent_events(&self, req: crate::ports::GetRecentEventsRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetRecentEventsResponse, crate::ports::UseCaseError>> {
            let buf = self.buf.clone();
            let wid = self.wid.clone();
            Box::pin(async move {
                let ev = crate::ports::WorkspaceEvent {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    session_id: req.session_id.clone(),
                    workspace_id: wid,
                    kind: crate::ports::WorkspaceEventKind::ExplainExecuted { buffer_id: buf.clone(), result: "ctx-explain".to_string() },
                };
                Ok(crate::ports::GetRecentEventsResponse { events: vec![ev] })
            })
        }

        fn get_session_snapshot(&self, _req: crate::ports::GetSessionSnapshotRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetSessionSnapshotResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn create_checkpoint(&self, _req: crate::ports::CreateCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::CreateCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn save_checkpoint(&self, _req: crate::ports::SaveCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SaveCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(&self, _req: crate::ports::LoadCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::LoadCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(&self, _req: crate::ports::RestoreCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::RestoreCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
    }

    let fake_service = std::sync::Arc::new(FakeSvc::new(crate::ports::BufferId::from("buf:fake"), wid.clone())) as std::sync::Arc<dyn crate::ports::WorkspaceService>;

    let mut comp = DesktopComposition::new();
    // Use refresh_with_service so the composition will consult the fake service and recent events.
    comp.refresh_with_service(arc, sid.clone(), Some(wid.clone()), Some(fake_service)).await.expect("refresh ok");

    let report = comp.latest_consistency_report();
    assert!(report.overall_ok, "consistency report should be OK in this basic happy path");
    assert!(report.status_present_matches_summary);
    assert!(report.active_buffer_matches_details);
    assert!(report.active_buffer_in_opened_buffers);
    assert!(report.presenter_window_matches_status);
}

#[tokio::test]
async fn latest_shell_context_is_composed() {
    use std::sync::Arc;
    use uuid::Uuid;
    use chrono::Utc;

    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let wid = zaroxi_kernel_types::Id::new();

    // Minimal fake service that returns a single opened buffer and a single ExplainExecuted event.
    struct FakeSvc {
        buf: crate::ports::BufferId,
        wid: zaroxi_kernel_types::Id,
    }

    impl FakeSvc {
        fn new(buf: crate::ports::BufferId, wid: zaroxi_kernel_types::Id) -> Self {
            Self { buf, wid }
        }
    }

    impl crate::ports::WorkspaceService for FakeSvc {
        fn boot_workspace(&self, _req: crate::ports::WorkspaceBootRequest) -> crate::BoxFuture<'static, Result<crate::ports::WorkspaceBootResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(&self, _req: crate::ports::OpenBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::OpenBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(&self, _req: crate::ports::ListBuffersRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ListBuffersResponse, crate::ports::UseCaseError>> {
            let b = self.buf.clone();
            Box::pin(async move { Ok(crate::ports::ListBuffersResponse { buffer_ids: vec![b], active_buffer: Some(crate::ports::BufferId::from("buf:fake")) }) })
        }
        fn set_active_buffer(&self, _req: crate::ports::SetActiveBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>> {
            let bid = self.buf.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveBufferResponse { buffer_id: bid }) })
        }
        fn set_editor_cursor(&self, _req: crate::ports::SetEditorCursorRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(&self, _req: crate::ports::SetSelectionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetSelectionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(&self, _req: crate::ports::ClearSelectionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ClearSelectionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(&self, _req: crate::ports::GetEditorStateRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetEditorStateResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(&self, _req: crate::ports::SetViewportRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SetViewportResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(&self, _req: crate::ports::ScrollViewportRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ScrollViewportResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(&self, _req: crate::ports::DispatchCommandRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(&self, _req: crate::ports::UpdateBufferRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::UpdateBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(&self, _req: crate::ports::ApplyTextTransactionRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Ok(crate::ports::ApplyTextTransactionResponse { ok: true, state: crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: crate::ports::GetRecentCommandsRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetRecentCommandsResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Ok(crate::ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }

        fn get_recent_events(&self, req: crate::ports::GetRecentEventsRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetRecentEventsResponse, crate::ports::UseCaseError>> {
            let buf = self.buf.clone();
            let wid = self.wid.clone();
            Box::pin(async move {
                let ev = crate::ports::WorkspaceEvent {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    session_id: req.session_id.clone(),
                    workspace_id: wid,
                    kind: crate::ports::WorkspaceEventKind::ExplainExecuted { buffer_id: buf.clone(), result: "ctx-explain".to_string() },
                };
                Ok(crate::ports::GetRecentEventsResponse { events: vec![ev] })
            })
        }

        fn get_session_snapshot(&self, _req: crate::ports::GetSessionSnapshotRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetSessionSnapshotResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn create_checkpoint(&self, _req: crate::ports::CreateCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::CreateCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn save_checkpoint(&self, _req: crate::ports::SaveCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::SaveCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(&self, _req: crate::ports::LoadCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::LoadCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(&self, _req: crate::ports::RestoreCheckpointRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::RestoreCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
    }

    let fake_service = std::sync::Arc::new(FakeSvc::new(crate::ports::BufferId::from("buf:fake"), wid.clone())) as std::sync::Arc<dyn crate::ports::WorkspaceService>;

    let mut comp = DesktopComposition::new();
    // Use refresh_with_service so the composition will consult the fake service and recent events.
    comp.refresh_with_service(arc, sid.clone(), Some(wid.clone()), Some(fake_service)).await.expect("refresh ok");

    let ctx = comp.latest_shell_context().expect("context present");
    assert_eq!(ctx.latest_revision, comp.latest_revision());
    assert_eq!(ctx.active_buffer.unwrap(), crate::ports::BufferId::from("buf:fake"));
    assert_eq!(ctx.active_display.unwrap(), "fake".to_string());
    assert_eq!(ctx.latest_refresh_reason.unwrap(), RefreshReason::AiProjectionUpdated);
    assert!(ctx.has_ai_projection);
}

#[tokio::test]
async fn latest_window_contains_no_inline_marker_text() {
    use std::sync::Arc;
    use zaroxi_application_workspace::ports::{WorkspaceView, SessionId};

    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let wid = zaroxi_kernel_types::Id::new();

    let mut comp = DesktopComposition::new();
    comp.refresh(arc.clone(), sid.clone(), Some(wid.clone())).await.expect("refresh ok");

    // Ensure the shell-facing latest_window text does not contain inline marker tokens
    let win = comp.latest_window().expect("window present");
    for line in win.lines.iter() {
        let mut reconstructed = String::new();
        for sp in line.spans.iter() {
            reconstructed.push_str(&sp.text);
        }
        assert!(!reconstructed.contains("|^|"), "visible line must not contain cursor marker");
        assert!(!reconstructed.contains("|/|/"), "visible line must not contain debug marker");
    }
}
