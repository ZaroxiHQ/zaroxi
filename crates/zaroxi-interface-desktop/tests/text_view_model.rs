use std::sync::{Arc, Mutex};

use zaroxi_interface_desktop::TextView;
use zaroxi_application_workspace::ports::{WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, GetActiveEditorDocumentResponse, GetVisibleLinesResponse, EditorDocument, EditorCursor};
use zaroxi_application_workspace::ports as aw;
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;

/// Shared in-test state: editor document + visible window that can be updated by the fake service.
struct SharedState {
    doc: Mutex<EditorDocument>,
    window: Mutex<VisibleLinesWindow>,
}

impl SharedState {
    fn new(initial_content: &str, cursor: EditorCursor) -> Self {
        let content = Some(initial_content.to_string());
        let ed = EditorDocument {
            buffer_id: BufferId::from("buf:fake"),
            content: content.clone(),
            cursor: cursor.clone(),
            selection: None,
            line_count: initial_content.lines().count(),
            current_line: content.and_then(|c| c.lines().nth(0).map(|s| s.to_string())),
        };
        let vl = VisibleLine {
            line_number: 1,
            text: initial_content.to_string(),
            is_cursor_line: true,
            cursor_column: Some(cursor.column as usize),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = VisibleLinesWindow { top_line: 1, total_lines: initial_content.lines().count(), lines: vec![vl] };
        SharedState { doc: Mutex::new(ed), window: Mutex::new(vw) }
    }
}

/// Fake view that reads from SharedState.
struct FakeView {
    shared: Arc<SharedState>,
}

impl FakeView {
    fn new(shared: Arc<SharedState>) -> Self {
        Self { shared }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(&self, _buffer_id: aw::BufferId) -> aw::BoxFuture<'static, Result<Option<String>, aw::UseCaseError>> {
        let s = self.shared.doc.lock().unwrap().content.clone();
        Box::pin(async move { Ok(s) })
    }

    fn get_active_buffer_content(&self, _session_id: aw::SessionId) -> aw::BoxFuture<'static, Result<Option<String>, aw::UseCaseError>> {
        let s = self.shared.doc.lock().unwrap().content.clone();
        Box::pin(async move { Ok(s) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> aw::BoxFuture<'static, Result<GetActiveEditorDocumentResponse, aw::UseCaseError>> {
        let d = self.shared.doc.lock().unwrap().clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> aw::BoxFuture<'static, Result<GetVisibleLinesResponse, aw::UseCaseError>> {
        let w = self.shared.window.lock().unwrap().clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

/// Fake service that mutates the shared state when set_editor_cursor or apply_text_transaction is called.
struct FakeService {
    shared: Arc<SharedState>,
}

impl FakeService {
    fn new(shared: Arc<SharedState>) -> Self {
        Self { shared }
    }
}

impl zaroxi_application_workspace::ports::WorkspaceService for FakeService {
    fn boot_workspace(&self, _req: aw::WorkspaceBootRequest) -> aw::BoxFuture<'static, Result<aw::WorkspaceBootResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownWorkspace) })
    }
    fn open_buffer(&self, _req: aw::OpenBufferRequest) -> aw::BoxFuture<'static, Result<aw::OpenBufferResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
    fn list_open_buffers(&self, _req: aw::ListBuffersRequest) -> aw::BoxFuture<'static, Result<aw::ListBuffersResponse, aw::UseCaseError>> {
        let b = self.shared.doc.lock().unwrap().buffer_id.clone();
        Box::pin(async move { Ok(aw::ListBuffersResponse { buffer_ids: vec![b], active_buffer: Some(aw::BufferId::from("buf:fake")) }) })
    }
    fn set_active_buffer(&self, _req: aw::SetActiveBufferRequest) -> aw::BoxFuture<'static, Result<aw::SetActiveBufferResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
    fn get_active_buffer(&self, _req: aw::GetActiveBufferRequest) -> aw::BoxFuture<'static, Result<aw::GetActiveBufferResponse, aw::UseCaseError>> {
        let bid = self.shared.doc.lock().unwrap().buffer_id.clone();
        Box::pin(async move { Ok(aw::GetActiveBufferResponse { buffer_id: bid }) })
    }

