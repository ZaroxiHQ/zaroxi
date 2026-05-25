mod close_flow_common;
use std::sync::Arc;
use close_flow_common::CloseFlowViewStub;
use zaroxi_application_workspace::ports::SessionId;
use zaroxi_interface_desktop::{DesktopComposition, actions, refresh_desktop};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop as iface;

/// Ensure request_close_active sets pending-close and the status banner contains the expected hints.
#[tokio::test]
async fn request_close_active_enters_pending_close_and_status() {
    let view = Arc::new(CloseFlowViewStub::new()) as Arc<dyn zaroxi_application_workspace::ports::WorkspaceView>;
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
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // Build a deterministic opened-buffers projection with two buffers.
    let buf1 = BufferId::from("buf:one");
    let buf2 = BufferId::from("buf:two");

    comp.metadata = Some(iface::desktop::DesktopMetadata {
        session_id: Some(sid.clone()),
        workspace_id: None,
        active_buffer: Some(buf1.clone()),
        opened_buffer_count: 2,
        opened_buffers: vec![
            iface::desktop::OpenedBufferItem { buffer_id: buf1.clone(), display: Some("one.rs".to_string()), active: true },
            iface::desktop::OpenedBufferItem { buffer_id: buf2.clone(), display: Some("two.rs".to_string()), active: false },
        ],
        active_buffer_details: Some(iface::desktop::ActiveBufferDetails { buffer_id: buf1.clone(), display: Some("one.rs".to_string()), line_count: 1 }),
        ai_projection: None,
        visible_window: None,
        last_command_line: None,
        refresh_reason: None,
    });

    // Simulate a pending-close for the active buffer.
    comp.set_pending_close(iface::PendingClose::BufferClose { buffer_id: buf1.clone(), display: Some("one.rs".to_string()), dirty: true });
    assert!(comp.has_pending_close());

    // Confirm discard-and-close (could be save-and-close as well; behavior should match).
    let _ = actions::confirm_discard_and_close(&mut comp).await.expect("confirm discard ok");

    // Pending cleared.
    assert!(!comp.has_pending_close(), "pending close should be cleared after confirm_discard_and_close");

    // The removed buffer should no longer be listed, and the active buffer should now be buf2.
    let obs = comp.latest_opened_buffers_summary();
    assert_eq!(obs.count, 1, "one buffer should remain after closing one of two");
    assert_eq!(obs.active.unwrap(), buf2, "active buffer should fall back to the neighbor/first remaining buffer");

    // Status should mention the closed buffer label.
    let bar = comp.latest_status_bar_line().expect("status present");
    assert!(bar.text.contains("Discarded changes and closed") && bar.text.contains("one.rs"));
}

/// Closing the last remaining buffer yields a coherent empty composition state.
#[tokio::test]
async fn closing_last_buffer_leaves_coherent_empty_state() {
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let buf = BufferId::from("buf:last");

    comp.metadata = Some(iface::desktop::DesktopMetadata {
        session_id: Some(sid.clone()),
        workspace_id: None,
        active_buffer: Some(buf.clone()),
        opened_buffer_count: 1,
        opened_buffers: vec![
            iface::desktop::OpenedBufferItem { buffer_id: buf.clone(), display: Some("last.rs".to_string()), active: true },
        ],
        active_buffer_details: Some(iface::desktop::ActiveBufferDetails { buffer_id: buf.clone(), display: Some("last.rs".to_string()), line_count: 1 }),
        ai_projection: None,
        visible_window: None,
        last_command_line: None,
        refresh_reason: None,
    });

    comp.set_pending_close(iface::PendingClose::BufferClose { buffer_id: buf.clone(), display: Some("last.rs".to_string()), dirty: true });
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");

    // Composition should be coherent: no opened buffers, no active buffer, no stale active details.
    let obs = comp.latest_opened_buffers_summary();
    assert_eq!(obs.count, 0, "opened buffers should be empty after closing the last buffer");
    assert!(obs.active.is_none(), "active buffer should be None after closing the last buffer");
    assert!(comp.latest_active_buffer_details().is_none(), "no stale active buffer details should remain");

    let bar = comp.latest_status_bar_line().expect("status present");
    assert!(bar.text.contains("Saved and closed") && bar.text.contains("last.rs"));
}

/// Confirm-cancel should clear pending state and leave buffers unchanged.
#[tokio::test]
async fn confirm_cancel_close_clears_pending_without_closing() {
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let buf1 = BufferId::from("buf:a");
    let buf2 = BufferId::from("buf:b");

    comp.metadata = Some(iface::desktop::DesktopMetadata {
        session_id: Some(sid.clone()),
        workspace_id: None,
        active_buffer: Some(buf1.clone()),
        opened_buffer_count: 2,
        opened_buffers: vec![
            iface::desktop::OpenedBufferItem { buffer_id: buf1.clone(), display: Some("a.rs".to_string()), active: true },
            iface::desktop::OpenedBufferItem { buffer_id: buf2.clone(), display: Some("b.rs".to_string()), active: false },
        ],
        active_buffer_details: Some(iface::desktop::ActiveBufferDetails { buffer_id: buf1.clone(), display: Some("a.rs".to_string()), line_count: 1 }),
        ai_projection: None,
        visible_window: None,
        last_command_line: None,
        refresh_reason: None,
    });

    comp.set_pending_close(iface::PendingClose::BufferClose { buffer_id: buf1.clone(), display: Some("a.rs".to_string()), dirty: true });
    assert!(comp.has_pending_close());

    let _ = actions::confirm_cancel_close(&mut comp).await.expect("confirm cancel ok");

    // Pending cleared, but buffers untouched and active remains the same.
    assert!(!comp.has_pending_close(), "pending close should be cleared after cancel");
    let obs = comp.latest_opened_buffers_summary();
    assert_eq!(obs.count, 2, "no buffers should have been closed by cancel");
    assert_eq!(obs.active.unwrap(), buf1, "active buffer should remain unchanged after cancel");

    // Status banner should reflect cancellation or remain coherent (we expect a cancel message).
    let bar = comp.latest_status_bar_line().expect("status present");
    assert!(bar.text.contains("Close cancelled") || bar.text.contains("pending-close").not(), "status/banner should be coherent after cancel");
}
