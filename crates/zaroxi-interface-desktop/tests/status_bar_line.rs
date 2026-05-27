use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use zaroxi_application_workspace::ports::{
    EditorCursor, EditorDocument, GetActiveEditorDocumentRequest, GetActiveEditorDocumentResponse,
    GetVisibleLinesRequest, GetVisibleLinesResponse, SessionId, WorkspaceView,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop::DesktopComposition;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Minimal in-test WorkspaceView used to populate presenter/composition.
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
    fn get_buffer_content(
        &self,
        _buffer_id: zaroxi_application_workspace::ports::BufferId,
    ) -> BoxFuture<'static, Result<Option<String>, zaroxi_application_workspace::ports::UseCaseError>>
    {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(
        &self,
        _session_id: zaroxi_application_workspace::ports::SessionId,
    ) -> BoxFuture<'static, Result<Option<String>, zaroxi_application_workspace::ports::UseCaseError>>
    {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(
        &self,
        _req: GetActiveEditorDocumentRequest,
    ) -> BoxFuture<
        'static,
        Result<GetActiveEditorDocumentResponse, zaroxi_application_workspace::ports::UseCaseError>,
    > {
        let d = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(
        &self,
        _req: GetVisibleLinesRequest,
    ) -> BoxFuture<
        'static,
        Result<GetVisibleLinesResponse, zaroxi_application_workspace::ports::UseCaseError>,
    > {
        let w = self.window.clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

#[tokio::test]
async fn status_bar_initially_none() {
    let comp = DesktopComposition::new();
    assert!(comp.latest_status_bar_line().is_none());
}

#[tokio::test]
async fn status_bar_after_refresh_reports_reason() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();
    comp.refresh(arc, sid.clone(), None).await.expect("refresh ok");

    let s = comp.latest_status_bar_line().expect("status present");
    // First refresh in this composition path maps to "initial load".
    assert_eq!(s.text, "initial load");
}

#[tokio::test]
async fn status_bar_prefers_ai_projection_when_present() {
    use chrono::Utc;
    use uuid::Uuid;

    // Build fake view and fake service that returns an ExplainExecuted event.
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let wid = zaroxi_kernel_types::Id::new();

    struct FakeSvc;
    impl zaroxi_application_workspace::ports::WorkspaceService for FakeSvc {
        fn boot_workspace(
            &self,
            _req: zaroxi_application_workspace::ports::WorkspaceBootRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::WorkspaceBootResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownWorkspace)
            })
        }
        fn open_buffer(
            &self,
            _req: zaroxi_application_workspace::ports::OpenBufferRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::OpenBufferResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn list_open_buffers(
            &self,
            _req: zaroxi_application_workspace::ports::ListBuffersRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::ListBuffersResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            let b = zaroxi_application_workspace::ports::BufferId::from("buf:fake");
            Box::pin(async move {
                Ok(zaroxi_application_workspace::ports::ListBuffersResponse {
                    buffer_ids: vec![b.clone()],
                    active_buffer: Some(b.clone()),
                })
            })
        }
        fn set_active_buffer(
            &self,
            _req: zaroxi_application_workspace::ports::SetActiveBufferRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::SetActiveBufferResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn get_active_buffer(
            &self,
            _req: zaroxi_application_workspace::ports::GetActiveBufferRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::GetActiveBufferResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Ok(zaroxi_application_workspace::ports::GetActiveBufferResponse {
                    buffer_id: zaroxi_application_workspace::ports::BufferId::from("buf:fake"),
                })
            })
        }
        fn set_editor_cursor(
            &self,
            _req: zaroxi_application_workspace::ports::SetEditorCursorRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::SetEditorCursorResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn set_editor_selection(
            &self,
            _req: zaroxi_application_workspace::ports::SetSelectionRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::SetSelectionResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn clear_editor_selection(
            &self,
            _req: zaroxi_application_workspace::ports::ClearSelectionRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::ClearSelectionResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn get_editor_state(
            &self,
            _req: zaroxi_application_workspace::ports::GetEditorStateRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::GetEditorStateResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn set_viewport_state(
            &self,
            _req: zaroxi_application_workspace::ports::SetViewportRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::SetViewportResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn scroll_viewport(
            &self,
            _req: zaroxi_application_workspace::ports::ScrollViewportRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::ScrollViewportResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn explain_active_buffer(
            &self,
            _req: zaroxi_application_workspace::ports::GetActiveBufferRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::DispatchCommandResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::NoActiveBuffer)
            })
        }
        fn dispatch_command(
            &self,
            _req: zaroxi_application_workspace::ports::DispatchCommandRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::DispatchCommandResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn update_buffer(
            &self,
            _req: zaroxi_application_workspace::ports::UpdateBufferRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::UpdateBufferResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn apply_text_transaction(
            &self,
            _req: zaroxi_application_workspace::ports::ApplyTextTransactionRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::ApplyTextTransactionResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Ok(zaroxi_application_workspace::ports::ApplyTextTransactionResponse {
                    ok: true,
                    state: zaroxi_application_workspace::ports::EditorState {
                        cursor: zaroxi_application_workspace::ports::EditorCursor::zero(),
                        selection: None,
                    },
                    content: None,
                })
            })
        }
        fn get_recent_commands(
            &self,
            _req: zaroxi_application_workspace::ports::GetRecentCommandsRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::GetRecentCommandsResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Ok(zaroxi_application_workspace::ports::GetRecentCommandsResponse {
                    commands: Vec::new(),
                })
            })
        }
        fn get_recent_events(
            &self,
            req: zaroxi_application_workspace::ports::GetRecentEventsRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::GetRecentEventsResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            let buf = zaroxi_application_workspace::ports::BufferId::from("buf:fake");
            let wid = zaroxi_kernel_types::Id::new();
            Box::pin(async move {
                let ev = zaroxi_application_workspace::ports::WorkspaceEvent {
                    id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    session_id: req.session_id.clone(),
                    workspace_id: wid,
                    kind:
                        zaroxi_application_workspace::ports::WorkspaceEventKind::ExplainExecuted {
                            buffer_id: buf.clone(),
                            result: "mocked explain".to_string(),
                        },
                };
                Ok(zaroxi_application_workspace::ports::GetRecentEventsResponse {
                    events: vec![ev],
                })
            })
        }
        fn get_session_snapshot(
            &self,
            _req: zaroxi_application_workspace::ports::GetSessionSnapshotRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::GetSessionSnapshotResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn create_checkpoint(
            &self,
            _req: zaroxi_application_workspace::ports::CreateCheckpointRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::CreateCheckpointResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn save_checkpoint(
            &self,
            _req: zaroxi_application_workspace::ports::SaveCheckpointRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::SaveCheckpointResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn load_checkpoint(
            &self,
            _req: zaroxi_application_workspace::ports::LoadCheckpointRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::LoadCheckpointResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
        fn restore_checkpoint(
            &self,
            _req: zaroxi_application_workspace::ports::RestoreCheckpointRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::RestoreCheckpointResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Err(zaroxi_application_workspace::ports::UseCaseError::UnknownSession)
            })
        }
    }

    // Use the fake service to refresh the composition and surface AI projection.
    let fake_service = std::sync::Arc::new(FakeSvc)
        as std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceService>;
    let mut comp = DesktopComposition::new();
    comp.refresh_with_service(arc, sid.clone(), Some(wid), Some(fake_service))
        .await
        .expect("refresh ok");

    let s = comp.latest_status_bar_line().expect("status present");
    assert!(s.text.starts_with("AI: "));
}
