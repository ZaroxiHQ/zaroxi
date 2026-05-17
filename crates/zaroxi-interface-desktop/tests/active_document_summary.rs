use std::sync::{Arc, Mutex};

use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_application_workspace::ports::{GetActiveEditorDocumentRequest, GetVisibleLinesRequest, SessionId, EditorDocument, EditorCursor, GetActiveEditorDocumentResponse, GetVisibleLinesResponse};
use zaroxi_application_workspace::view::{VisibleLine, VisibleLinesWindow};
use zaroxi_application_workspace::ports::WorkspaceView;
use zaroxi_kernel_types::Id;

/// Minimal fake view that exposes a mutable EditorDocument so tests can mutate cursor/content.
struct MutableFakeView {
    inner: Arc<Mutex<EditorDocument>>,
    window: Arc<Mutex<VisibleLinesWindow>>,
}

impl MutableFakeView {
    fn new(buffer_id: BufferId, content: Option<String>, cursor: EditorCursor) -> Self {
        let line_text = content.clone().unwrap_or_default();
        let vl = VisibleLine {
            line_number: 1,
            text: line_text.clone(),
            is_cursor_line: true,
            cursor_column: Some(cursor.column as usize),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = VisibleLinesWindow { top_line: 1, total_lines: content.as_ref().map(|s| s.lines().count()).unwrap_or(0), lines: vec![vl] };
        let doc = EditorDocument {
            buffer_id,
            content,
            cursor,
            selection: None,
            line_count: vw.total_lines,
            current_line: None,
        };
        Self { inner: Arc::new(Mutex::new(doc)), window: Arc::new(Mutex::new(vw)) }
    }

    fn set_cursor(&self, cursor: EditorCursor) {
        {
            let mut d = self.inner.lock().unwrap();
            d.cursor = cursor.clone();
        }
        // Update the visible window to reflect the cursor change.
        if let Ok(mut w) = self.window.lock() {
            if let Some(line) = w.lines.get_mut(0) {
                line.is_cursor_line = true;
                line.cursor_column = Some(cursor.column as usize);
            }
        }
    }

    fn set_content(&self, content: Option<String>) {
        {
            let mut d = self.inner.lock().unwrap();
            d.content = content.clone();
        }
        // Keep the visible window in-sync with the underlying document snapshot.
        if let Ok(mut w) = self.window.lock() {
            let txt = content.clone().unwrap_or_default();
            if let Some(line) = w.lines.get_mut(0) {
                line.text = txt.clone();
                // If no explicit cursor info, preserve existing cursor_column (or default to 0).
                if line.cursor_column.is_none() {
                    line.cursor_column = Some(0);
                }
            }
            w.total_lines = content.as_ref().map(|s| s.lines().count()).unwrap_or(0);
        }
    }
}

impl WorkspaceView for MutableFakeView {
    fn get_buffer_content(&self, _buffer_id: zaroxi_application_workspace::ports::BufferId) -> zaroxi_application_workspace::ports::BoxFuture<'static, Result<Option<String>, zaroxi_application_workspace::ports::UseCaseError>> {
        let d = self.inner.lock().unwrap().content.clone();
        Box::pin(async move { Ok(d) })
    }

    fn get_active_buffer_content(&self, _session_id: zaroxi_application_workspace::ports::SessionId) -> zaroxi_application_workspace::ports::BoxFuture<'static, Result<Option<String>, zaroxi_application_workspace::ports::UseCaseError>> {
        let d = self.inner.lock().unwrap().content.clone();
        Box::pin(async move { Ok(d) })
    }

    fn get_active_editor_document(&self, _req: GetActiveEditorDocumentRequest) -> zaroxi_application_workspace::ports::BoxFuture<'static, Result<GetActiveEditorDocumentResponse, zaroxi_application_workspace::ports::UseCaseError>> {
        let d = self.inner.lock().unwrap().clone();
        Box::pin(async move { Ok(GetActiveEditorDocumentResponse { document: d }) })
    }

    fn get_visible_lines(&self, _req: GetVisibleLinesRequest) -> zaroxi_application_workspace::ports::BoxFuture<'static, Result<GetVisibleLinesResponse, zaroxi_application_workspace::ports::UseCaseError>> {
        let w = self.window.lock().unwrap().clone();
        Box::pin(async move { Ok(GetVisibleLinesResponse { window: w }) })
    }
}

