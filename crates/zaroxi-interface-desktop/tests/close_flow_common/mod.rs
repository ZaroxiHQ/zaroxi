use zaroxi_application_workspace::ports::{
    self, BoxFuture, GetActiveEditorDocumentRequest, GetActiveEditorDocumentResponse,
    GetVisibleLinesRequest, GetVisibleLinesResponse, SessionId, UseCaseError, WorkspaceView,
};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_core_editor_buffer::ports::BufferId;

/// Minimal single helper used across tests.
pub struct CloseFlowViewStub {
    pub buffer_id: BufferId,
}

impl CloseFlowViewStub {
    pub fn new() -> Self {
        Self { buffer_id: BufferId::from("buf:fake") }
    }
}

impl WorkspaceView for CloseFlowViewStub {
    fn get_buffer_content(
        &self,
        _buffer_id: BufferId,
    ) -> BoxFuture<'static, Result<Option<String>, UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_buffer_content(
        &self,
        _session_id: SessionId,
    ) -> BoxFuture<'static, Result<Option<String>, UseCaseError>> {
        Box::pin(async move { Ok(Some("".to_string())) })
    }

    fn get_active_editor_document(
        &self,
        _req: GetActiveEditorDocumentRequest,
    ) -> BoxFuture<'static, Result<GetActiveEditorDocumentResponse, UseCaseError>> {
        let doc = ports::EditorDocument {
            buffer_id: self.buffer_id.clone(),
            content: Some("line1".to_string()),
            cursor: ports::EditorCursor::zero(),
            selection: None,
            line_count: 1,
            current_line: Some("line1".to_string()),
        };
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: doc }) })
    }

    fn get_visible_lines(
        &self,
        _req: GetVisibleLinesRequest,
    ) -> BoxFuture<'static, Result<GetVisibleLinesResponse, UseCaseError>> {
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
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: vw }) })
    }
}
