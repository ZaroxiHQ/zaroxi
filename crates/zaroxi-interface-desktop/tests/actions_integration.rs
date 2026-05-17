/*!
Interface-desktop action integration tests (Phase 18)

Rationale:
- Provide a small, focused integration-style test suite at the interface-desktop
  boundary that verifies the three tiny shell actions:
    - refresh_desktop
    - move_cursor_to_start_and_refresh
    - insert_line_at_start_and_refresh
- Tests exercise the public seam only. They use minimal in-test fakes that
  implement the application-level ports (WorkspaceView / WorkspaceService) so
  the adapter/presenter/composition pipeline is exercised end-to-end.
- Keep tests deterministic, small, and seam-focused (no infra wiring, no harness).
- This file intentionally lives in `crates/zaroxi-interface-desktop/tests` so the
  crate's public API is verified as an external consumer would use it.

Files added:
- crates/zaroxi-interface-desktop/tests/actions_integration.rs (this file)

Validation (run these from the workspace root):
# cargo test -p zaroxi-interface-desktop
# cargo run -p zaroxi-desktop-harness
# cargo test -p zaroxi-application-workspace
# cargo test -p zaroxi-application-ai
# bash scripts/architecture_check.sh

The shell commands are included here for convenience; run them after applying the test file.
*/

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc as StdArc;

use zaroxi_interface_desktop::{
    refresh_desktop, move_cursor_to_start_and_refresh, actions, DesktopComposition,
};
use zaroxi_interface_desktop::desktop::RefreshReason;
use zaroxi_application_workspace::ports::{
    WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId,
    GetActiveEditorDocumentResponse, GetVisibleLinesResponse, EditorDocument, EditorCursor,
    WorkspaceService, GetActiveBufferRequest, GetActiveBufferResponse, SetEditorCursorRequest,
    ApplyTextTransactionRequest, ApplyTextTransactionResponse, UseCaseError,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_kernel_types::Id;

// Bring the ports module into a local alias for unambiguous references in function signatures.
use zaroxi_application_workspace::ports as ports;

use std::pin::Pin;
use std::future::Future;
use std::boxed::Box;

/// Helper boxed future alias matching the application port signature used in tests.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Minimal in-test WorkspaceView that returns a tiny document and a one-line visible window.
///
/// This test-local fake is intentionally small and deterministic.
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
    fn get_buffer_content(&self, _buffer_id: ports::BufferId) -> BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: ports::SessionId) -> BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> BoxFuture<'static, Result<GetActiveEditorDocumentResponse, UseCaseError>> {
        let d = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> BoxFuture<'static, Result<GetVisibleLinesResponse, UseCaseError>> {
        let w = self.window.clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

/// Minimal fake WorkspaceService implementing only the tiny surface required by the actions.
///
/// Other methods return UnknownSession/UnknownWorkspace as appropriate to keep the fake small.
struct FakeService {
    buffer_id: BufferId,
    /// Track whether set_editor_cursor was invoked (test probe).
    set_called: StdArc<AtomicBool>,
    /// Track whether apply_text_transaction was invoked (test probe).
    apply_called: StdArc<AtomicBool>,
    /// Mutable opened buffer list (shared Arc so tests can mutate between refreshes).
    opened: StdArc<std::sync::Mutex<Vec<BufferId>>>,
    /// Mutable active buffer marker.
    active: StdArc<std::sync::Mutex<Option<BufferId>>>,
}

impl FakeService {
    fn new(buffer_id: BufferId) -> Self {
        // Clone early to avoid moving `buffer_id` before using clones for initializers.
        let v = vec![buffer_id.clone()];
        let active_init = Some(buffer_id.clone());
        Self {
            buffer_id,
            set_called: StdArc::new(AtomicBool::new(false)),
            apply_called: StdArc::new(AtomicBool::new(false)),
            opened: StdArc::new(std::sync::Mutex::new(v)),
            active: StdArc::new(std::sync::Mutex::new(active_init)),
        }
    }
}