#[tokio::test]
async fn summary_reflects_initial_open_and_cursor() {
    let v = MutableFakeView::new(BufferId::from("buf:fake"), Some("abcd".to_string()), EditorCursor { line: 0, column: 2 });
    let arc: Arc<dyn WorkspaceView> = Arc::new(v);
    let sid = SessionId(Id::new());
    let mut comp = DesktopComposition::new();

    comp.refresh(arc.clone(), sid.clone(), None).await.expect("refresh ok");
    let sum = comp.latest_active_document_summary().expect("summary present");
    assert_eq!(sum.display.unwrap(), "fake".to_string());
    assert_eq!(sum.line_count, 1);
    assert_eq!(sum.cursor_line, Some(1));
    assert_eq!(sum.cursor_column, Some(2));
    assert!(!sum.selection_present);
    assert!(sum.current_line_snippet.unwrap().contains("abcd"));
}

#[tokio::test]
async fn summary_updates_after_cursor_change() {
    let v = MutableFakeView::new(BufferId::from("buf:fake"), Some("hello world".to_string()), EditorCursor { line: 0, column: 5 });
    let arc_v = Arc::new(v);
    let view_clone = arc_v.clone() as Arc<dyn WorkspaceView>;
    let sid = SessionId(Id::new());
    let mut comp = DesktopComposition::new();

    // initial refresh
    comp.refresh(view_clone.clone(), sid.clone(), None).await.expect("refresh ok");
    let s1 = comp.latest_active_document_summary().expect("summary present");
    assert_eq!(s1.cursor_column, Some(5));

    // mutate cursor in the underlying view and refresh again
    let concrete: &MutableFakeView = unsafe { &*(&*arc_v as *const _ as *const MutableFakeView) };
    concrete.set_cursor(EditorCursor { line: 0, column: 0 });
    comp.refresh(view_clone.clone(), sid.clone(), None).await.expect("refresh ok");
    let s2 = comp.latest_active_document_summary().expect("summary present");
    assert_eq!(s2.cursor_column, Some(0));
}

#[tokio::test]
async fn summary_updates_after_content_change_and_buffer_switch() {
    let v = MutableFakeView::new(BufferId::from("buf:one"), Some("line1".to_string()), EditorCursor { line: 0, column: 0 });
    let arc_v = Arc::new(v);
    let view_clone = arc_v.clone() as Arc<dyn WorkspaceView>;
    let sid = SessionId(Id::new());
    let mut comp = DesktopComposition::new();

    comp.refresh(view_clone.clone(), sid.clone(), None).await.expect("refresh ok");
    let s1 = comp.latest_active_document_summary().expect("summary present");
    assert_eq!(s1.display.unwrap(), "one".to_string());

    // mutate content and buffer id (simulate open/switch)
    let concrete: &MutableFakeView = unsafe { &*(&*arc_v as *const _ as *const MutableFakeView) };
    concrete.set_content(Some("first\nsecond\nthird".to_string()));
    // Manually change internal buffer id to simulate a switch (testing projection behavior)
    {
        let mut d = concrete.inner.lock().unwrap();
        d.buffer_id = BufferId::from("buf:two");
    }
    comp.refresh(view_clone.clone(), sid.clone(), None).await.expect("refresh ok");
    let s2 = comp.latest_active_document_summary().expect("summary present");
    assert_eq!(s2.display.unwrap(), "two".to_string());
    assert!(s2.line_count >= 1);
}
