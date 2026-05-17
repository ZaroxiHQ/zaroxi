use std::sync::Arc;
use zaroxi_kernel_types::Id;
use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_application_workspace::ports::{WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, GetActiveEditorDocumentResponse, GetVisibleLinesResponse, EditorDocument, EditorCursor};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use std::pin::Pin;
use std::future::Future;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Minimal in-test WorkspaceView used to populate presenter/composition.
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
            cursor: EditorCursor { line: 0, column: 2 },
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
    fn get_buffer_content(&self, _buffer_id: zaroxi_application_workspace::ports::BufferId) -> BoxFuture<'static, Result<Option<String>, zaroxi_application_workspace::ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(&self, _session_id: zaroxi_application_workspace::ports::SessionId) -> BoxFuture<'static, Result<Option<String>, zaroxi_application_workspace::ports::UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> BoxFuture<'static, Result<GetActiveEditorDocumentResponse, zaroxi_application_workspace::ports::UseCaseError>> {
        let d = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> BoxFuture<'static, Result<GetVisibleLinesResponse, zaroxi_application_workspace::ports::UseCaseError>> {
        let w = self.window.clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

#[tokio::test]
async fn shell_snapshot_present_after_refresh() {
    let v = FakeView::new();
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());
    let mut comp = DesktopComposition::new();

    comp.refresh(arc, sid.clone(), None).await.expect("refresh ok");

    let snap_opt = comp.latest_shell_snapshot();
    assert!(snap_opt.is_some());
    let snap = snap_opt.unwrap();

    // Basic coherence checks
    assert_eq!(snap.context.latest_revision, comp.latest_revision());
    // count should match the number of items in the opened_buffers projection
    assert_eq!(snap.opened_buffers.count, snap.opened_buffers.items.len());
    // Active document and viewport should be present for this simple view
    assert!(snap.active_document.is_some());
    assert!(snap.viewport.is_some());
}
