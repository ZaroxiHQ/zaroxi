/*!
Small runtime helper that ties a mapped Action to the existing action/refresh
path and returns adapted ShellRegions suitable for the GPU presenter.

This file intentionally lives inside `zaroxi-interface-desktop` and is
crate-local so the native binary can delegate to the canonical action
and refresh functions instead of duplicating logic.

Flow implemented here:
- Accept an EventBridge::Action
- Invoke the appropriate existing action/refresh helper (in `actions`)
- Obtain the refreshed shell-facing view model
- Adapt it into ShellRegions via a thin adapter from DesktopComposition metadata
- Return ShellRegions for the presenter to paint

This helper now wires a real DesktopComposition instance (keeps composition
logic in the application layer) while using a tiny in-process WorkspaceView
stub to provide visible-lines/doc responses. The goal is to ensure the GPU
binary renders the real composition snapshot rather than a hard-coded demo.
*/

use std::sync::{Arc, Mutex};
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use zaroxi_application_workspace::view;

use crate::events::Action;
use crate::presenters::model::{ShellRegions, GpuShellPresenter};
use crate::desktop::{DesktopComposition, DesktopMetadata};
use crate::ports::{WorkspaceView, GetActiveEditorDocumentRequest, GetVisibleLinesRequest, EditorDocument, EditorCursor};
use zaroxi_core_editor_buffer::ports::BufferId;
use crate::ports::SessionId;
use zaroxi_kernel_types::Id;

/// Minimal in-memory WorkspaceView used by the native binary to drive the
/// composition refresh path. This keeps the binary thin while ensuring the
/// real DesktopComposition refresh logic is exercised.
struct FakeView {
    // Active buffer id string (e.g. "buf:one")
    active_buffer: Mutex<String>,
    // Simple one-line document text for the active buffer.
    text: String,
}

impl FakeView {
    fn new(initial: &str, text: &str) -> Self {
        Self { active_buffer: Mutex::new(initial.to_string()), text: text.to_string() }
    }

    fn set_active_buffer(&self, id: &str) {
        let mut guard = self.active_buffer.lock().unwrap();
        *guard = id.to_string();
    }

    fn current_active_buffer(&self) -> String {
        self.active_buffer.lock().unwrap().clone()
    }
}

// Implement the application WorkspaceView trait using the small fake document.
// The implementation mirrors the small test stubs used elsewhere in this crate.
impl WorkspaceView for FakeView {
    fn get_buffer_content(
        &self,
        _buffer_id: crate::ports::BufferId,
    ) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>>
    {
        let txt = self.text.clone();
        Box::pin(async move { Ok(Some(txt)) })
    }

    fn get_active_buffer_content(
        &self,
        _session_id: crate::ports::SessionId,
    ) -> crate::ports::BoxFuture<'static, Result<Option<String>, crate::ports::UseCaseError>>
    {
        let txt = self.text.clone();
        Box::pin(async move { Ok(Some(txt)) })
    }

    fn get_active_editor_document(
        &self,
        _req: GetActiveEditorDocumentRequest,
    ) -> crate::ports::BoxFuture<
        'static,
        Result<crate::ports::GetActiveEditorDocumentResponse, crate::ports::UseCaseError>,
    > {
        // Build a simple EditorDocument using the active_buffer id and a single-line content.
        let buf = BufferId::from(self.current_active_buffer().as_str());
        let content = Some(self.text.clone());
        let ed = EditorDocument {
            buffer_id: buf,
            content,
            cursor: EditorCursor { line: 0, column: 0 },
            selection: None,
            line_count: 1,
            current_line: Some(self.text.clone()),
        };
        Box::pin(async move { Ok(crate::ports::GetActiveEditorDocumentResponse { document: ed }) })
    }

    fn get_visible_lines(
        &self,
        _req: GetVisibleLinesRequest,
    ) -> crate::ports::BoxFuture<
        'static,
        Result<crate::ports::GetVisibleLinesResponse, crate::ports::UseCaseError>,
    > {
        // Provide a one-line VisibleLinesWindow that mirrors the tiny document.
        let vl = view::VisibleLine {
            line_number: 1,
            text: self.text.clone(),
            is_cursor_line: true,
            cursor_column: Some(0),
            selection_intersects: false,
            selection_start_column: None,
            selection_end_column: None,
        };
        let vw = view::VisibleLinesWindow { top_line: 1, total_lines: 1, lines: vec![vl] };
        Box::pin(async move { Ok(crate::ports::GetVisibleLinesResponse { window: vw }) })
    }
}

// Internal runtime holder initialized on first access.
struct CompositionRuntime {
    comp: DesktopComposition,
    view: Arc<FakeView>,
    session: SessionId,
}

