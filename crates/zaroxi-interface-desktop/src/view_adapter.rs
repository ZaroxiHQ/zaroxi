/*!
Thin adapter seam: expose application renderable lines to the interface-desktop crate.

Purpose:
- Request visible lines / active editor document from the application WorkspaceView.
- Reuse the application projection (project_renderable_lines) to obtain RenderableLine/RenderSpan.
- Map application renderable types into a tiny, read-only interface-facing DTO.
- Keep this adapter minimal and deterministic: no layout, styling, or rendering logic here.
*/

use std::sync::Arc;

use zaroxi_application_workspace::ports::{
    GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, WorkspaceView,
};
use zaroxi_application_workspace::view::{
    project_renderable_lines, RenderableLine as AppRenderableLine, SpanKind as AppSpanKind,
};

/// Interface-facing span kind (very small, read-only).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InterfaceSpanKind {
    Normal,
    Selection,
    Cursor,
    SelectionCursor,
}

/// Interface-facing render span.
#[derive(Clone, Debug)]
pub struct InterfaceRenderSpan {
    pub kind: InterfaceSpanKind,
    pub text: String,
    pub start_col: usize,
    pub end_col: usize,
}

/// Interface-facing renderable line.
#[derive(Clone, Debug)]
pub struct InterfaceRenderableLine {
    pub line_number: usize,
    pub spans: Vec<InterfaceRenderSpan>,
    pub total_columns: usize,
}

/// Interface-facing rendered window.
#[derive(Clone, Debug)]
pub struct InterfaceRenderableWindow {
    pub top_line: usize,
    pub total_lines: usize,
    pub lines: Vec<InterfaceRenderableLine>,
}

/// Fetch the renderable window for the active editor in the given session.
///
/// - `view`: an Arc'd WorkspaceView (application-provided).
/// - `session_id`: typed session id.
///
/// This function:
/// 1. Reads the active editor document to ensure a buffer is active (and to allow callers
///    to set viewports prior to calling this function).
/// 2. Requests the VisibleLinesWindow via `get_visible_lines`.
/// 3. Calls application::view::project_renderable_lines to compute character-span render model.
/// 4. Maps application render model into the small interface DTOs.
///
/// Returns an error string on failure.
pub async fn fetch_renderable_window(
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
) -> Result<InterfaceRenderableWindow, String> {
    // 1) Resolve active editor document (to ensure there's an active buffer and to fail fast).
    let doc_resp = view
        .get_active_editor_document(GetActiveEditorDocumentRequest {
            session_id: session_id.clone(),
        })
        .await
        .map_err(|e| e.to_string())?;

    let _doc = doc_resp.document; // currently not needed beyond validation, kept for clarity.

    // 2) Request visible lines (the application orchestrator will use stored viewport state).
    let vis_resp = view
        .get_visible_lines(GetVisibleLinesRequest {
            session_id: session_id.clone(),
            buffer_id: _doc.buffer_id.clone(),
        })
        .await
        .map_err(|e| e.to_string())?;

    let visible_window = vis_resp.window;

    // 3) Reuse application projection to get renderable lines (character-indexed spans).
    let app_renderable: Vec<AppRenderableLine> = project_renderable_lines(&visible_window);

    // 4) Map application renderable model to interface DTOs.
    let mut lines: Vec<InterfaceRenderableLine> = Vec::with_capacity(app_renderable.len());
    for ar in app_renderable.into_iter() {
        let mut spans: Vec<InterfaceRenderSpan> = Vec::with_capacity(ar.spans.len());
        for s in ar.spans.into_iter() {
            let kind = match s.kind {
                AppSpanKind::Normal => InterfaceSpanKind::Normal,
                AppSpanKind::Selection => InterfaceSpanKind::Selection,
                AppSpanKind::Cursor => InterfaceSpanKind::Cursor,
                AppSpanKind::SelectionCursor => InterfaceSpanKind::SelectionCursor,
            };
            spans.push(InterfaceRenderSpan {
                kind,
                text: s.text,
                start_col: s.start_col,
                end_col: s.end_col,
            });
        }
        lines.push(InterfaceRenderableLine { line_number: ar.line_number, spans, total_columns: ar.total_columns });
    }

    Ok(InterfaceRenderableWindow {
        top_line: visible_window.top_line,
        total_lines: visible_window.total_lines,
        lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use zaroxi_application_workspace::ports::{WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, EditorDocument, EditorCursor};
    use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
    use zaroxi_core_editor_buffer::ports::BufferId;

    /// Minimal in-test WorkspaceView stub that returns a tiny document and a prebuilt visible window.
    struct FakeView {
        doc: EditorDocument,
        window: VisibleLinesWindow,
    }

    impl FakeView {
        fn new() -> Self {
            // Build a simple document with one line "abcd" and cursor at col 2.
            let content = Some("abcd".to_string());
            let ed = EditorDocument {
                buffer_id: BufferId::from("buf:fake"),
                content: content.clone(),
                cursor: EditorCursor { line: 0, column: 2 },
                selection: None,
                line_count: 1,
                current_line: content.and_then(|c| c.lines().nth(0).map(|s| s.to_string())),
            };

            // Build a VisibleLinesWindow of one line (it mirrors small projection semantics).
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
        fn get_buffer_content(&self, _buffer_id: crate::ports::BufferId) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
            Box::pin(async move { Ok(Some("".to_string())) })
        }

        fn get_active_buffer_content(&self, _session_id: crate::ports::SessionId) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>> {
            Box::pin(async move { Ok(Some("".to_string())) })
        }

        fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetActiveEditorDocumentResponse, crate::ports::UseCaseError>> {
            let d = self.doc.clone();
            Box::pin(async move { Ok(crate::ports::GetActiveEditorDocumentResponse { document: d }) })
        }

        fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> crate::ports::BoxFuture<'static, Result<crate::ports::GetVisibleLinesResponse, crate::ports::UseCaseError>> {
            let w = self.window.clone();
            Box::pin(async move { Ok(crate::ports::GetVisibleLinesResponse { window: w }) })
        }
    }

    #[tokio::test]
    async fn adapter_maps_renderable_window() {
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let res = fetch_renderable_window(arc, sid).await.expect("fetch ok");
        assert_eq!(res.total_lines, 1);
        assert_eq!(res.lines.len(), 1);
        let rl = &res.lines[0];
        assert_eq!(rl.line_number, 1);
        // expect a zero-width cursor span (empty text) or a Cursor span somewhere
        let has_cursor = rl.spans.iter().any(|s| s.kind == InterfaceSpanKind::Cursor || s.kind == InterfaceSpanKind::SelectionCursor);
        assert!(has_cursor);
    }

    #[tokio::test]
    async fn presenter_refresh_stores_window() {
        use crate::presenter::Presenter;
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let mut p = Presenter::new();
        p.refresh(arc, sid).await.expect("refresh ok");
        let win = p.latest().expect("window present");
        assert_eq!(win.total_lines, 1);
    }
}
