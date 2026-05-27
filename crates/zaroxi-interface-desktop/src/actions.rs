mod actions_inner;
pub use actions_inner::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::RefreshReason;
    use std::sync::Arc;
    use std::sync::Arc as StdArc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use zaroxi_application_workspace::ports::{
        EditorCursor, EditorDocument, GetActiveEditorDocumentRequest, GetVisibleLinesRequest,
        SessionId, WorkspaceView,
    };
    use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
    use zaroxi_core_editor_buffer::ports::BufferId;

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
        fn get_buffer_content(
            &self,
            _buffer_id: crate::ports::BufferId,
        ) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>>
        {
            Box::pin(async move { Ok(Some("".to_string())) })
        }

        fn get_active_buffer_content(
            &self,
            _session_id: crate::ports::SessionId,
        ) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>>
        {
            Box::pin(async move { Ok(Some("".to_string())) })
        }

        fn get_active_editor_document(
            &self,
            _req: GetActiveEditorDocumentRequest,
        ) -> crate::ports::BoxFuture<
            'static,
            Result<crate::ports::GetActiveEditorDocumentResponse, crate::ports::UseCaseError>,
        > {
            let d = self.doc.clone();
            Box::pin(
                async move { Ok(crate::ports::GetActiveEditorDocumentResponse { document: d }) },
            )
        }

        fn get_visible_lines(
            &self,
            _req: GetVisibleLinesRequest,
        ) -> crate::ports::BoxFuture<
            'static,
            Result<crate::ports::GetVisibleLinesResponse, crate::ports::UseCaseError>,
        > {
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
        apply_called: StdArc<AtomicBool>,
        last_update: StdArc<std::sync::Mutex<Option<String>>>,
    }

    impl FakeService {
        fn new(buffer_id: BufferId) -> Self {
            Self {
                buffer_id,
                set_called: StdArc::new(AtomicBool::new(false)),
                apply_called: StdArc::new(AtomicBool::new(false)),
                last_update: StdArc::new(std::sync::Mutex::new(None)),
            }
        }
    }

    impl crate::ports::WorkspaceService for FakeService {
        fn boot_workspace(
            &self,
            _req: crate::ports::WorkspaceBootRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::WorkspaceBootResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(
            &self,
            _req: crate::ports::OpenBufferRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::OpenBufferResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(
            &self,
            _req: crate::ports::ListBuffersRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::ListBuffersResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn set_active_buffer(
            &self,
            _req: crate::ports::SetActiveBufferRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(
            &self,
            _req: crate::ports::GetActiveBufferRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>,
        > {
            let bid = self.buffer_id.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveBufferResponse { buffer_id: bid }) })
        }

        fn set_editor_cursor(
            &self,
            req: crate::ports::SetEditorCursorRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>,
        > {
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

        fn set_editor_selection(
            &self,
            _req: crate::ports::SetSelectionRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::SetSelectionResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(
            &self,
            _req: crate::ports::ClearSelectionRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::ClearSelectionResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(
            &self,
            _req: crate::ports::GetEditorStateRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::GetEditorStateResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn set_viewport_state(
            &self,
            _req: crate::ports::SetViewportRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::SetViewportResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(
            &self,
            _req: crate::ports::ScrollViewportRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::ScrollViewportResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(
            &self,
            _req: crate::ports::GetActiveBufferRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(
            &self,
            _req: crate::ports::DispatchCommandRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(
            &self,
            req: crate::ports::UpdateBufferRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::UpdateBufferResponse, crate::ports::UseCaseError>,
        > {
            let mut guard = self.last_update.lock().unwrap();
            *guard = Some(req.new_content.clone());
            Box::pin(async move { Ok(crate::ports::UpdateBufferResponse { ok: true }) })
        }
        fn apply_text_transaction(
            &self,
            _req: crate::ports::ApplyTextTransactionRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>,
        > {
            let apply_called = self.apply_called.clone();
            Box::pin(async move {
                apply_called.store(true, Ordering::SeqCst);
                Ok(crate::ports::ApplyTextTransactionResponse {
                    ok: true,
                    state: crate::ports::EditorState {
                        cursor: crate::ports::EditorCursor::zero(),
                        selection: None,
                    },
                    content: None,
                })
            })
        }

        fn get_recent_commands(
            &self,
            _req: crate::ports::GetRecentCommandsRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::GetRecentCommandsResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Ok(crate::ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }
        fn get_recent_events(
            &self,
            _req: crate::ports::GetRecentEventsRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::GetRecentEventsResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Ok(crate::ports::GetRecentEventsResponse { events: Vec::new() }) })
        }

        // Phase 10: application-level AI orchestration API (test mock implementations).
        fn request_ai_edit(&self, req: crate::ports::RequestAiEditRequest) -> crate::BoxFuture<'static, Result<crate::ports::RequestAiEditResponse, crate::ports::UseCaseError>> {
            let proposal = format!("// AI Edit: proposed change\n{}", req.content.clone().unwrap_or_default());
            let resp = crate::ports::RequestAiEditResponse {
                proposal: crate::ports::AiProposal {
                    target_buffer: req.buffer_id.clone(),
                    proposal_text: proposal.clone(),
                    summary: Some("AI edit proposed".to_string()),
                },
            };
            Box::pin(async move { Ok(resp) })
        }

        fn apply_ai_edit(&self, req: crate::ports::ApplyAiEditRequest) -> crate::BoxFuture<'static, Result<crate::ports::ApplyAiEditResponse, crate::ports::UseCaseError>> {
            // Record the applied content similarly to update_buffer for test observation.
            let mut guard = self.last_update.lock().unwrap();
            *guard = Some(req.proposal_text.clone());
            Box::pin(async move { Ok(crate::ports::ApplyAiEditResponse { ok: true }) })
        }

        fn cancel_ai_edit(&self, _req: crate::ports::CancelAiEditRequest) -> crate::BoxFuture<'static, Result<crate::ports::CancelAiEditResponse, crate::ports::UseCaseError>> {
            Box::pin(async move { Ok(crate::ports::CancelAiEditResponse { ok: true }) })
        }

        fn get_session_snapshot(
            &self,
            _req: crate::ports::GetSessionSnapshotRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::GetSessionSnapshotResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn create_checkpoint(
            &self,
            _req: crate::ports::CreateCheckpointRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::CreateCheckpointResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }

        fn save_checkpoint(
            &self,
            _req: crate::ports::SaveCheckpointRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::SaveCheckpointResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(
            &self,
            _req: crate::ports::LoadCheckpointRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::LoadCheckpointResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(
            &self,
            _req: crate::ports::RestoreCheckpointRequest,
        ) -> crate::BoxFuture<
            'static,
            Result<crate::ports::RestoreCheckpointResponse, crate::ports::UseCaseError>,
        > {
            Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
        }
    }

    #[tokio::test]
    async fn refresh_action_updates_composition() {
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let mut comp = crate::desktop::DesktopComposition::new();
        // Call the tiny action (no service available in this test)
        let ar =
            refresh_desktop(&mut comp, arc, sid.clone(), None, None).await.expect("refresh ok");
        assert!(ar.success);
        assert!(ar.refreshed);
        assert_eq!(comp.get_session_id().unwrap(), sid);
        let win = comp.latest_window().expect("window present");
        assert_eq!(win.total_lines, 1);
        assert_eq!(win.lines.len(), 1);

        // Composition should record a refresh reason for this explicit refresh action.
        let rr = comp.latest_refresh_reason().expect("reason present");
        assert_eq!(rr, RefreshReason::RefreshAction);

        // Status snapshot should be available for shell consumption.
        let status = comp.latest_status().expect("status present");
        assert!(status.has_render_window);
        assert!(status.has_metadata);
        assert!(status.has_opened_buffers);
        assert!(!status.has_ai_projection);
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
        let _ = refresh_desktop(&mut comp, view_arc.clone(), sid.clone(), None, None)
            .await
            .expect("initial refresh ok");

        // Execute the move-cursor action which should call set_editor_cursor on the service
        // and then refresh the composition again.
        let res = move_cursor_to_start_and_refresh(
            &mut comp,
            service_arc.clone(),
            view_arc.clone(),
            sid.clone(),
            None,
        )
        .await;
        assert!(res.is_ok(), "move cursor action should succeed");
        let ar = res.unwrap();
        assert!(ar.success);
        assert!(ar.refreshed);

        // There is no direct observable cursor state on the composition beyond refresh success,
        // but success indicates the orchestration path executed (get_active_buffer -> set_editor_cursor -> refresh).
    }

    #[tokio::test]
    async fn insert_line_action_inserts_and_refreshes() {
        // Set up a fake view and fake service that cooperatively simulate a running orchestrator.
        let v = FakeView::new();
        let view_arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());

        // Fake service uses the same buffer id as the FakeView (buf:fake).
        let fake_service = FakeService::new(BufferId::from("buf:fake"));
        let service_arc: StdArc<dyn crate::ports::WorkspaceService> = StdArc::new(fake_service);

        let mut comp = crate::desktop::DesktopComposition::new();

        // First refresh to populate presenter state
        let _ = refresh_desktop(&mut comp, view_arc.clone(), sid.clone(), None, None)
            .await
            .expect("initial refresh ok");

        // Execute the insert-line action which should call apply_text_transaction on the service
        // and then refresh the composition again.
        let res = insert_line_at_start_and_refresh(
            &mut comp,
            service_arc.clone(),
            view_arc.clone(),
            sid.clone(),
            None,
        )
        .await;
        assert!(res.is_ok(), "insert-line action should succeed");
        let ar = res.unwrap();
        assert!(ar.success);
        assert!(ar.refreshed);
    }

    #[tokio::test]
    async fn set_active_buffer_detects_external_change() {
        // Scenario:
        // - Composition has not been refreshed (no metadata).
        // - WorkspaceService reports the requested buffer is already active (external change).
        // Expected:
        // - The convenience action should mark ActiveBufferChanged so the upcoming refresh
        //   records the authoritative active-buffer transition for the shell.
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let mut comp = crate::desktop::DesktopComposition::new();

        // Fake service reports buf:two as the currently active buffer.
        let fake_service = std::sync::Arc::new(FakeService::new(BufferId::from("buf:two")))
            as std::sync::Arc<dyn crate::ports::WorkspaceService>;

        let res = set_active_buffer_and_get_shell_context(
            &mut comp,
            fake_service.clone(),
            arc.clone(),
            sid.clone(),
            None,
            BufferId::from("buf:two"),
        )
        .await
        .expect("action ok");
        assert!(res.action.success);

        let rr = comp.latest_refresh_reason().expect("reason present");
        assert_eq!(rr, crate::desktop::RefreshReason::ActiveBufferChanged);
    }
}
