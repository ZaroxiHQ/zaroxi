/*!
Tiny desktop composition state (Phase 13).

Purpose:
- Provide a minimal read-only shell-level composition object that groups:
  - current session id,
  - optional active workspace id (when composition caller has it),
  - active editor presenter snapshot (via existing Presenter).
- Keep this strictly compositional: reuse Presenter and the view_adapter seam.
- No UI, rendering, layout, or editor policy is added here.

This file is intentionally small and focused on composition only.
*/

use std::sync::Arc;

use crate::presenter::Presenter;
use zaroxi_application_workspace::ports::{WorkspaceView, SessionId};
use zaroxi_kernel_types::Id;
use crate::view_adapter::InterfaceRenderableWindow;

/// Minimal read-only metadata projection exposed to the shell.
///
/// This small struct is intentionally tiny and shell-oriented. It captures a few
/// facts useful to the outer harness / interface without reimplementing application
/// snapshot logic.
#[derive(Clone, Debug)]
pub struct DesktopMetadata {
    /// Recorded session id (if composition was refreshed).
    pub session_id: Option<SessionId>,
    /// Optional workspace id associated with the session (if provided during refresh).
    pub workspace_id: Option<Id>,
    /// Currently active buffer id when available (application-provided).
    pub active_buffer: Option<crate::ports::BufferId>,
    /// Tiny opened buffers count projection. For Phase 19 this is computed conservatively:
    ///  - 1 when an active editor document exists, 0 otherwise. This is a light-weight,
    ///    shell-facing projection that avoids expanding the interface surface.
    pub opened_buffer_count: usize,
}

/// Minimal desktop-level composition state.
///
/// Mostly read-only: composition is updated via `refresh` which delegates to the
/// existing Presenter. The struct exposes simple accessors for harnesses or
/// thin interface glue to print or inspect the current shell-level state.
#[derive(Clone, Debug)]
pub struct DesktopComposition {
    presenter: Presenter,
    /// Typed session id for the active UI session.
    pub session_id: Option<SessionId>,
    /// Optional workspace id associated with the session (if known to caller).
    pub workspace_id: Option<Id>,
    /// Small cached metadata projection for shell consumption.
    metadata: Option<DesktopMetadata>,
}

impl DesktopComposition {
    /// Create a new empty composition.
    pub fn new() -> Self {
        Self {
            presenter: Presenter::new(),
            session_id: None,
            workspace_id: None,
            metadata: None,
        }
    }

    /// Refresh composition by asking the Presenter to refresh its snapshot.
    ///
    /// - `view`: application-provided read-only WorkspaceView (Arc'd).
    /// - `session_id`: typed session id to query active editor/presenter.
    /// - `workspace_id`: optional workspace id (caller-supplied) to be recorded in composition.
    ///
    /// The function delegates to presenter which uses the adapter seam to compute the renderable window.
    /// Additionally it queries the application read-path `get_active_editor_document` to populate
    /// a small, read-only metadata projection for the shell. Errors from the optional metadata
    /// query are tolerated: when the application reports no active editor the metadata will reflect that.
    pub async fn refresh(
        &mut self,
        view: Arc<dyn WorkspaceView>,
        session_id: SessionId,
        workspace_id: Option<Id>,
    ) -> Result<(), String> {
        // 1) Refresh presenter snapshot (reuses adapter seam and existing projection).
        self.presenter.refresh(view.clone(), session_id.clone()).await?;

        // 2) Attempt to read the active editor document via the WorkspaceView seam.
        //    This is the existing application read pathway and avoids adding new ports.
        let active_buf_opt = match view.get_active_editor_document(crate::ports::GetActiveEditorDocumentRequest { session_id: session_id.clone() }).await {
            Ok(resp) => Some(resp.document.buffer_id.clone()),
            Err(_) => None,
        };

        // Conservative opened buffer count: 1 if active buffer exists, otherwise 0.
        let opened_count = if active_buf_opt.is_some() { 1 } else { 0 };

        // 3) Update composition metadata and simple recorded ids.
        self.session_id = Some(session_id.clone());
        self.workspace_id = workspace_id;
        self.metadata = Some(DesktopMetadata {
            session_id: Some(session_id),
            workspace_id: self.workspace_id.clone(),
            active_buffer: active_buf_opt,
            opened_buffer_count: opened_count,
        });

        Ok(())
    }

    /// Get the latest renderable window from the underlying presenter (if any).
    pub fn latest_window(&self) -> Option<InterfaceRenderableWindow> {
        self.presenter.latest()
    }

    /// Get the recorded session id (if composition was refreshed).
    pub fn get_session_id(&self) -> Option<SessionId> {
        self.session_id.clone()
    }

    /// Get the recorded workspace id (if provided during refresh).
    pub fn get_workspace_id(&self) -> Option<Id> {
        self.workspace_id.clone()
    }

    /// Return the small, read-only metadata projection for shell consumption.
    pub fn latest_metadata(&self) -> Option<DesktopMetadata> {
        self.metadata.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use zaroxi_application_workspace::ports::{
        WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, GetActiveEditorDocumentResponse, GetVisibleLinesResponse, EditorDocument, EditorCursor,
    };
    use zaroxi_core_editor_buffer::ports::BufferId;
    use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};

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

            // Build a VisibleLinesWindow of one line.
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
    async fn desktop_composition_refreshes_and_stores_metadata() {
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let wid = zaroxi_kernel_types::Id::new();

        let mut comp = DesktopComposition::new();
        comp.refresh(arc, sid.clone(), Some(wid.clone())).await.expect("refresh ok");

        assert_eq!(comp.get_session_id().unwrap(), sid);
        assert_eq!(comp.get_workspace_id().unwrap(), wid);
        let win = comp.latest_window().expect("window present");
        assert_eq!(win.total_lines, 1);
        assert_eq!(win.lines.len(), 1);

        // Verify tiny metadata projection populated from the application read-path.
        let meta = comp.latest_metadata().expect("metadata present");
        assert_eq!(meta.session_id.unwrap(), sid);
        assert_eq!(meta.workspace_id.unwrap(), wid);
        assert_eq!(meta.active_buffer.unwrap(), crate::ports::BufferId::from("buf:fake"));
        assert_eq!(meta.opened_buffer_count, 1);
    }
}
