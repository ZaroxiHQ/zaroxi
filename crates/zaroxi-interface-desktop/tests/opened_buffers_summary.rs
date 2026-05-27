use std::sync::Arc;

use std::future::Future;
use std::pin::Pin;
use zaroxi_application_workspace::ports::{
    EditorCursor, EditorDocument, GetActiveBufferRequest, GetActiveBufferResponse,
    GetActiveEditorDocumentRequest, GetActiveEditorDocumentResponse, GetVisibleLinesRequest,
    GetVisibleLinesResponse, ListBuffersRequest, ListBuffersResponse, SessionId, WorkspaceView,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_kernel_types::Id;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Minimal in-test WorkspaceView used for presenter/population (no errors).
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
async fn opened_buffers_summary_initially_empty() {
    let comp = DesktopComposition::new();
    let sum = comp.latest_opened_buffers_summary();
    assert_eq!(sum.count, 0);
    assert!(sum.items.is_empty());
    assert!(sum.active.is_none());
}

#[tokio::test]
async fn opened_buffers_summary_after_one_buffer() {
    // Fake service returning one opened buffer
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
            _req: ListBuffersRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<ListBuffersResponse, zaroxi_application_workspace::ports::UseCaseError>,
        > {
            let b = zaroxi_application_workspace::ports::BufferId::from("buf:one");
            Box::pin(async move {
                Ok(ListBuffersResponse {
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
            _req: GetActiveBufferRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<GetActiveBufferResponse, zaroxi_application_workspace::ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(GetActiveBufferResponse {
                    buffer_id: zaroxi_application_workspace::ports::BufferId::from("buf:one"),
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
            _req: zaroxi_application_workspace::ports::GetRecentEventsRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::GetRecentEventsResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Ok(zaroxi_application_workspace::ports::GetRecentEventsResponse {
                    events: Vec::new(),
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

    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());
    let wid = Id::new();
    let mut comp = DesktopComposition::new();

    // Refresh with service (populates opened_buffers)
    let svc = std::sync::Arc::new(FakeSvc)
        as std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceService>;
    comp.refresh_with_service(arc, sid.clone(), Some(wid.clone()), Some(svc))
        .await
        .expect("refresh ok");

    let sum = comp.latest_opened_buffers_summary();
    assert_eq!(sum.count, 1);
    assert_eq!(sum.items.len(), 1);
    let it = &sum.items[0];
    assert_eq!(it.buffer_id, BufferId::from("buf:one"));
    assert!(it.active);
}

#[tokio::test]
async fn opened_buffers_summary_multiple_buffers() {
    // Fake service returning two opened buffers, second active
    struct FakeSvc2;
    impl zaroxi_application_workspace::ports::WorkspaceService for FakeSvc2 {
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
            _req: ListBuffersRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<ListBuffersResponse, zaroxi_application_workspace::ports::UseCaseError>,
        > {
            let b1 = zaroxi_application_workspace::ports::BufferId::from("buf:one");
            let b2 = zaroxi_application_workspace::ports::BufferId::from("buf:two");
            Box::pin(async move {
                Ok(ListBuffersResponse {
                    buffer_ids: vec![b1.clone(), b2.clone()],
                    active_buffer: Some(b2.clone()),
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
            _req: GetActiveBufferRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<GetActiveBufferResponse, zaroxi_application_workspace::ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(GetActiveBufferResponse {
                    buffer_id: zaroxi_application_workspace::ports::BufferId::from("buf:two"),
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
            _req: zaroxi_application_workspace::ports::GetRecentEventsRequest,
        ) -> zaroxi_application_workspace::ports::BoxFuture<
            'static,
            Result<
                zaroxi_application_workspace::ports::GetRecentEventsResponse,
                zaroxi_application_workspace::ports::UseCaseError,
            >,
        > {
            Box::pin(async {
                Ok(zaroxi_application_workspace::ports::GetRecentEventsResponse {
                    events: Vec::new(),
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

    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());
    let wid = Id::new();
    let mut comp = DesktopComposition::new();

    let svc = std::sync::Arc::new(FakeSvc2)
        as std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceService>;
    comp.refresh_with_service(arc, sid.clone(), Some(wid.clone()), Some(svc))
        .await
        .expect("refresh ok");

    let sum = comp.latest_opened_buffers_summary();
    assert_eq!(sum.count, 2);
    assert_eq!(sum.items.len(), 2);
    let active = sum.active.expect("active present");
    assert_eq!(active, BufferId::from("buf:two"));
}
