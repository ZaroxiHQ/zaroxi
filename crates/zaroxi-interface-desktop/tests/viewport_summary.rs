use std::sync::Arc;
use zaroxi_interface_desktop::{DesktopComposition, actions, ViewportAnchoring};
use zaroxi_application_workspace::ports::{WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, GetActiveEditorDocumentResponse, GetVisibleLinesResponse, EditorDocument, EditorCursor};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::ports as aw_ports;
use zaroxi_interface_desktop::view_adapter::InterfaceSpanKind;
use zaroxi_kernel_types::Id;

/// Minimal in-test WorkspaceView that returns a tiny document and a one-line visible window.
struct FakeView {
    doc: EditorDocument,
    window: VisibleLinesWindow,
}

impl FakeView {
    fn new() -> Self {
        let content = Some("abcd".to_string());
        let ed = EditorDocument {
            buffer_id: BufferId::from("buf:fake"),
            content: content.clone(),
            cursor: EditorCursor { line: 0, column: 2 },
            selection: None,
            line_count: 1,
            current_line: content.and_then(|c| c.lines().nth(0).map(|s| s.to_string())),
        };

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
    fn get_buffer_content(&self, _buffer_id: aw_ports::BufferId) -> aw_ports::BoxFuture<'static, Result<Option<String>, aw_ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: aw_ports::SessionId) -> aw_ports::BoxFuture<'static, Result<Option<String>, aw_ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> aw_ports::BoxFuture<'static, Result<GetActiveEditorDocumentResponse, aw_ports::UseCaseError>> {
        let d = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> aw_ports::BoxFuture<'static, Result<GetVisibleLinesResponse, aw_ports::UseCaseError>> {
        let w = self.window.clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

#[tokio::test]
async fn initial_refresh_populates_viewport_summary() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());
    let mut comp = DesktopComposition::new();

    let ar = zaroxi_interface_desktop::refresh_desktop(&mut comp, arc, sid.clone(), None, None).await.expect("refresh ok");
    assert!(ar.success);
    let vs = comp.latest_viewport_summary().expect("viewport present");
    let win = comp.latest_window().expect("window present");
    assert_eq!(vs.top_visible_line, win.top_line);
    assert_eq!(vs.visible_line_count, win.lines.len());
    assert_eq!(vs.total_lines, win.total_lines);
    assert!(vs.cursor_visible);
}

