mod close_flow_common;
use close_flow_common::CloseFlowViewStub;
use std::sync::Arc;
use zaroxi_application_workspace::ports::SessionId;
use zaroxi_interface_desktop::{DesktopComposition, actions, refresh_desktop};

#[tokio::test]
async fn confirm_save_and_close_clears_pending_and_sets_status() {
    let view = Arc::new(CloseFlowViewStub::new())
        as Arc<dyn zaroxi_application_workspace::ports::WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None)
        .await
        .expect("refresh ok");
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone())
        .await
        .expect("request close ok");
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");
    assert!(!comp.has_pending_close(), "pending close should be cleared after save-and-close");

    // The opened buffer should be removed and active cleared when it was the last buffer.
    let obs = comp.latest_opened_buffers_summary();
    assert_eq!(obs.count, 0, "opened buffers should be empty after closing the last buffer");
    assert!(obs.active.is_none(), "active buffer should be None after closing the last buffer");

    let bar = comp.latest_status_bar_line().expect("status bar present after save");
    assert!(bar.text.contains("Saved and closed"), "status should reflect save success");
    // When display is not present we fall back to buffer id; ensure the status mentions the closed buffer.
    assert!(bar.text.contains("buf:fake"), "status should include the closed buffer identifier");
}

#[tokio::test]
async fn confirm_discard_and_close_removes_buffer_and_sets_status() {
    let view = Arc::new(CloseFlowViewStub::new())
        as Arc<dyn zaroxi_application_workspace::ports::WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None)
        .await
        .expect("refresh ok");
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone())
        .await
        .expect("request close ok");
    assert!(comp.has_pending_close());

    let _ = actions::confirm_discard_and_close(&mut comp).await.expect("confirm discard ok");
    assert!(!comp.has_pending_close(), "pending close should be cleared after discard-and-close");

    let obs = comp.latest_opened_buffers_summary();
    assert_eq!(
        obs.count, 0,
        "opened buffers should be empty after discarding and closing the last buffer"
    );
    assert!(
        obs.active.is_none(),
        "active buffer should be None after discarding and closing the last buffer"
    );

    let bar = comp.latest_status_bar_line().expect("status bar present after discard");
    assert!(bar.text.contains("Discarded"), "status should reflect discard-and-close outcome");
    assert!(bar.text.contains("buf:fake"), "status should include the closed buffer identifier");
}
