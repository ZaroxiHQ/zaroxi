mod close_flow_common;
use std::sync::Arc;
use close_flow_common::CloseFlowViewStub;
use zaroxi_application_workspace::ports::SessionId;
use zaroxi_interface_desktop::{DesktopComposition, actions, refresh_desktop};

#[tokio::test]
async fn request_close_active_enters_pending_close_and_status_banner() {
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
