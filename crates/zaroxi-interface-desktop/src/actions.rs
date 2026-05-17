/*!
Tiny action seam: refresh desktop composition.

Architectural rationale (Phase 14 - minimal desktop action flow):
- Provide a tiny, explicit action in the interface layer that composes existing
  seams (WorkspaceView, Presenter, DesktopComposition) to refresh the active
  desktop composition snapshot.
- Keep this strictly orchestration-only: do not duplicate any editor logic,
  do not modify application ports, and avoid introducing broader controller
  abstractions or event buses.
- The action delegates to DesktopComposition::refresh which already uses the
  Presenter + adapter seam (view_adapter) to obtain the renderable window.
- This lets external harnesses or potential future UI shells call a single
  intent-focused function to update presenter/composition state.

Public API:
- pub async fn refresh_desktop(
      comp: &mut DesktopComposition,
      view: std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceView>,
      session_id: zaroxi_application_workspace::ports::SessionId,
      workspace_id: Option<zaroxi_kernel_types::Id>,
  ) -> Result<(), String>

The function is intentionally tiny and documented. Tests exercise the happy-path using
a small in-test WorkspaceView stub.
*/

use std::sync::Arc;

use zaroxi_application_workspace::ports::{WorkspaceView, SessionId};
use zaroxi_kernel_types::Id;

use crate::desktop::DesktopComposition;

/// Refresh the given DesktopComposition by delegating to its async `refresh` method.
///
/// Parameters:
/// - comp: mutable reference to an existing DesktopComposition instance (presenter state).
/// - view: Arc'd application WorkspaceView (read-only seam).
/// - session_id: typed SessionId for the active UI session.
/// - workspace_id: optional Workspace Id for caller metadata.
///
/// Returns:
/// - Ok(()) on success.
/// - Err(String) if the underlying presenter/composition refresh failed.
pub async fn refresh_desktop(
    comp: &mut DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
) -> Result<(), String> {
    comp.refresh(view, session_id, workspace_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use zaroxi_application_workspace::ports::{
        WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest,
        GetActiveEditorDocumentResponse, GetVisibleLinesResponse, EditorDocument, EditorCursor,
    };
    use zaroxi_core_editor_buffer::ports::BufferId;

    /// Minimal in-test WorkspaceView stub that returns a tiny document and a prebuilt visible window.
    struct FakeView {
        doc: EditorDocument,
        window: crate::super::view::VisibleLinesWindow,
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

            // Build a VisibleLinesWindow of one line.
            let vl = crate::super::view::VisibleLine {
                line_number: 1,
                text: "abcd".to_string(),
                is_cursor_line: true,
                cursor_column: Some(2),
                selection_intersects: false,
                selection_start_column: None,
                selection_end_column: None,
            };
            let vw = crate::super::view::VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };

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

        fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> crate::ports::BoxFuture<'static, Result<GetActiveEditorDocumentResponse, crate::ports::UseCaseError>> {
            let d = self.doc.clone();
            Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
        }

        fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> crate::ports::BoxFuture<'static, Result<GetVisibleLinesResponse, crate::ports::UseCaseError>> {
            let w = self.window.clone();
            Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
        }
    }

    #[tokio::test]
    async fn refresh_action_updates_composition() {
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let wid = zaroxi_kernel_types::Id::new();

        let mut comp = DesktopComposition::new();
        // Call the tiny action
        refresh_desktop(&mut comp, arc, sid.clone(), Some(wid.clone())).await.expect("refresh ok");

        assert_eq!(comp.get_session_id().unwrap(), sid);
        assert_eq!(comp.get_workspace_id().unwrap(), wid);
        let win = comp.latest_window().expect("window present");
        assert_eq!(win.total_lines, 1);
        assert_eq!(win.lines.len(), 1);
    }
}
