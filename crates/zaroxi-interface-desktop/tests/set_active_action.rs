use std::sync::Arc;
use zaroxi_interface_desktop::{refresh_desktop, actions, DesktopComposition};
use zaroxi_application_workspace::ports::{WorkspaceView, WorkspaceService, SessionId};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::ports as aw_ports;
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_application_workspace::ports::EditorDocument;
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
            cursor: aw_ports::EditorCursor { line: 0, column: 2 },
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

    fn get_active_editor_document(&self, _req: aw_ports::GetActiveEditorDocumentRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetActiveEditorDocumentResponse, aw_ports::UseCaseError>> {
        let d = self.doc.clone();
        Box::pin(async move { Ok(aw_ports::GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(&self, _req: aw_ports::GetVisibleLinesRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetVisibleLinesResponse, aw_ports::UseCaseError>> {
        let w = self.window.clone();
        Box::pin(async move { Ok(aw_ports::GetVisibleLinesResponse { window: w }) })
    }
}

/// Minimal fake WorkspaceService that supports setting/listing active buffer.
struct FakeServiceActive {
    opened: std::sync::Arc<std::sync::Mutex<Vec<aw_ports::BufferId>>>,
    active: std::sync::Arc<std::sync::Mutex<Option<aw_ports::BufferId>>>,
}

impl FakeServiceActive {
    fn new(initial: aw_ports::BufferId) -> Self {
        let v = vec![initial.clone()];
        Self { opened: std::sync::Arc::new(std::sync::Mutex::new(v)), active: std::sync::Arc::new(std::sync::Mutex::new(Some(initial))) }
    }
}

impl WorkspaceService for FakeServiceActive {
    fn boot_workspace(&self, _req: aw_ports::WorkspaceBootRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::WorkspaceBootResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownWorkspace) })
    }

    fn open_buffer(&self, _req: aw_ports::OpenBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::OpenBufferResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }

    fn list_open_buffers(&self, _req: aw_ports::ListBuffersRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ListBuffersResponse, aw_ports::UseCaseError>> {
        let opened = self.opened.clone();
        let active = self.active.clone();
        Box::pin(async move {
            let list = opened.lock().unwrap().clone();
            let act = active.lock().unwrap().clone();
            Ok(aw_ports::ListBuffersResponse { buffer_ids: list, active_buffer: act })
        })
    }

    fn set_active_buffer(&self, req: aw_ports::SetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetActiveBufferResponse, aw_ports::UseCaseError>> {
        let active = self.active.clone();
        Box::pin(async move {
            let mut a = active.lock().unwrap();
            *a = Some(req.buffer_id.clone());
            Ok(aw_ports::SetActiveBufferResponse { ok: true })
        })
    }

    fn get_active_buffer(&self, _req: aw_ports::GetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetActiveBufferResponse, aw_ports::UseCaseError>> {
        let active = self.active.clone();
        Box::pin(async move {
            match active.lock().unwrap().clone() {
                Some(b) => Ok(aw_ports::GetActiveBufferResponse { buffer_id: b }),
                None => Err(aw_ports::UseCaseError::NoActiveBuffer),
            }
        })
    }

    // The rest of the trait methods are not needed for this tiny test; return UnknownSession or defaults.
    fn set_editor_cursor(&self, _req: aw_ports::SetEditorCursorRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetEditorCursorResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn set_editor_selection(&self, _req: aw_ports::SetSelectionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetSelectionResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn clear_editor_selection(&self, _req: aw_ports::ClearSelectionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ClearSelectionResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn get_editor_state(&self, _req: aw_ports::GetEditorStateRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetEditorStateResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn set_viewport_state(&self, _req: aw_ports::SetViewportRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetViewportResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn scroll_viewport(&self, _req: aw_ports::ScrollViewportRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ScrollViewportResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn explain_active_buffer(&self, _req: aw_ports::GetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::DispatchCommandResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::NoActiveBuffer) })
    }
    fn dispatch_command(&self, _req: aw_ports::DispatchCommandRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::DispatchCommandResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn update_buffer(&self, _req: aw_ports::UpdateBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::UpdateBufferResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn apply_text_transaction(&self, _req: aw_ports::ApplyTextTransactionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ApplyTextTransactionResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Ok(aw_ports::ApplyTextTransactionResponse { ok: true, state: aw_ports::EditorState { cursor: aw_ports::EditorCursor::zero(), selection: None }, content: None }) })
    }
    fn get_recent_commands(&self, _req: aw_ports::GetRecentCommandsRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetRecentCommandsResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Ok(aw_ports::GetRecentCommandsResponse { commands: Vec::new() }) })
    }
    fn get_recent_events(&self, _req: aw_ports::GetRecentEventsRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetRecentEventsResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Ok(aw_ports::GetRecentEventsResponse { events: Vec::new() }) })
    }
    fn get_session_snapshot(&self, _req: aw_ports::GetSessionSnapshotRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetSessionSnapshotResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn create_checkpoint(&self, _req: aw_ports::CreateCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::CreateCheckpointResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn save_checkpoint(&self, _req: aw_ports::SaveCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SaveCheckpointResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn load_checkpoint(&self, _req: aw_ports::LoadCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::LoadCheckpointResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
    fn restore_checkpoint(&self, _req: aw_ports::RestoreCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::RestoreCheckpointResponse, aw_ports::UseCaseError>> {
        Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
    }
}

#[tokio::test]
async fn set_active_buffer_action_sets_and_refreshes() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());

    // Build a fake service that supports active-buffer changes.
    let fake = std::sync::Arc::new(FakeServiceActive::new(BufferId::from("buf:one"))) as std::sync::Arc<dyn WorkspaceService>;

    let mut comp = DesktopComposition::new();
    // pre-refresh to populate presenter (realistic)
    let _ = refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, None).await.expect("initial refresh ok");

    // Invoke the tiny convenience action to set active buffer to "buf:two"
    let res = actions::set_active_buffer_and_get_shell_context(&mut comp, fake.clone(), arc.clone(), sid.clone(), None, BufferId::from("buf:two")).await.expect("action ok");
    assert!(res.action.success);
    assert!(res.action.refreshed);
    let ctx = res.context.expect("context present");
    assert_eq!(ctx.active_buffer.unwrap(), BufferId::from("buf:two"));
}

#[tokio::test]
async fn set_active_buffer_noop_when_already_active() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc as StdArc;

    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());

    // Minimal fake service that starts with the requested buffer already active and counts set_active calls.
    struct FakeSvcCounting {
        opened: StdArc<std::sync::Mutex<Vec<aw_ports::BufferId>>>,
        active: StdArc<std::sync::Mutex<Option<aw_ports::BufferId>>>,
        set_count: StdArc<AtomicUsize>,
    }

    impl FakeSvcCounting {
        fn new(active: aw_ports::BufferId) -> Self {
            let v = vec![active.clone()];
            Self {
                opened: StdArc::new(std::sync::Mutex::new(v)),
                active: StdArc::new(std::sync::Mutex::new(Some(active))),
                set_count: StdArc::new(AtomicUsize::new(0)),
            }
        }
    }

    impl aw_ports::WorkspaceService for FakeSvcCounting {
        fn boot_workspace(&self, _req: aw_ports::WorkspaceBootRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::WorkspaceBootResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(&self, _req: aw_ports::OpenBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::OpenBufferResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(&self, _req: aw_ports::ListBuffersRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ListBuffersResponse, aw_ports::UseCaseError>> {
            let opened = self.opened.clone();
            let active = self.active.clone();
            Box::pin(async move {
                let list = opened.lock().unwrap().clone();
                let act = active.lock().unwrap().clone();
                Ok(aw_ports::ListBuffersResponse { buffer_ids: list, active_buffer: act })
            })
        }
        fn set_active_buffer(&self, req: aw_ports::SetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetActiveBufferResponse, aw_ports::UseCaseError>> {
            let set_called = self.set_count.clone();
            let active = self.active.clone();
            Box::pin(async move {
                set_called.fetch_add(1, Ordering::SeqCst);
                let mut a = active.lock().unwrap();
                *a = Some(req.buffer_id.clone());
                Ok(aw_ports::SetActiveBufferResponse { ok: true })
            })
        }
        fn get_active_buffer(&self, _req: aw_ports::GetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetActiveBufferResponse, aw_ports::UseCaseError>> {
            let active = self.active.clone();
            Box::pin(async move {
                match active.lock().unwrap().clone() {
                    Some(b) => Ok(aw_ports::GetActiveBufferResponse { buffer_id: b }),
                    None => Err(aw_ports::UseCaseError::NoActiveBuffer),
                }
            })
        }

        // The rest are not needed for this tiny test; return defaults/errors.
        fn set_editor_cursor(&self, _req: aw_ports::SetEditorCursorRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetEditorCursorResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(&self, _req: aw_ports::SetSelectionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetSelectionResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(&self, _req: aw_ports::ClearSelectionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ClearSelectionResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(&self, _req: aw_ports::GetEditorStateRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetEditorStateResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(&self, _req: aw_ports::SetViewportRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SetViewportResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(&self, _req: aw_ports::ScrollViewportRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ScrollViewportResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(&self, _req: aw_ports::GetActiveBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::DispatchCommandResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(&self, _req: aw_ports::DispatchCommandRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::DispatchCommandResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(&self, _req: aw_ports::UpdateBufferRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::UpdateBufferResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(&self, _req: aw_ports::ApplyTextTransactionRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::ApplyTextTransactionResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Ok(aw_ports::ApplyTextTransactionResponse { ok: true, state: aw_ports::EditorState { cursor: aw_ports::EditorCursor::zero(), selection: None }, content: None }) })
        }
        fn get_recent_commands(&self, _req: aw_ports::GetRecentCommandsRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetRecentCommandsResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Ok(aw_ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }
        fn get_recent_events(&self, _req: aw_ports::GetRecentEventsRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetRecentEventsResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Ok(aw_ports::GetRecentEventsResponse { events: Vec::new() }) })
        }
        fn get_session_snapshot(&self, _req: aw_ports::GetSessionSnapshotRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::GetSessionSnapshotResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn create_checkpoint(&self, _req: aw_ports::CreateCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::CreateCheckpointResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn save_checkpoint(&self, _req: aw_ports::SaveCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::SaveCheckpointResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(&self, _req: aw_ports::LoadCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::LoadCheckpointResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(&self, _req: aw_ports::RestoreCheckpointRequest) -> aw_ports::BoxFuture<'static, Result<aw_ports::RestoreCheckpointResponse, aw_ports::UseCaseError>> {
            Box::pin(async { Err(aw_ports::UseCaseError::UnknownSession) })
        }
    }

    // Create service with buf:two already active.
    let svc = StdArc::new(FakeSvcCounting::new(BufferId::from("buf:two"))) as StdArc<dyn aw_ports::WorkspaceService>;
    let set_count = svc.as_any().downcast_ref::<FakeSvcCounting>().map(|s| s.set_count.clone());

    // pre-refresh to populate presenter (realistic)
    let _ = refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, None).await.expect("initial refresh ok");

    // Invoke action to set active to buf:two (already active)
    let res = actions::set_active_buffer_and_get_shell_context(&mut comp, svc.clone(), arc.clone(), sid.clone(), None, BufferId::from("buf:two")).await.expect("action ok");
    assert!(res.action.success);
    assert!(res.action.refreshed);
    let ctx = res.context.expect("context present");
    assert_eq!(ctx.active_buffer.unwrap(), BufferId::from("buf:two"));

    // Ensure set_active_buffer was not called on the service (counter remains 0).
    if let Some(cnt) = set_count {
        assert_eq!(cnt.load(Ordering::SeqCst), 0);
    }
}