impl ports::WorkspaceService for FakeService {
    fn boot_workspace(&self, _req: ports::WorkspaceBootRequest) -> BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) })
    }

    fn open_buffer(&self, _req: ports::OpenBufferRequest) -> BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn list_open_buffers(&self, _req: ports::ListBuffersRequest) -> BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>> {
        let opened = self.opened.clone();
        let active = self.active.clone();
        Box::pin(async move {
            let list = opened.lock().unwrap().clone();
            let act = active.lock().unwrap().clone();
            Ok(ports::ListBuffersResponse { buffer_ids: list, active_buffer: act })
        })
    }

    fn set_active_buffer(&self, _req: ports::SetActiveBufferRequest) -> BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn get_active_buffer(&self, _req: GetActiveBufferRequest) -> BoxFuture<'static, Result<GetActiveBufferResponse, ports::UseCaseError>> {
        let active = self.active.clone();
        Box::pin(async move {
            match active.lock().unwrap().clone() {
                Some(b) => Ok(GetActiveBufferResponse { buffer_id: b }),
                None => Err(ports::UseCaseError::NoActiveBuffer),
            }
        })
    }

    fn set_editor_cursor(&self, req: SetEditorCursorRequest) -> BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>> {
        let expected = self.buffer_id.clone();
        let set_called = self.set_called.clone();
        Box::pin(async move {
            if req.buffer_id == expected && req.cursor.line == 0 && req.cursor.column == 0 {
                set_called.store(true, Ordering::SeqCst);
                Ok(ports::SetEditorCursorResponse { ok: true })
            } else {
                Err(ports::UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()))
            }
        })
    }

    fn set_editor_selection(&self, _req: ports::SetSelectionRequest) -> BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn clear_editor_selection(&self, _req: ports::ClearSelectionRequest) -> BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn get_editor_state(&self, _req: ports::GetEditorStateRequest) -> BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn set_viewport_state(&self, _req: ports::SetViewportRequest) -> BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn scroll_viewport(&self, _req: ports::ScrollViewportRequest) -> BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn explain_active_buffer(&self, _req: ports::GetActiveBufferRequest) -> BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) })
    }

    fn dispatch_command(&self, _req: ports::DispatchCommandRequest) -> BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn update_buffer(&self, _req: ports::UpdateBufferRequest) -> BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn apply_text_transaction(&self, _req: ApplyTextTransactionRequest) -> BoxFuture<'static, Result<ApplyTextTransactionResponse, ports::UseCaseError>> {
        let apply_called = self.apply_called.clone();
        Box::pin(async move {
            apply_called.store(true, Ordering::SeqCst);
            Ok(ApplyTextTransactionResponse { ok: true, state: ports::EditorState { cursor: EditorCursor::zero(), selection: None }, content: None })
        })
    }

    fn get_recent_commands(&self, _req: ports::GetRecentCommandsRequest) -> BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>> {
        Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) })
    }

    fn get_recent_events(&self, _req: ports::GetRecentEventsRequest) -> BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>> {
        Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) })
    }

    fn get_session_snapshot(&self, _req: ports::GetSessionSnapshotRequest) -> BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn create_checkpoint(&self, _req: ports::CreateCheckpointRequest) -> BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn save_checkpoint(&self, _req: ports::SaveCheckpointRequest) -> BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn load_checkpoint(&self, _req: ports::LoadCheckpointRequest) -> BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }

    fn restore_checkpoint(&self, _req: ports::RestoreCheckpointRequest) -> BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>> {
        Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
    }
}

#[tokio::test]
async fn refresh_desktop_returns_action_result_and_updates_composition() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());
    let mut comp = DesktopComposition::new();

    let ar = refresh_desktop(&mut comp, arc, sid.clone(), None, None).await.expect("refresh ok");
    assert!(ar.success);
    assert!(ar.refreshed);
    assert_eq!(comp.get_session_id().unwrap(), sid);
    let win = comp.latest_window().expect("window present");
    assert_eq!(win.total_lines, 1);
    assert_eq!(win.lines.len(), 1);

    // ensure refresh reason recorded
    let rr = comp.latest_refresh_reason().expect("reason present");
    assert_eq!(rr, RefreshReason::RefreshAction);
}