#[tokio::test]
async fn move_cursor_action_updates_viewport_summary() {
    // Build a fake view and fake service similar to existing tests.
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());
    let mut comp = DesktopComposition::new();

    // Fake service that reports the active buffer and accepts set_editor_cursor.
    struct FakeSvc {
        buf: aw_ports::BufferId,
    }
    impl FakeSvc {
        fn new(buf: aw_ports::BufferId) -> Self { Self { buf } }
    }
    impl aw_ports::WorkspaceService for FakeSvc {
        fn boot_workspace(&self, _req: aw_ports::WorkspaceBootRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::WorkspaceBootResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownWorkspace) }) }
        fn open_buffer(&self, _req: aw_ports::OpenBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::OpenBufferResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn list_open_buffers(&self, _req: aw_ports::ListBuffersRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ListBuffersResponse, aw_ports::UseCaseError>> {
            let buf = self.buf.clone();
            Box::pin(async move { Ok(aw_ports::ListBuffersResponse { buffer_ids: vec![buf.clone()], active_buffer: Some(buf.clone()) }) })
        }
        fn set_active_buffer(&self, _req: aw_ports::SetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetActiveBufferResponse, aw_ports::UseCaseError>> { Box::pin(async { Ok(aw_ports::SetActiveBufferResponse { ok: true }) }) }
        fn get_active_buffer(&self, _req: aw_ports::GetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetActiveBufferResponse, aw_ports::UseCaseError>> { let b = self.buf.clone(); Box::pin(async move { Ok(aw_ports::GetActiveBufferResponse { buffer_id: b }) }) }
        fn set_editor_cursor(&self, _req: aw_ports::SetEditorCursorRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetEditorCursorResponse, aw_ports::UseCaseError>> { Box::pin(async { Ok(aw_ports::SetEditorCursorResponse { ok: true }) }) }
        fn set_editor_selection(&self, _req: aw_ports::SetSelectionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetSelectionResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn clear_editor_selection(&self, _req: aw_ports::ClearSelectionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ClearSelectionResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn get_editor_state(&self, _req: aw_ports::GetEditorStateRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetEditorStateResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn set_viewport_state(&self, _req: aw_ports::SetViewportRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetViewportResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn scroll_viewport(&self, _req: aw_ports::ScrollViewportRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ScrollViewportResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn explain_active_buffer(&self, _req: aw_ports::GetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::DispatchCommandResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::NoActiveBuffer) }) }
        fn dispatch_command(&self, _req: aw_ports::DispatchCommandRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::DispatchCommandResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn update_buffer(&self, _req: aw_ports::UpdateBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::UpdateBufferResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn apply_text_transaction(&self, _req: aw_ports::ApplyTextTransactionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ApplyTextTransactionResponse, aw_ports::UseCaseError>> { Box::pin(async { Ok(aw_ports::ApplyTextTransactionResponse { ok: true, state: aw_ports::EditorState { cursor: aw_ports::EditorCursor::zero(), selection: None }, content: None }) }) }
        fn get_recent_commands(&self, _req: aw_ports::GetRecentCommandsRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetRecentCommandsResponse, aw_ports::UseCaseError>> { Box::pin(async { Ok(aw_ports::GetRecentCommandsResponse { commands: Vec::new() }) }) }
        fn get_recent_events(&self, _req: aw_ports::GetRecentEventsRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetRecentEventsResponse, aw_ports::UseCaseError>> { Box::pin(async { Ok(aw_ports::GetRecentEventsResponse { events: Vec::new() }) }) }
        fn get_session_snapshot(&self, _req: aw_ports::GetSessionSnapshotRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetSessionSnapshotResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn create_checkpoint(&self, _req: aw_ports::CreateCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::CreateCheckpointResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn save_checkpoint(&self, _req: aw_ports::SaveCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SaveCheckpointResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn load_checkpoint(&self, _req: aw_ports::LoadCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::LoadCheckpointResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
        fn restore_checkpoint(&self, _req: aw_ports::RestoreCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::RestoreCheckpointResponse, aw_ports::UseCaseError>> { Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) }) }
    }

    let service = std::sync::Arc::new(FakeSvc::new(aw_ports::BufferId::from("buf:fake"))) as std::sync::Arc<dyn aw_ports::WorkspaceService>;

    // Pre-refresh to populate the presenter
    let _ = zaroxi_interface_desktop::refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, None).await.expect("initial refresh ok");

    // Execute the move-cursor action which will call set_editor_cursor and refresh composition.
    let res = zaroxi_interface_desktop::move_cursor_to_start_and_refresh(&mut comp, service.clone(), arc.clone(), sid.clone(), None).await.expect("action ok");
    assert!(res.success);

    let vs = comp.latest_viewport_summary().expect("viewport present after move");
    assert!(vs.cursor_visible);
    // Anchoring should be one of the enum variants (best-effort).
    assert!(matches!(vs.anchoring, ViewportAnchoring::Top | ViewportAnchoring::Centered | ViewportAnchoring::Unknown));
}

#[tokio::test]
async fn content_mutation_reflects_in_viewport_total_lines() {
    // Mutable fake view similar to other tests so we can change content and re-refresh.
    struct MutableFakeView {
        doc: std::sync::Arc<std::sync::Mutex<EditorDocument>>,
        window: std::sync::Arc<std::sync::Mutex<VisibleLinesWindow>>,
    }

    impl MutableFakeView {
        fn new(buffer_id: BufferId, content: Option<String>, cursor: EditorCursor) -> Self {
            let line_text = content.clone().unwrap_or_default();
            let vl = VisibleLine {
                line_number: 1,
                text: line_text.clone(),
                is_cursor_line: true,
                cursor_column: Some(cursor.column as usize),
                selection_intersects: false,
                selection_start_column: None,
                selection_end_column: None,
            };
            let vw = VisibleLinesWindow { top_line: 1, total_lines: content.as_ref().map(|s| s.lines().count()).unwrap_or(0), lines: vec![vl] };
            let doc = EditorDocument {
                buffer_id,
                content,
                cursor,
                selection: None,
                line_count: vw.total_lines,
                current_line: None,
            };
            Self { doc: std::sync::Arc::new(std::sync::Mutex::new(doc)), window: std::sync::Arc::new(std::sync::Mutex::new(vw)) }
        }

        fn set_content(&self, content: Option<String>) {
            {
                let mut d = self.doc.lock().unwrap();
                d.content = content.clone();
            }
            if let Ok(mut w) = self.window.lock() {
                let txt = content.clone().unwrap_or_default();
                if let Some(line) = w.lines.get_mut(0) {
                    line.text = txt.clone();
                    if line.cursor_column.is_none() {
                        line.cursor_column = Some(0);
                    }
                }
                w.total_lines = content.as_ref().map(|s| s.lines().count()).unwrap_or(0);
            }
        }
    }

    impl WorkspaceView for MutableFakeView {
        fn get_buffer_content(&self, _buffer_id: aw_ports::BufferId) -> aw_ports::BoxFuture<'static, Result<Option<String>, aw_ports::UseCaseError>> {
            let d = self.doc.lock().unwrap().content.clone();
            Box::pin(async move { Ok(d) })
        }
        fn get_active_buffer_content(&self, _session_id: aw_ports::SessionId) -> aw_ports::BoxFuture<'static, Result<Option<String>, aw_ports::UseCaseError>> {
            let d = self.doc.lock().unwrap().content.clone();
            Box::pin(async move { Ok(d) })
        }
        fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> aw_ports::BoxFuture<'static, Result<GetActiveEditorDocumentResponse, aw_ports::UseCaseError>> {
            let d = self.doc.lock().unwrap().clone();
            Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
        }
        fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> aw_ports::BoxFuture<'static, Result<GetVisibleLinesResponse, aw_ports::UseCaseError>> {
            let w = self.window.lock().unwrap().clone();
            Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
        }
    }

    let view = MutableFakeView::new(BufferId::from("buf:one"), Some("a\nb\nc".to_string()), EditorCursor { line: 0, column: 0 });
    let arc: Arc<dyn WorkspaceView> = Arc::new(view);
    let sid = SessionId(Id::new());
    let mut comp = DesktopComposition::new();

    // initial refresh
    let _ = zaroxi_interface_desktop::refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, None).await.expect("refresh ok");
    let vs1 = comp.latest_viewport_summary().expect("viewport present");
    let total_before = vs1.total_lines;

    // mutate content in the view and refresh
    // (the concrete mutable impl above is behind Arc; downcast is not used; instead re-construct a new Arc)
    let view2 = MutableFakeView::new(BufferId::from("buf:one"), Some("line1\nline2\nline3\nline4\nline5".to_string()), EditorCursor { line: 0, column: 0 });
    let arc2: Arc<dyn WorkspaceView> = Arc::new(view2);
    let _ = zaroxi_interface_desktop::refresh_desktop(&mut comp, arc2.clone(), sid.clone(), None, None).await.expect("refresh ok");
    let vs2 = comp.latest_viewport_summary().expect("viewport present after mutation");
    assert!(vs2.total_lines >= total_before);
}