    fn set_editor_cursor(&self, req: aw::SetEditorCursorRequest) -> aw::BoxFuture<'static, Result<aw::SetEditorCursorResponse, aw::UseCaseError>> {
        let shared = self.shared.clone();
        Box::pin(async move {
            // Update the shared editor document and visible window to reflect the new cursor.
            {
                let mut doc = shared.doc.lock().unwrap();
                doc.cursor = req.cursor.clone();
                doc.current_line = doc.content.as_ref().and_then(|c| c.lines().nth(doc.cursor.line as usize).map(|s| s.to_string()));
            }
            {
                let mut win = shared.window.lock().unwrap();
                if let Some(line) = win.lines.get_mut(0) {
                    line.is_cursor_line = true;
                    line.cursor_column = Some(req.cursor.column as usize);
                }
            }
            Ok(aw::SetEditorCursorResponse { ok: true })
        })
    }

    fn set_editor_selection(&self, _req: aw::SetSelectionRequest) -> aw::BoxFuture<'static, Result<aw::SetSelectionResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
    fn clear_editor_selection(&self, _req: aw::ClearSelectionRequest) -> aw::BoxFuture<'static, Result<aw::ClearSelectionResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
    fn get_editor_state(&self, _req: aw::GetEditorStateRequest) -> aw::BoxFuture<'static, Result<aw::GetEditorStateResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
    fn set_viewport_state(&self, _req: aw::SetViewportRequest) -> aw::BoxFuture<'static, Result<aw::SetViewportResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
    fn scroll_viewport(&self, _req: aw::ScrollViewportRequest) -> aw::BoxFuture<'static, Result<aw::ScrollViewportResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
    fn explain_active_buffer(&self, _req: aw::GetActiveBufferRequest) -> aw::BoxFuture<'static, Result<aw::DispatchCommandResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::NoActiveBuffer) })
    }
    fn dispatch_command(&self, _req: aw::DispatchCommandRequest) -> aw::BoxFuture<'static, Result<aw::DispatchCommandResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
    fn update_buffer(&self, _req: aw::UpdateBufferRequest) -> aw::BoxFuture<'static, Result<aw::UpdateBufferResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }

    fn apply_text_transaction(&self, req: aw::ApplyTextTransactionRequest) -> aw::BoxFuture<'static, Result<aw::ApplyTextTransactionResponse, aw::UseCaseError>> {
        let shared = self.shared.clone();
        Box::pin(async move {
            // Very small simulated mutation: if Insert at index 0 with "\n", insert a blank first line.
            match &req.transaction {
                aw::TextEdit::Insert { index: 0, text } if text == "\n" => {
                    // Mutate document content and update visible window for the test.
                    {
                        let mut doc = shared.doc.lock().unwrap();
                        let old = doc.content.take().unwrap_or_default();
                        let new = format!("\n{}", old);
                        doc.content = Some(new.clone());
                        doc.line_count = new.lines().count();
                        doc.current_line = doc.content.as_ref().and_then(|c| c.lines().nth(doc.cursor.line as usize).map(|s| s.to_string()));
                    }
                    {
                        let mut win = shared.window.lock().unwrap();
                        // Rebuild window lines from updated content (simple, split lines).
                        win.lines.clear();
                        if let Some(ref content) = shared.doc.lock().unwrap().content {
                            for (i, l) in content.lines().enumerate() {
                                let vl = VisibleLine {
                                    line_number: i + 1,
                                    text: l.to_string(),
                                    is_cursor_line: i == (shared.doc.lock().unwrap().cursor.line as usize),
                                    cursor_column: if i == (shared.doc.lock().unwrap().cursor.line as usize) { Some(shared.doc.lock().unwrap().cursor.column as usize) } else { None },
                                    selection_intersects: false,
                                    selection_start_column: None,
                                    selection_end_column: None,
                                };
                                win.lines.push(vl);
                            }
                            win.top_line = 1;
                            win.total_lines = shared.doc.lock().unwrap().line_count;
                        }
                    }
                    Ok(aw::ApplyTextTransactionResponse { ok: true, state: aw::EditorState { cursor: aw::EditorCursor::zero(), selection: None }, content: shared.doc.lock().unwrap().content.clone() })
                }
                _ => {
                    // No-op success
                    Ok(aw::ApplyTextTransactionResponse { ok: true, state: aw::EditorState { cursor: aw::EditorCursor::zero(), selection: None }, content: shared.doc.lock().unwrap().content.clone() })
                }
            }
        })
    }

    fn get_recent_commands(&self, _req: aw::GetRecentCommandsRequest) -> aw::BoxFuture<'static, Result<aw::GetRecentCommandsResponse, aw::UseCaseError>> {
        Box::pin(async { Ok(aw::GetRecentCommandsResponse { commands: Vec::new() }) })
    }

    fn get_recent_events(&self, _req: aw::GetRecentEventsRequest) -> aw::BoxFuture<'static, Result<aw::GetRecentEventsResponse, aw::UseCaseError>> {
        Box::pin(async { Ok(aw::GetRecentEventsResponse { events: Vec::new() }) })
    }

    fn get_session_snapshot(&self, _req: aw::GetSessionSnapshotRequest) -> aw::BoxFuture<'static, Result<aw::GetSessionSnapshotResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }

    fn create_checkpoint(&self, _req: aw::CreateCheckpointRequest) -> aw::BoxFuture<'static, Result<aw::CreateCheckpointResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }

    fn save_checkpoint(&self, _req: aw::SaveCheckpointRequest) -> aw::BoxFuture<'static, Result<aw::SaveCheckpointResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }

    fn load_checkpoint(&self, _req: aw::LoadCheckpointRequest) -> aw::BoxFuture<'static, Result<aw::LoadCheckpointResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }

    fn restore_checkpoint(&self, _req: aw::RestoreCheckpointRequest) -> aw::BoxFuture<'static, Result<aw::RestoreCheckpointResponse, aw::UseCaseError>> {
        Box::pin(async { Err(aw::UseCaseError::UnknownSession) })
    }
}