#[tokio::test]
async fn move_cursor_action_calls_service_and_refreshes() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());

    let fake_service = StdArc::new(FakeService::new(BufferId::from("buf:fake")));
    let set_called = fake_service.set_called.clone();
    let service_arc: StdArc<dyn WorkspaceService> = fake_service.clone();

    let mut comp = DesktopComposition::new();
    // pre-refresh to populate presenter (not required but realistic)
    let _ = refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, None).await.expect("initial refresh ok");

    let res = move_cursor_to_start_and_refresh(&mut comp, service_arc.clone(), arc.clone(), sid.clone(), None).await;
    assert!(res.is_ok());
    let ar = res.unwrap();
    assert!(ar.success);
    assert!(ar.refreshed);
    assert!(set_called.load(Ordering::SeqCst), "set_editor_cursor should have been called on the service");

    // Cursor-move recorded
    let rr = comp.latest_refresh_reason().expect("reason present");
    assert_eq!(rr, RefreshReason::CursorMoved);

    // Cursor move should be recorded as the refresh reason.
    let rr = comp.latest_refresh_reason().expect("reason present");
    assert_eq!(rr, RefreshReason::CursorMoved);
}

#[tokio::test]
async fn insert_line_action_applies_transaction_and_refreshes() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());

    let fake_service = StdArc::new(FakeService::new(BufferId::from("buf:fake")));
    let apply_called = fake_service.apply_called.clone();
    let service_arc: StdArc<dyn WorkspaceService> = fake_service.clone();

    let mut comp = DesktopComposition::new();
    // pre-refresh
    let _ = refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, None).await.expect("initial refresh ok");

    let res = actions::insert_line_at_start_and_refresh(&mut comp, service_arc.clone(), arc.clone(), sid.clone(), None).await;
    assert!(res.is_ok());
    let ar = res.unwrap();
    assert!(ar.success);
    assert!(ar.refreshed);
    assert!(apply_called.load(Ordering::SeqCst), "apply_text_transaction should have been called on the service");

    // Buffer update recorded
    let rr = comp.latest_refresh_reason().expect("reason present");
    assert_eq!(rr, RefreshReason::BufferUpdated);

    // Insert-line should be recorded as a buffer update refresh.
    let rr = comp.latest_refresh_reason().expect("reason present");
    assert_eq!(rr, RefreshReason::BufferUpdated);
}

#[tokio::test]
async fn opened_buffers_projection_refreshes() {
    // Use the real tiny presenter/composition flow with a mutable fake service that
    // returns an authoritative opened-buffer list. We verify composition metadata
    // reflects the service-provided list and updates after the service list changes.
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());

    // Create a shared fake service wrapped in Arc so tests can mutate its internal list.
    let fake = StdArc::new(FakeService::new(BufferId::from("buf:one")));
    // initial state: opened = ["buf:one"], active = Some("buf:one")
    let service_trait: StdArc<dyn WorkspaceService> = fake.clone();

    let mut comp = DesktopComposition::new();

    // First refresh with service: should reflect single opened buffer
    let ar = refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, Some(service_trait.clone())).await.expect("refresh ok");
    assert!(ar.success);
    let meta1 = comp.latest_metadata().expect("meta present");
    assert_eq!(meta1.opened_buffer_count, 1);
    assert_eq!(meta1.opened_buffers.len(), 1);
    assert!(meta1.opened_buffers.iter().any(|i| i.active && i.buffer_id == BufferId::from("buf:one")));

    // Mutate the fake service: add second buffer and mark it active
    {
        let mut o = fake.opened.lock().unwrap();
        o.push(BufferId::from("buf:two"));
    }
    {
        let mut a = fake.active.lock().unwrap();
        *a = Some(BufferId::from("buf:two"));
    }

    // Second refresh should pick up updated opened buffer list and active marker
    let ar2 = refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, Some(service_trait.clone())).await.expect("refresh ok");
    assert!(ar2.success);
    let meta2 = comp.latest_metadata().expect("meta present");
    assert_eq!(meta2.opened_buffer_count, 2);
    assert_eq!(meta2.opened_buffers.len(), 2);
    // ensure exactly one active buffer and that it is buf:two
    let active_items: Vec<_> = meta2.opened_buffers.iter().filter(|i| i.active).collect();
    assert_eq!(active_items.len(), 1);
    assert_eq!(active_items[0].buffer_id, BufferId::from("buf:two"));
}
