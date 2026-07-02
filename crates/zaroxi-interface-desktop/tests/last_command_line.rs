use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use zaroxi_application_workspace::ports as aw_ports;
use zaroxi_application_workspace::ports::{
    EditorCursor, EditorDocument, GetActiveEditorDocumentRequest, GetActiveEditorDocumentResponse,
    GetVisibleLinesRequest, GetVisibleLinesResponse, SessionId, WorkspaceView,
};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_kernel_types::Id;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Minimal in-test WorkspaceView used to populate presenter/composition.
struct FakeView {
    doc: EditorDocument,
    window: zaroxi_application_workspace::view::VisibleLinesWindow,
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

        let vl = zaroxi_application_workspace::view::VisibleLine {
            line_number: 1,
            text: "abcd".to_string(),
            is_cursor_line: true,
            cursor_column: Some(2),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = zaroxi_application_workspace::view::VisibleLinesWindow {
            top_line: 1,
            total_lines: 1,
            lines: vec![vl],
        };

        FakeView { doc: ed, window: vw }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(
        &self,
        _buffer_id: aw_ports::BufferId,
    ) -> BoxFuture<'static, Result<Option<String>, aw_ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(
        &self,
        _session_id: aw_ports::SessionId,
    ) -> BoxFuture<'static, Result<Option<String>, aw_ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(
        &self,
        _req: GetActiveEditorDocumentRequest,
    ) -> BoxFuture<'static, Result<GetActiveEditorDocumentResponse, aw_ports::UseCaseError>> {
        let d = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(
        &self,
        _req: GetVisibleLinesRequest,
    ) -> BoxFuture<'static, Result<GetVisibleLinesResponse, aw_ports::UseCaseError>> {
        let w = self.window.clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

#[tokio::test]
async fn last_command_line_present_and_parsed() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());
    let wid = Id::new();

    // Fake service that returns a single recent command (OpenBuffer success).
    struct FakeSvc {
        sid: aw_ports::SessionId,
        wid: Id,
    }
    impl FakeSvc {
        fn new(sid: aw_ports::SessionId, wid: Id) -> Self {
            Self { sid, wid }
        }
    }

    impl aw_ports::WorkspaceService for FakeSvc {
        fn boot_workspace(
            &self,
            _req: aw_ports::WorkspaceBootRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::WorkspaceBootResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(
            &self,
            _req: aw_ports::OpenBufferRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::OpenBufferResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(
            &self,
            _req: aw_ports::ListBuffersRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::ListBuffersResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(aw_ports::ListBuffersResponse {
                    buffer_ids: vec![aw_ports::BufferId::from("buf:fake")],
                    active_buffer: Some(aw_ports::BufferId::from("buf:fake")),
                })
            })
        }
        fn set_active_buffer(
            &self,
            _req: aw_ports::SetActiveBufferRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::SetActiveBufferResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(
            &self,
            _req: aw_ports::GetActiveBufferRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::GetActiveBufferResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(aw_ports::GetActiveBufferResponse {
                    buffer_id: aw_ports::BufferId::from("buf:fake"),
                })
            })
        }

        fn set_editor_cursor(
            &self,
            _req: aw_ports::SetEditorCursorRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::SetEditorCursorResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(
            &self,
            _req: aw_ports::SetSelectionRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::SetSelectionResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(
            &self,
            _req: aw_ports::ClearSelectionRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::ClearSelectionResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(
            &self,
            _req: aw_ports::GetEditorStateRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::GetEditorStateResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(
            &self,
            _req: aw_ports::SetViewportRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::SetViewportResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(
            &self,
            _req: aw_ports::ScrollViewportRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::ScrollViewportResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(
            &self,
            _req: aw_ports::GetActiveBufferRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::DispatchCommandResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(
            &self,
            _req: aw_ports::DispatchCommandRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::DispatchCommandResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(
            &self,
            _req: aw_ports::UpdateBufferRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::UpdateBufferResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(
            &self,
            _req: aw_ports::ApplyTextTransactionRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::ApplyTextTransactionResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(aw_ports::ApplyTextTransactionResponse {
                    ok: true,
                    state: aw_ports::EditorState {
                        cursor: aw_ports::EditorCursor::zero(),
                        selection: None,
                    },
                    content: None,
                })
            })
        }
        fn get_recent_commands(
            &self,
            _req: aw_ports::GetRecentCommandsRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::GetRecentCommandsResponse, aw_ports::UseCaseError>,
        > {
            let sid = self.sid.clone();
            let wid = self.wid;
            Box::pin(async move {
                let rec = aw_ports::CommandRecord::new_success(
                    aw_ports::CommandKind::OpenBuffer { path: PathBuf::from("main.rs") },
                    Some(sid.0),
                    Some(wid),
                    Some(BufferId::from("buf:fake")),
                    Some("opened".to_string()),
                );
                Ok(aw_ports::GetRecentCommandsResponse { commands: vec![rec] })
            })
        }
        fn get_recent_events(
            &self,
            _req: aw_ports::GetRecentEventsRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::GetRecentEventsResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Ok(aw_ports::GetRecentEventsResponse { events: Vec::new() }) })
        }
        fn get_session_snapshot(
            &self,
            _req: aw_ports::GetSessionSnapshotRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::GetSessionSnapshotResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn create_checkpoint(
            &self,
            _req: aw_ports::CreateCheckpointRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::CreateCheckpointResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn save_checkpoint(
            &self,
            _req: aw_ports::SaveCheckpointRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::SaveCheckpointResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(
            &self,
            _req: aw_ports::LoadCheckpointRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::LoadCheckpointResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(
            &self,
            _req: aw_ports::RestoreCheckpointRequest,
        ) -> aw_ports::BoxFuture<
            'static,
            Result<aw_ports::RestoreCheckpointResponse, aw_ports::UseCaseError>,
        > {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
    }

    let fake_service = std::sync::Arc::new(FakeSvc::new(sid.clone(), wid))
        as std::sync::Arc<dyn aw_ports::WorkspaceService>;

    let mut comp = DesktopComposition::new();
    comp.refresh_with_service(arc, sid.clone(), Some(wid), Some(fake_service))
        .await
        .expect("refresh ok");

    let ctx = comp.latest_shell_context().expect("context present");
    assert!(ctx.last_command_line.is_some());
    assert_eq!(ctx.last_command_line.unwrap(), "OpenBuffer ✓");
}