#[tokio::test]
async fn text_view_reflects_cursor_move_and_insert() {
    let shared = Arc::new(SharedState::new("abcd", EditorCursor { line: 0, column: 2 }));
    let v = FakeView::new(shared.clone());
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());

    let svc = Arc::new(FakeService::new(shared.clone())) as Arc<dyn zaroxi_application_workspace::ports::WorkspaceService>;

    // Initial renderable window fetch (avoid using DesktopComposition.refresh which may
    // exercise more async interactions); build TextView directly from the adapter output.
    let win = zaroxi_interface_desktop::fetch_renderable_window(arc.clone(), sid.clone()).await.expect("fetch renderable window ok");
    let tv1 = TextView::from_window(&win).expect("tv present");
    assert_eq!(tv1.lines.len(), 1);
    assert_eq!(tv1.cursor_line, Some(1));
    assert_eq!(tv1.cursor_column, Some(2));

    // Move cursor -> set_editor_cursor on service will update shared state and refresh consumes it.
    // Simulate the move-cursor action by directly calling the service to update cursor,
    // then re-fetch the renderable window and build a fresh TextView for assertions.
    svc.set_editor_cursor(aw::SetEditorCursorRequest {
        session_id: sid.clone(),
        buffer_id: aw::BufferId::from("buf:fake"),
        cursor: aw::EditorCursor { line: 0, column: 0 },
    }).await.expect("set_editor_cursor ok");

    let win2 = zaroxi_interface_desktop::fetch_renderable_window(arc.clone(), sid.clone()).await.expect("fetch renderable window after move");
    let tv2 = TextView::from_window(&win2).expect("tv present after move");
    assert_eq!(tv2.cursor_line, Some(1));
    assert_eq!(tv2.cursor_column, Some(0));

    // Insert line at start -> service.apply_text_transaction will mutate shared content and window.
    // Simulate insert-line-at-start by applying a text transaction via the service,
    // then re-fetch the renderable window and build a fresh TextView for assertions.
    svc.apply_text_transaction(aw::ApplyTextTransactionRequest {
        session_id: sid.clone(),
        buffer_id: aw::BufferId::from("buf:fake"),
        transaction: aw::TextEdit::Insert { index: 0, text: "\n".to_string() },
    }).await.expect("apply_text_transaction ok");

    let win3 = zaroxi_interface_desktop::fetch_renderable_window(arc.clone(), sid.clone()).await.expect("fetch renderable window after insert");
    let tv3 = TextView::from_window(&win3).expect("tv present after insert");
    // After inserting a leading newline the first visible line should be empty.
    assert!(tv3.lines.len() >= 1);
    assert_eq!(tv3.lines[0], "");
}
