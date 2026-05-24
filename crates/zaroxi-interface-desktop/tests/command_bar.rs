use std::sync::Arc;
use zaroxi_interface_desktop::{DesktopComposition, actions};
use zaroxi_application_workspace::ports::{WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::ports as ports;

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
    fn get_buffer_content(&self, _buffer_id: BufferId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: SessionId) -> ports::BoxFuture<'static, Result<Option<String>, ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> ports::BoxFuture<'static, Result<ports::GetActiveEditorDocumentResponse, ports::UseCaseError>> {
        let doc = ports::EditorDocument {
            buffer_id: self.buffer_id.clone(),
            content: Some("line1".to_string()),
            cursor: ports::EditorCursor::zero(),
            selection: None,
            line_count: 1,
            current_line: Some("line1".to_string()),
        };
        Box::pin(async move { Ok(ports::GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> ports::BoxFuture<'static, Result<ports::GetVisibleLinesResponse, ports::UseCaseError>> {
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
        Box::pin(async move { Ok(ports::GetVisibleLinesResponse { window: vw }) })
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
}

#[tokio::test]
async fn execute_refresh_command_triggers_refresh() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // Execute refresh via action (no service required)
    comp.open_command_bar();
    let idx = comp.latest_command_bar().and_then(|cb| cb.commands.iter().position(|c| c == "Refresh")).unwrap_or(0);
    let ar = actions::execute_command_by_index(&mut comp, arc.clone(), None, sid.clone(), None, idx).await.expect("execute ok");
    assert!(ar.success);
    // Composition should record a refresh reason for this explicit refresh action.
    let rr = comp.latest_refresh_reason().expect("reason present");
    assert_eq!(rr, zaroxi_interface_desktop::desktop::RefreshReason::RefreshAction);
}

#[tokio::test]
async fn execute_request_close_enters_pending_close() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = actions::refresh_desktop(&mut comp, arc.clone(), sid.clone(), None, None).await.expect("refresh ok");

    comp.open_command_bar();
    let idx = comp.latest_command_bar().and_then(|cb| cb.commands.iter().position(|c| c == "Request close active")).unwrap();
    let ar = actions::execute_command_by_index(&mut comp, arc.clone(), None, sid.clone(), None, idx).await.expect("execute close request ok");
    assert!(ar.success);
    assert!(comp.has_pending_close(), "pending close should be set after request_close_active");
}