static RUNTIME: OnceCell<Mutex<CompositionRuntime>> = OnceCell::new();

fn ensure_runtime_initialized(_width: u32, _height: u32) -> std::sync::MutexGuard<'static, CompositionRuntime> {
    RUNTIME.get_or_init(|| {
        // Create an initial fake view and composition.
        let view = Arc::new(FakeView::new("buf:one", "hello from composition"));
        let comp = DesktopComposition::new();
        // Create a session id for the in-process binary.
        let session = SessionId(Id::new());

        let mut holder = CompositionRuntime { comp, view, session };
        {
            // Perform an initial refresh synchronously using a temporary runtime.
            let guard_view = holder.view.clone();
            let rt = Runtime::new().expect("tokio runtime init");
            rt.block_on(async {
                // Use the composition's async refresh to populate metadata.
                let _ = holder.comp.refresh(guard_view, holder.session.clone(), None).await;
            });
        }

        Mutex::new(holder)
    }).lock().unwrap()
}

/// Map DesktopMetadata -> ShellRegions using the presenter's map_regions helper.
/// This is intentionally a thin adapter so the GPU presenter receives the canonical ShellRegions.
fn metadata_to_regions(width: u32, height: u32, meta: Option<DesktopMetadata>) -> ShellRegions {
    // Conservative defaults
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    let mut regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);

    if let Some(m) = meta {
        // Marker & chrome_label: prefer active_buffer display if present.
        if let Some(ref ab) = m.active_buffer {
            regions.marker = Some(ab.to_string());
            regions.chrome_label = Some(ab.to_string());
            regions.status_text = Some(m.last_command_line.clone().unwrap_or_else(|| format!("status: {}", ab.to_string())));
        } else if let Some(ref cmd) = m.last_command_line {
            regions.status_text = Some(cmd.clone());
        }

        // content_preview: expose a single-line hint when active document summary or content_preview exists.
        if let Some(ref details) = m.active_buffer_details {
            regions.content_preview = details.display.clone();
            regions.content_preview_count = Some(1);
        }

        // active buffer semantic projection: use the explicit active buffer label when available.
        regions.active_buffer_label = m.active_buffer.as_ref().map(|b| b.to_string());

        // opened buffers: if present, synthesize a small ai_indicator / marker for visibility.
        if !m.opened_buffers.is_empty() {
            regions.ai_indicator = Some(format!("opened={}", m.opened_buffers.len()));
        }
    }

    regions
}

/// Return the current ShellRegions snapshot derived from the real DesktopComposition.
/// This function performs a synchronous refresh to ensure the composition metadata is up-to-date.
pub fn current_regions(width: u32, height: u32) -> ShellRegions {
    let mut runtime = ensure_runtime_initialized(width, height);
    // Refresh composition (async) to ensure latest metadata is present.
    // Avoid borrowing `runtime` across the async block by taking owned/cloned locals.
    let view = runtime.view.clone();
    let session = runtime.session.clone();
    // Obtain a mutable reference to the composition to call refresh without also
    // borrowing `runtime` immutably inside the async block.
    let comp_ref = &mut runtime.comp;
    let rt = Runtime::new().expect("tokio runtime init");
    rt.block_on(async {
        let _ = comp_ref.refresh(view, session, None).await;
    });

    let meta = runtime.comp.latest_metadata();
    metadata_to_regions(width, height, meta)
}

/// Apply an action (e.g. SetActiveBuffer) by mutating the in-process FakeView and
/// refreshing the real DesktopComposition. Returns the new ShellRegions snapshot.
///
/// This keeps action handling outside the GUI binary's paint code while exercising
/// the canonical composition refresh path.
pub fn apply_action_and_get_regions(action: Action, width: u32, height: u32) -> ShellRegions {
    let mut runtime = ensure_runtime_initialized(width, height);
    // If action targets a specific active buffer, update the fake view so the composition
    // refresh will pick up the change from the view boundary (keeps logic out of the binary).
    match action {
        Action::SetActiveBuffer(name) => {
            runtime.view.set_active_buffer(&name);
        }
        _ => {}
    }

    // Run an explicit refresh to update metadata, avoiding borrowing `runtime` across await.
    let view = runtime.view.clone();
    let session = runtime.session.clone();
    let comp_ref = &mut runtime.comp;
    let rt = Runtime::new().expect("tokio runtime init");
    rt.block_on(async {
        let _ = comp_ref.refresh(view, session, None).await;
    });

    // Build ShellRegions from the updated metadata.
    let meta = runtime.comp.latest_metadata();
    metadata_to_regions(width, height, meta)
}
