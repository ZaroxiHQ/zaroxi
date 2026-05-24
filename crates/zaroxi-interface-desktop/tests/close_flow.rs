use std::sync::Arc;

use zaroxi_interface_desktop::{DesktopComposition, refresh_desktop, actions};
use zaroxi_application_workspace::ports::{
    WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;

/// Minimal fake WorkspaceView used to populate a DesktopComposition for tests.
struct FakeView {
    buffer_id: BufferId,
}

impl FakeView {
    fn new() -> Self {
        Self { buffer_id: BufferId::from("buf:fake") }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(&self, _buffer_id: BufferId) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: SessionId) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetActiveEditorDocumentResponse, crate::ports::UseCaseError>> {
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

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetVisibleLinesResponse, crate::ports::UseCaseError>> {
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
async fn request_close_active_enters_pending_close_and_status_banner() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    // populate composition so latest_active_buffer_details is present
    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");

    // Request close: should set pending close and status banner should reflect it.
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close(), "pending close should be set after request_close_active");

    let bar = comp.latest_status_bar_line().expect("status bar present");
    let text = bar.text;
    assert!(text.contains("Close buffer"), "status banner should mention Close buffer");
    assert!(text.contains("unsaved changes") || text.contains("[S]ave"), "status banner should include action hints or unsaved indicator");
}

#[tokio::test]
async fn confirm_save_and_close_clears_pending_and_sets_status() {
    let view = Arc::new(FakeView::new()) as Arc<dyn WorkspaceView>;
    let sid = SessionId(zaroxi_kernel_types::Id::new());
    let mut comp = DesktopComposition::new();

    let _ = refresh_desktop(&mut comp, view.clone(), sid.clone(), None, None).await.expect("refresh ok");
    let _ = actions::request_close_active(&mut comp, view.clone(), sid.clone()).await.expect("request close ok");
    assert!(comp.has_pending_close());

    let _ = actions::confirm_save_and_close(&mut comp).await.expect("confirm save ok");
    assert!(!comp.has_pending_close(), "pending close should be cleared after save-and-close");
    let bar = comp.latest_status_bar_line().expect("status bar present after save");
    assert!(bar.text.contains("Saved and closed"), "status should reflect save success");
}
