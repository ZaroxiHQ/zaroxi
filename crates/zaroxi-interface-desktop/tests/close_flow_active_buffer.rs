mod close_flow_common;
use close_flow_common::CloseFlowViewStub;
use std::sync::Arc;
use zaroxi_application_workspace::ports;
use zaroxi_application_workspace::ports::SessionId;
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop as iface;
use zaroxi_interface_desktop::{DesktopComposition, actions, refresh_desktop};

/// Ensure request_close_active sets pending-close and the status banner contains the expected hints.
#[tokio::test]
async fn request_close_active_enters_pending_close_and_status() {
    let view = Arc::new(CloseFlowViewStub::new()) as Arc<dyn ports::WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None)
        .await
        .expect("refresh ok");

    // Request close: should set pending close and status banner should reflect it.
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone())
        .await
        .expect("request close ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_active");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    let text = bar.text;
    assert!(text.contains("Close buffer"), "status banner should mention Close buffer");
    assert!(
        text.contains("unsaved changes") || text.contains("[S]ave"),
        "status banner should include action hints or unsaved indicator"
    );
}

/// Confirm closing an active buffer when multiple opened buffers exist:
/// - The closed buffer is removed from opened buffers.
/// - Active buffer falls back deterministically (previous neighbor or first).
/// - Status message contains an explicit closed buffer label.
#[tokio::test]
async fn confirm_close_active_prefers_neighbor_and_updates_state() {
    let view = Arc::new(CloseFlowViewStub::new()) as Arc<dyn ports::WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // Build a deterministic opened-buffers projection with two buffers by using a fake service.
    struct TwoBufService;
    impl ports::WorkspaceService for TwoBufService {
        fn boot_workspace(
            &self,
            _req: ports::WorkspaceBootRequest,
        ) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(
            &self,
            _req: ports::OpenBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(
            &self,
            _req: ports::ListBuffersRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>>
        {
            let buf1 = BufferId::from("buf:one");
            let buf2 = BufferId::from("buf:two");
            Box::pin(async move {
                Ok(ports::ListBuffersResponse {
                    buffer_ids: vec![buf1.clone(), buf2.clone()],
                    active_buffer: Some(buf1.clone()),
                })
            })
        }
        fn set_active_buffer(
            &self,
            _req: ports::SetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(
            &self,
            _req: ports::GetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_cursor(
            &self,
            _req: ports::SetEditorCursorRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(
            &self,
            _req: ports::SetSelectionRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(
            &self,
            _req: ports::ClearSelectionRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(
            &self,
            _req: ports::GetEditorStateRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(
            &self,
            _req: ports::SetViewportRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(
            &self,
            _req: ports::ScrollViewportRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(
            &self,
            _req: ports::GetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(
            &self,
            _req: ports::DispatchCommandRequest,
        ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(
            &self,
            _req: ports::UpdateBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(
            &self,
            _req: ports::ApplyTextTransactionRequest,
        ) -> ports::BoxFuture<
            'static,
            Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(ports::ApplyTextTransactionResponse {
                    ok: true,
                    state: ports::EditorState {
                        cursor: ports::EditorCursor::zero(),
                        selection: None,
                    },
                    content: None,
                })
            })
        }
        fn get_recent_commands(
            &self,
            _req: ports::GetRecentCommandsRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }
        fn get_recent_events(
            &self,
            _req: ports::GetRecentEventsRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) })
        }
        fn get_session_snapshot(
            &self,
            _req: ports::GetSessionSnapshotRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn create_checkpoint(
            &self,
            _req: ports::CreateCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn save_checkpoint(
            &self,
            _req: ports::SaveCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(
            &self,
            _req: ports::LoadCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(
            &self,
            _req: ports::RestoreCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
    }

    let svc = std::sync::Arc::new(TwoBufService) as std::sync::Arc<dyn ports::WorkspaceService>;

    // refresh to populate composition metadata via the fake service
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(svc.clone()))
        .await
        .expect("refresh ok");

    // Simulate a pending-close for the active buffer.
    let buf1 = BufferId::from("buf:one");
    comp.set_pending_close(iface::PendingClose::BufferClose {
        buffer_id: buf1.clone(),
        display: Some("one.rs".to_string()),
        dirty: true,
    });
    assert!(comp.has_pending_close());

    // Confirm discard-and-close (could be save-and-close as well; behavior should match).
    let _ = actions::confirm_discard_and_close(&mut comp).await.expect("confirm discard ok");

    // Pending cleared.
    assert!(
        !comp.has_pending_close(),
        "pending close should be cleared after confirm_discard_and_close"
    );

    // The removed buffer should no longer be listed, and the active buffer should now be buf2.
    let obs = comp.latest_opened_buffers_summary();
    assert_eq!(obs.count, 1, "one buffer should remain after closing one of two");
    let buf2 = BufferId::from("buf:two");
    assert_eq!(
        obs.active.unwrap(),
        buf2,
        "active buffer should fall back to the neighbor/first remaining buffer"
    );

    // Status should mention the closed buffer label.
    let bar = comp.latest_status_bar_line().expect("status present");
    assert!(bar.text.contains("Discarded changes and closed") && bar.text.contains("one.rs"));
}

/// Closing the last remaining buffer yields a coherent empty composition state.
#[tokio::test]
async fn closing_last_buffer_leaves_coherent_empty_state() {
    let view = Arc::new(CloseFlowViewStub::new()) as Arc<dyn ports::WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // Fake service reporting single opened buffer
    struct OneBufService;
    impl ports::WorkspaceService for OneBufService {
        fn boot_workspace(
            &self,
            _req: ports::WorkspaceBootRequest,
        ) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(
            &self,
            _req: ports::OpenBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(
            &self,
            _req: ports::ListBuffersRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>>
        {
            let buf = BufferId::from("buf:last");
            Box::pin(async move {
                Ok(ports::ListBuffersResponse {
                    buffer_ids: vec![buf.clone()],
                    active_buffer: Some(buf.clone()),
                })
            })
        }
        fn set_active_buffer(
            &self,
            _req: ports::SetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(
            &self,
            _req: ports::GetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_cursor(
            &self,
            _req: ports::SetEditorCursorRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(
            &self,
            _req: ports::SetSelectionRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(
            &self,
            _req: ports::ClearSelectionRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(
            &self,
            _req: ports::GetEditorStateRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(
            &self,
            _req: ports::SetViewportRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(
            &self,
            _req: ports::ScrollViewportRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(
            &self,
            _req: ports::GetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(
            &self,
            _req: ports::DispatchCommandRequest,
        ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(
            &self,
            _req: ports::UpdateBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(
            &self,
            _req: ports::ApplyTextTransactionRequest,
        ) -> ports::BoxFuture<
            'static,
            Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(ports::ApplyTextTransactionResponse {
                    ok: true,
                    state: ports::EditorState {
                        cursor: ports::EditorCursor::zero(),
                        selection: None,
                    },
                    content: None,
                })
            })
        }
        fn get_recent_commands(
            &self,
            _req: ports::GetRecentCommandsRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }
        fn get_recent_events(
            &self,
            _req: ports::GetRecentEventsRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) })
        }
        fn get_session_snapshot(
            &self,
            _req: ports::GetSessionSnapshotRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn create_checkpoint(
            &self,
            _req: ports::CreateCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn save_checkpoint(
            &self,
            _req: ports::SaveCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(
            &self,
            _req: ports::LoadCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(
            &self,
            _req: ports::RestoreCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
    }

    let svc = std::sync::Arc::new(OneBufService) as std::sync::Arc<dyn ports::WorkspaceService>;
    // refresh to populate composition metadata via the fake service
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(svc.clone()))
        .await
        .expect("refresh ok");

    let buf = BufferId::from("buf:last");
    comp.set_pending_close(iface::PendingClose::BufferClose {
        buffer_id: buf.clone(),
        display: Some("last.rs".to_string()),
        dirty: true,
    });
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");

    // Composition should be coherent: no opened buffers, no active buffer, no stale active details.
    let obs = comp.latest_opened_buffers_summary();
    assert_eq!(obs.count, 0, "opened buffers should be empty after closing the last buffer");
    assert!(obs.active.is_none(), "active buffer should be None after closing the last buffer");
    assert!(
        comp.latest_active_buffer_details().is_none(),
        "no stale active buffer details should remain"
    );

    let bar = comp.latest_status_bar_line().expect("status present");
    assert!(bar.text.contains("Saved and closed") && bar.text.contains("last.rs"));
}

/// Confirm-cancel should clear pending state and leave buffers unchanged.
#[tokio::test]
async fn confirm_cancel_close_clears_pending_without_closing() {
    let view = Arc::new(CloseFlowViewStub::new()) as Arc<dyn ports::WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // Fake service reporting two opened buffers
    struct TwoBufSvcForCancel;
    impl ports::WorkspaceService for TwoBufSvcForCancel {
        fn boot_workspace(
            &self,
            _req: ports::WorkspaceBootRequest,
        ) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(
            &self,
            _req: ports::OpenBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn list_open_buffers(
            &self,
            _req: ports::ListBuffersRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>>
        {
            let buf1 = BufferId::from("buf:a");
            let buf2 = BufferId::from("buf:b");
            Box::pin(async move {
                Ok(ports::ListBuffersResponse {
                    buffer_ids: vec![buf1.clone(), buf2.clone()],
                    active_buffer: Some(buf1.clone()),
                })
            })
        }
        fn set_active_buffer(
            &self,
            _req: ports::SetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn get_active_buffer(
            &self,
            _req: ports::GetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_cursor(
            &self,
            _req: ports::SetEditorCursorRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(
            &self,
            _req: ports::SetSelectionRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(
            &self,
            _req: ports::ClearSelectionRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(
            &self,
            _req: ports::GetEditorStateRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(
            &self,
            _req: ports::SetViewportRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(
            &self,
            _req: ports::ScrollViewportRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(
            &self,
            _req: ports::GetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(
            &self,
            _req: ports::DispatchCommandRequest,
        ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(
            &self,
            _req: ports::UpdateBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(
            &self,
            _req: ports::ApplyTextTransactionRequest,
        ) -> ports::BoxFuture<
            'static,
            Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(ports::ApplyTextTransactionResponse {
                    ok: true,
                    state: ports::EditorState {
                        cursor: ports::EditorCursor::zero(),
                        selection: None,
                    },
                    content: None,
                })
            })
        }
        fn get_recent_commands(
            &self,
            _req: ports::GetRecentCommandsRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }
        fn get_recent_events(
            &self,
            _req: ports::GetRecentEventsRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) })
        }
        fn get_session_snapshot(
            &self,
            _req: ports::GetSessionSnapshotRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn create_checkpoint(
            &self,
            _req: ports::CreateCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn save_checkpoint(
            &self,
            _req: ports::SaveCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(
            &self,
            _req: ports::LoadCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(
            &self,
            _req: ports::RestoreCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
    }

    let svc =
        std::sync::Arc::new(TwoBufSvcForCancel) as std::sync::Arc<dyn ports::WorkspaceService>;
    // refresh to populate composition metadata via the fake service
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, Some(svc.clone()))
        .await
        .expect("refresh ok");

    let buf1 = BufferId::from("buf:a");
    comp.set_pending_close(iface::PendingClose::BufferClose {
        buffer_id: buf1.clone(),
        display: Some("a.rs".to_string()),
        dirty: true,
    });
    assert!(comp.has_pending_close());

    let _ = actions::confirm_cancel_close(&mut comp).await.expect("confirm cancel ok");

    // Pending cleared, but buffers untouched and active remains the same.
    assert!(!comp.has_pending_close(), "pending close should be cleared after cancel");
    let obs = comp.latest_opened_buffers_summary();
    assert_eq!(obs.count, 2, "no buffers should have been closed by cancel");
    assert_eq!(obs.active.unwrap(), buf1, "active buffer should remain unchanged after cancel");

    // Status banner should reflect cancellation or remain coherent (we expect a cancel message).
    let bar = comp.latest_status_bar_line().expect("status present");
    assert!(
        bar.text.contains("Close cancelled") || !bar.text.contains("pending-close"),
        "status/banner should be coherent after cancel"
    );
}
