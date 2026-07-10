//! Content-identity guard for the refresh path.
//!
//! `refresh_with_service` fetches the `visible_window` projection for the
//! WorkspaceView's active editor document.  When the AUTHORITATIVE active
//! buffer is a DIFFERENT document (e.g. a direct/large-file tab, or a
//! post-close fallback whose identity was set independently of the view), that
//! window holds the WRONG file's text and MUST be dropped so
//! `build_work_content` never emits it as the authoritative file's
//! `editor_body`.  This is the exact "path says file A but bytes come from file
//! B" divergence, at the refresh seam.

use std::sync::Arc;

use zaroxi_application_workspace::ports::{
    EditorCursor, EditorDocument, GetActiveEditorDocumentRequest, GetActiveEditorDocumentResponse,
    GetVisibleLinesRequest, GetVisibleLinesResponse, SessionId, WorkspaceView,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_kernel_types::Id;

/// Minimal WorkspaceView stub: active editor document "buf:fake" (A) with a
/// two-line visible window.
struct FakeView {
    doc: EditorDocument,
    window: VisibleLinesWindow,
}

impl FakeView {
    fn new() -> Self {
        let content = Some("alpha\nbeta".to_string());
        let doc = EditorDocument {
            buffer_id: BufferId::from("buf:fake"),
            content: content.clone(),
            cursor: EditorCursor { line: 0, column: 0 },
            selection: None,
            line_count: 2,
            current_line: content.as_ref().and_then(|c| c.lines().next().map(|s| s.to_string())),
        };
        let window = VisibleLinesWindow {
            top_line: 1,
            total_lines: 2,
            lines: vec![
                VisibleLine {
                    line_number: 1,
                    text: "alpha".to_string(),
                    is_cursor_line: true,
                    cursor_column: Some(0),
                    selection_intersects: false,
                    selection_start_column: None,
                    selection_end_column: None,
                },
                VisibleLine {
                    line_number: 2,
                    text: "beta".to_string(),
                    is_cursor_line: false,
                    cursor_column: None,
                    selection_intersects: false,
                    selection_start_column: None,
                    selection_end_column: None,
                },
            ],
        };
        FakeView { doc, window }
    }
}

impl WorkspaceView for FakeView {
    fn get_buffer_content(
        &self,
        _buffer_id: zaroxi_application_workspace::ports::BufferId,
    ) -> zaroxi_application_workspace::ports::BoxFuture<
        'static,
        Result<Option<String>, zaroxi_application_workspace::ports::UseCaseError>,
    > {
        Box::pin(async move { Ok(Some(String::new())) })
    }

    fn get_active_buffer_content(
        &self,
        _session_id: zaroxi_application_workspace::ports::SessionId,
    ) -> zaroxi_application_workspace::ports::BoxFuture<
        'static,
        Result<Option<String>, zaroxi_application_workspace::ports::UseCaseError>,
    > {
        Box::pin(async move { Ok(Some(String::new())) })
    }

    fn get_active_editor_document(
        &self,
        _req: GetActiveEditorDocumentRequest,
    ) -> zaroxi_application_workspace::ports::BoxFuture<
        'static,
        Result<GetActiveEditorDocumentResponse, zaroxi_application_workspace::ports::UseCaseError>,
    > {
        let d = self.doc.clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(
        &self,
        _req: GetVisibleLinesRequest,
    ) -> zaroxi_application_workspace::ports::BoxFuture<
        'static,
        Result<GetVisibleLinesResponse, zaroxi_application_workspace::ports::UseCaseError>,
    > {
        let w = self.window.clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

#[tokio::test]
async fn refresh_drops_foreign_visible_window_on_authoritative_mismatch() {
    let arc: Arc<dyn WorkspaceView> = Arc::new(FakeView::new());
    let sid = SessionId(Id::new());
    let wid = Id::new();

    let mut comp = DesktopComposition::new();
    // Register a DIRECT (large-file) buffer B and make it the authoritative
    // active buffer — a DIFFERENT document than the view's active editor doc A.
    comp.add_opened_buffer_direct(BufferId::from("buf:/w/other.rs"), Some("other.rs".to_string()));

    comp.refresh_with_service(arc.clone(), sid.clone(), Some(wid), None).await.expect("refresh ok");

    let meta = comp.latest_metadata().expect("metadata present");
    // Authoritative active is the direct buffer B, NOT the view's A.
    assert_eq!(
        meta.active_buffer.as_ref().map(|b| b.to_string()).as_deref(),
        Some("buf:/w/other.rs"),
        "direct buffer must be authoritative active",
    );
    // The visible window fetched for A must be DROPPED because it belongs to a
    // different document than the authoritative active buffer.
    assert!(
        meta.visible_window.is_none(),
        "foreign visible_window (belonging to the view's active doc, not the \
         authoritative buffer) must be dropped on authoritative mismatch",
    );
}

#[tokio::test]
async fn refresh_keeps_visible_window_when_authoritative_matches_view() {
    let arc: Arc<dyn WorkspaceView> = Arc::new(FakeView::new());
    let sid = SessionId(Id::new());
    let wid = Id::new();

    // No direct buffer: the view's active doc ("buf:fake") is authoritative.
    let mut comp = DesktopComposition::new();
    comp.refresh_with_service(arc.clone(), sid.clone(), Some(wid), None).await.expect("refresh ok");

    let meta = comp.latest_metadata().expect("metadata present");
    assert_eq!(meta.active_buffer.as_ref().map(|b| b.to_string()).as_deref(), Some("buf:fake"),);
    assert!(
        meta.visible_window.is_some(),
        "matching-identity visible_window must be preserved when identities agree",
    );
    let vw = meta.visible_window.unwrap();
    assert_eq!(vw.lines, vec!["alpha".to_string(), "beta".to_string()]);
}
