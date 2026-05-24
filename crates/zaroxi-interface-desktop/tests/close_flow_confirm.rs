mod close_flow_common;
use std::sync::Arc;
use close_flow_common::CloseFlowViewStub;
use zaroxi_application_workspace::ports::SessionId;
use zaroxi_interface_desktop::{DesktopComposition, actions, refresh_desktop};

#[tokio::test]
async fn confirm_save_and_close_clears_pending_and_sets_status() {
    let view = Arc::new(CloseFlowViewStub::new()) as Arc<dyn zaroxi_application_workspace::ports::WorkspaceView>;
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
    let bar = comp.latest_status_bar_line().expect("status bar present after save");
    assert!(bar.text.contains("Saved and closed"), "status should reflect save success");
}
