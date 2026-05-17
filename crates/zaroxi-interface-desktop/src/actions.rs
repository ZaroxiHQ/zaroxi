/*!
Tiny action seam: refresh desktop composition.

Architectural rationale (Phase 14 - minimal desktop action flow):
- Provide a tiny, explicit action in the interface layer that composes existing
  seams (WorkspaceView, Presenter, DesktopComposition) to refresh the active
  desktop composition snapshot.
- Keep this strictly orchestration-only: do not duplicate any editor logic,
  do not modify application ports, and avoid introducing broader controller
  abstractions or event buses.
- The action delegates to DesktopComposition::refresh which already uses the
  Presenter + adapter seam (view_adapter) to obtain the renderable window.
- This lets external harnesses or potential future UI shells call a single
  intent-focused function to update presenter/composition state.

Public API:
- pub async fn refresh_desktop(
      comp: &mut DesktopComposition,
      view: std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceView>,
      session_id: zaroxi_application_workspace::ports::SessionId,
      workspace_id: Option<zaroxi_kernel_types::Id>,
  ) -> Result<(), String>

The function is intentionally tiny and documented. Tests exercise the happy-path using
a small in-test WorkspaceView stub.
*/

use std::sync::Arc;

use zaroxi_application_workspace::ports::{WorkspaceView, SessionId, WorkspaceService, GetActiveBufferRequest, SetEditorCursorRequest, EditorCursor};
use zaroxi_kernel_types::Id;

use crate::desktop::DesktopComposition;

/// Refresh the given DesktopComposition by delegating to its async `refresh` method.
///
/// Parameters:
/// - comp: mutable reference to an existing DesktopComposition instance (presenter state).
/// - view: Arc'd application WorkspaceView (read-only seam).
/// - session_id: typed SessionId for the active UI session.
/// - workspace_id: optional Workspace Id for caller metadata.
///
/// Returns:
/// - Ok(()) on success.
/// - Err(String) if the underlying presenter/composition refresh failed.
pub async fn refresh_desktop(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
) -> Result<(), String> {
    comp.refresh(view, session_id, workspace_id).await
}

