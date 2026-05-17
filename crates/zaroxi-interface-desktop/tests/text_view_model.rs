use std::sync::{Arc, Mutex};
use std::pin::Pin;
use std::future::Future;

use zaroxi_interface_desktop::{DesktopComposition, TextView, actions};
use zaroxi_application_workspace::ports::{WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, GetActiveEditorDocumentResponse, GetVisibleLinesResponse, EditorDocument, EditorCursor};
use zaroxi_application_workspace::ports as aw;
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

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
            cursor,
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
    fn set_active_buffer(&self, _req: crate::ports::SetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>> {
        Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
    }
    fn get_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>> {
        let bid = self.shared.doc.lock().unwrap().buffer_id.clone();
        Box::pin(async move { Ok(crate::ports::GetActiveBufferResponse { buffer_id: bid }) })
    }

    fn set_editor_cursor(&self, req: crate::ports::SetEditorCursorRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>> {
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
            Ok(crate::ports::SetEditorCursorResponse { ok: true })
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

    fn apply_text_transaction(&self, req: crate::ports::ApplyTextTransactionRequest) -> crate::BoxFuture<'static, Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>> {
        let shared = self.shared.clone();
        Box::pin(async move {
            // Very small simulated mutation: if Insert at index 0 with "\n", insert a blank first line.
            match &req.transaction {
                crate::ports::TextEdit::Insert { index: 0, text } if text == "\n" => {
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
                    Ok(crate::ports::ApplyTextTransactionResponse { ok: true, state: crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None }, content: shared.doc.lock().unwrap().content.clone() })
                }
                _ => {
                    // No-op success
                    Ok(crate::ports::ApplyTextTransactionResponse { ok: true, state: crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None }, content: shared.doc.lock().unwrap().content.clone() })
                }
            }
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
}

#[tokio::test]
async fn text_view_reflects_cursor_move_and_insert() {
    let shared = Arc::new(SharedState::new("abcd", EditorCursor { line: 0, column: 2 }));
    let v = FakeView::new(shared.clone());
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());

    let svc = Arc::new(FakeService::new(shared.clone())) as Arc<dyn zaroxi_application_workspace::ports::WorkspaceService>;
    let mut comp = DesktopComposition::new();

    // Initial refresh
    comp.refresh_with_service(arc.clone(), sid.clone(), None, Some(svc.clone())).await.expect("initial refresh ok");
    let tv1 = TextView::from_composition(&comp).expect("tv present");
    assert_eq!(tv1.lines.len(), 1);
    assert_eq!(tv1.cursor_line, Some(1));
    assert_eq!(tv1.cursor_column, Some(2));

    // Move cursor -> set_editor_cursor on service will update shared state and refresh consumes it.
    let res = actions::move_cursor_to_start_and_refresh(&mut comp, svc.clone(), arc.clone(), sid.clone(), None).await.expect("move ok");
    assert!(res.success);
    let tv2 = TextView::from_composition(&comp).expect("tv present after move");
    assert_eq!(tv2.cursor_line, Some(1));
    assert_eq!(tv2.cursor_column, Some(0));

    // Insert line at start -> service.apply_text_transaction will mutate shared content and window.
    let res2 = actions::insert_line_at_start_and_refresh(&mut comp, svc.clone(), arc.clone(), sid.clone(), None).await.expect("insert ok");
    assert!(res2.success);
    let tv3 = TextView::from_composition(&comp).expect("tv present after insert");
    // After inserting a leading newline the first visible line should be empty.
    assert!(tv3.lines.len() >= 1);
    assert_eq!(tv3.lines[0], "");
}
