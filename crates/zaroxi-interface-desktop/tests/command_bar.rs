use std::sync::Arc;
use zaroxi_application_workspace::ports;
use zaroxi_application_workspace::ports::{
    GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, WorkspaceView,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop::{DesktopComposition, actions};

/// Minimal fake view used for command-bar tests.
struct FakeView {
    buffer_id: BufferId,
}

impl FakeView {
    fn new() -> Self {
        Self { buffer_id: BufferId::from("buf:fake") }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(
        &self,
        _buffer_id: crate::ports::BufferId,
    ) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(
        &self,
        _session_id: crate::ports::SessionId,
    ) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(
        &self,
        _req: GetActiveEditorDocumentRequest,
    ) -> crate::ports::BoxFuture<
        'static,
        Result<crate::ports::GetActiveEditorDocumentResponse, crate::ports::UseCaseError>,
    > {
        let doc = crate::ports::EditorDocument {
            buffer_id: self.buffer_id.clone(),
            content: Some("line1".to_string()),
            cursor: crate::ports::EditorCursor::zero(),
            selection: None,
            line_count: 1,
            current_line: Some("line1".to_string()),
        };
        Box::pin(async move { Ok(crate::ports::GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(
        &self,
        _req: GetVisibleLinesRequest,
    ) -> crate::ports::BoxFuture<
        'static,
        Result<crate::ports::GetVisibleLinesResponse, crate::ports::UseCaseError>,
    > {
        let vl = VisibleLine {
            line_number: 1,
            text: "line1".to_string(),
            is_cursor_line: true,
            cursor_column: Some(0),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };
        Box::pin(async move { Ok(crate::ports::GetVisibleLinesResponse { window: vw }) })
    }
}

#[tokio::test]
async fn command_bar_opens_and_lists_commands() {
    let mut comp = DesktopComposition::new();
    comp.open_command_bar();
    assert!(comp.is_command_bar_open());
    let cb = comp.latest_command_bar().expect("command bar present");
    assert!(cb.commands.iter().any(|c| c == "Refresh"));
    assert!(cb.commands.iter().any(|c| c == "Request close active"));

    // Keyboard-oriented: initial deterministic selection should be 0
    assert_eq!(cb.selected, 0);
}

#[tokio::test]
async fn keyboard_navigation_updates_selection() {
    let mut comp = DesktopComposition::new();
    let _ = actions::open_command_bar(&mut comp).await.expect("open ok");
    let initial = comp.latest_command_bar().expect("cb").selected;
    // move next
    let _ = actions::navigate_command_bar_next(&mut comp).await.expect("nav next ok");
    let after_next = comp.latest_command_bar().expect("cb").selected;
    assert_eq!(after_next, (initial + 1) % comp.latest_command_bar().unwrap().commands.len());

    // move prev (should wrap back to initial)
    let _ = actions::navigate_command_bar_prev(&mut comp).await.expect("nav prev ok");
    let after_prev = comp.latest_command_bar().expect("cb").selected;
    assert_eq!(after_prev, initial);
}

#[tokio::test]
async fn keyboard_confirm_executes_selected_command_request_close() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = actions::refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, None)
        .await
        .expect("refresh ok");

    // open command bar via keyboard action
    let _ = actions::open_command_bar(&mut comp).await.expect("open ok");
    // navigate to "Request close active"
    let target = comp
        .latest_command_bar()
        .and_then(|cb| cb.commands.iter().position(|c| c == "Request close active"))
        .unwrap();
    // compute steps forward from current selected
    let mut steps = (target + comp.latest_command_bar().unwrap().commands.len()
        - comp.latest_command_bar().unwrap().selected)
        % comp.latest_command_bar().unwrap().commands.len();
    while steps > 0 {
        let _ = actions::navigate_command_bar_next(&mut comp).await.expect("nav next ok");
        steps -= 1;
    }

    // confirm via keyboard action (no service required for request_close_active)
    let res = actions::confirm_selected_command(&mut comp, arc.clone(), None, sid.clone(), None)
        .await
        .expect("confirm ok");
    assert!(res.success);
    assert!(
        comp.has_pending_close(),
        "pending close should be set after confirming request close via keyboard"
    );
    // command bar should be closed after a successful confirm
    assert!(!comp.is_command_bar_open());
}

#[tokio::test]
async fn keyboard_escape_cancels_command_bar() {
    let mut comp = DesktopComposition::new();
    let _ = actions::open_command_bar(&mut comp).await.expect("open ok");
    assert!(comp.is_command_bar_open());
    let _ = actions::cancel_command_bar(&mut comp).await.expect("cancel ok");
    assert!(!comp.is_command_bar_open());
    assert!(comp.latest_command_bar().is_none() || !comp.is_command_bar_open());
}