/// Small shell action: move the editor cursor for the active buffer to the document start
/// (line 0, column 0) and refresh the desktop composition.
///
/// Rationale:
/// - This is intentionally tiny and orchestration-only. It resolves the active buffer
///   using the WorkspaceService port, issues a typed editor-state mutation via
///   set_editor_cursor, and then refreshes the composition via the existing presenter
///   refresh seam. No editor logic is implemented here — the application handles
///   cursor mutation semantics.
///
/// Parameters:
/// - comp: mutable DesktopComposition to refresh after the side-effect.
/// - service: Arc<dyn WorkspaceService> used to perform the cursor mutation.
/// - view: Arc<dyn WorkspaceView> used to refresh the composition.
/// - session_id: typed session id.
/// - workspace_id: optional workspace id recorded on the composition.
///
/// Returns:
/// - Ok(()) on success; Err(String) with a user-friendly message on failure.
pub async fn move_cursor_to_start_and_refresh(
    comp: &mut crate::desktop::DesktopComposition,
    service: Arc<dyn WorkspaceService>,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
) -> Result<(), String> {
    // Resolve active buffer id from the service (explicit small use-case).
    let active_resp = service
        .get_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() })
        .await
        .map_err(|e| e.to_string())?;

    let buffer_id = active_resp.buffer_id;

    // Issue set_editor_cursor to move caret to start (0,0).
    let set_req = SetEditorCursorRequest {
        session_id: session_id.clone(),
        buffer_id: buffer_id.clone(),
        cursor: EditorCursor { line: 0, column: 0 },
    };

    service
        .set_editor_cursor(set_req)
        .await
        .map_err(|e| e.to_string())?;

    // Refresh composition via existing tiny action (keeps responsibilities separated).
    refresh_desktop(comp, view, session_id, workspace_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use zaroxi_application_workspace::ports::{
        WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, EditorDocument,
    };
    use zaroxi_core_editor_buffer::ports::BufferId;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc as StdArc;

    /// Minimal in-test WorkspaceView stub that returns a tiny document and a prebuilt visible window.
    struct FakeView {
        doc: EditorDocument,
        window: crate::super::view::VisibleLinesWindow,
    }

    impl FakeView {
        fn new() -> Self {
            // Build a simple document with one line "abcd" and cursor at col 2.
            let content = Some("abcd".to_string());
            let ed = EditorDocument {
                buffer_id: BufferId::from("buf:fake"),
                content: content.clone(),
                cursor: crate::super::ports::EditorCursor { line: 0, column: 2 },
                selection: None,
                line_count: 1,
                current_line: content.and_then(|c| c.lines().nth(0).map(|s| s.to_string())),
            };

            // Build a VisibleLinesWindow of one line.
            let vl = crate::super::view::VisibleLine {
                line_number: 1,
                text: "abcd".to_string(),
                is_cursor_line: true,
                cursor_column: Some(2),
                selection_intersects: false,
                selection_start_column: None,
                selection_end_column: None,
            };
            let vw = crate::super::view::VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };

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

        fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetActiveEditorDocumentResponse, crate::ports::UseCaseError>> {
            let d = self.doc.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveEditorDocumentResponse { document: d }) })
        }

        fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetVisibleLinesResponse, crate::ports::UseCaseError>> {
            let w = self.window.clone();
            Box::pin(async move { Ok(crate::ports::GetVisibleLinesResponse { window: w }) })
        }
    }

    /// Minimal fake WorkspaceService implementing only the small methods we need for this test;
    /// other methods return standard errors. This keeps the test focused and avoids pulling
    /// in application orchestrator boot semantics.
    struct FakeService {
        buffer_id: BufferId,
        set_called: StdArc<AtomicBool>,
    }

    impl FakeService {
        fn new(buffer_id: BufferId) -> Self {
            Self { buffer_id, set_called: StdArc::new(AtomicBool::new(false)) }
        }
    }

    impl crate::ports::WorkspaceService for FakeService {
        fn boot_workspace(&self, _req: crate::ports::WorkspaceBootRequest) -> crate::BoxFuture<'static, Result<crate::ports::WorkspaceBootResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(&self, _req: crate::ports::OpenBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::OpenBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(&self, _req: crate::ports::ListBuffersRequest) -> crate::BoxFuture<'static, Result<crate::ports::ListBuffersResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_active_buffer(&self, _req: crate::ports::SetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>> {
            let bid = self.buffer_id.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveBufferResponse { buffer_id: bid }) })
        }

        fn set_editor_cursor(&self, req: crate::ports::SetEditorCursorRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>> {
            let expected = self.buffer_id.clone();
            let set_called = self.set_called.clone();
            Box::pin(async move {
                if req.buffer_id == expected && req.cursor.line == 0 && req.cursor.column == 0 {
                    set_called.store(true, Ordering::SeqCst);
                    Ok(crate::ports::SetEditorCursorResponse { ok: true })
                } else {
                    Err(crate::ports::UseCaseError::InvalidActiveBuffer(req.buffer_id.to_string()))
                }
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
        fn apply_text_transaction(&self, _req: crate::ports::ApplyTextTransactionRequest) -> crate::BoxFuture<'static, Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
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
        fn load_checkpoint(&self, _req: crate::ports::LoadCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::LoadCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(&self, _req: crate::ports::RestoreCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::RestoreCheckpointResponse, crate::ports::UseCaseError>> {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
    }

    #[tokio::test]
    async fn refresh_action_updates_composition() {
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let mut comp = crate::desktop::DesktopComposition::new();
        // Call the tiny action
        refresh_desktop(&mut comp, arc, sid.clone(), None).await.expect("refresh ok");
        assert_eq!(comp.get_session_id().unwrap(), sid);
        let win = comp.latest_window().expect("window present");
        assert_eq!(win.total_lines, 1);
        assert_eq!(win.lines.len(), 1);
    }

    #[tokio::test]
    async fn move_cursor_action_moves_and_refreshes() {
        // Set up a fake view and fake service that cooperatively simulate a running orchestrator.
        let v = FakeView::new();
        let view_arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());

        // Fake service uses the same buffer id as the FakeView (buf:fake).
        let fake_service = FakeService::new(BufferId::from("buf:fake"));
        let service_arc: StdArc<dyn crate::ports::WorkspaceService> = StdArc::new(fake_service);

        let mut comp = crate::desktop::DesktopComposition::new();

        // First refresh to populate presenter state
        refresh_desktop(&mut comp, view_arc.clone(), sid.clone(), None).await.expect("initial refresh ok");

        // Execute the move-cursor action which should call set_editor_cursor on the service
        // and then refresh the composition again.
        let res = move_cursor_to_start_and_refresh(&mut comp, service_arc.clone(), view_arc.clone(), sid.clone(), None).await;
        assert!(res.is_ok(), "move cursor action should succeed");

        // There is no direct observable cursor state on the composition beyond refresh success,
        // but success indicates the orchestration path executed (get_active_buffer -> set_editor_cursor -> refresh).
    }
}
