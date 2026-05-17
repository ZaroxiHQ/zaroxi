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

/// Single opened-buffer projection item exposed to the shell.
///
/// Purpose:
/// - Tiny, read-only item that summarizes an opened buffer for the outer UI.
/// - Keeps presentation concerns minimal: buffer id, optional display label, and active flag.
#[derive(Clone, Debug)]
pub struct OpenedBufferItem {
    /// Canonical buffer id (core BufferId).
    pub buffer_id: crate::ports::BufferId,
    /// Optional display label (e.g. path or file name) suitable for shell printing.
    pub display: Option<String>,
    /// Whether this buffer is currently the active buffer in the session.
    pub active: bool,
}

/// Small read-only projection describing the currently active buffer for the shell.
///
/// Purpose:
/// - Tiny, shell-facing read model that gives the outer harness enough information
///   to print and reason about the active buffer without pulling application logic
///   into the interface layer.
/// - Kept intentionally small: id, optional display label (path), and a simple
///   line-count metric when available from the presenter's latest window.
#[derive(Clone, Debug)]
pub struct ActiveBufferDetails {
    /// Canonical buffer id.
    pub buffer_id: crate::ports::BufferId,
    /// Optional display label derived from BufferId.path() or opened-buffer projection.
    pub display: Option<String>,
    /// Number of lines in the buffer snapshot when available (0 if unknown).
    pub line_count: usize,
}

/// Tiny AI projection: a small, shell-facing read-only snapshot of the most recent AI outcome.
///
/// Keep this intentionally minimal:
/// - kind: a short label when available (e.g. "ExplainExecuted")
/// - result: the textual result produced by the AI (if any)
/// - target_buffer: the BufferId that was the target of the AI operation (if available)
#[derive(Clone, Debug)]
pub struct AiProjection {
    pub kind: Option<String>,
    pub result: Option<String>,
    pub target_buffer: Option<crate::ports::BufferId>,
}

/// Small enum describing why the DesktopComposition was refreshed.
///
/// This is a tiny, shell-facing model intended only to help outer layers (harness,
/// tests, UI glue) reason about refreshes in an explicit but minimal way. It is
/// deliberately not an event system — just a lightweight, descriptive reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefreshReason {
    InitialLoad,
    RefreshAction,
    CursorMoved,
    BufferUpdated,
    ActiveBufferChanged,
    AiProjectionUpdated,
}

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
    /// New: small read-only list of opened buffers projected for the shell.
    pub opened_buffers: Vec<OpenedBufferItem>,
    /// New: small, focused projection for the currently active buffer (when present).
    pub active_buffer_details: Option<ActiveBufferDetails>,
    /// New: small AI projection exposing the last AI result relevant to this session (if any).
    pub ai_projection: Option<AiProjection>,
    /// New: the reason the composition was refreshed most recently (shell-facing).
    pub refresh_reason: Option<RefreshReason>,
}

/// Tiny read-only status snapshot indicating which composition projections are currently populated.
///
/// Purpose:
/// - Very small, shell-facing struct summarizing presence/availability of
///   key projections without exposing their full contents.
/// - Values are booleans to remain compact and deterministic for the harness.
#[derive(Clone, Debug)]
pub struct DesktopStatus {
    /// Is there a presenter render window available?
    pub has_render_window: bool,
    /// Is the desktop metadata projection present?
    pub has_metadata: bool,
    /// Is the active-buffer details projection present?
    pub has_active_buffer_details: bool,
    /// Is the opened-buffers projection present and non-empty?
    pub has_opened_buffers: bool,
    /// Is there an AI projection available?
    pub has_ai_projection: bool,
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
    /// Small cached status snapshot summarizing which projections are populated.
    status: Option<DesktopStatus>,
    /// Monotonically increasing composition revision counter (shell-facing).
    revision: u64,
    /// Optional pending refresh reason set by callers which will be consumed by `refresh_with_service`.
    pending_refresh_reason: Option<RefreshReason>,
}

impl DesktopComposition {
    /// Create a new empty composition.
    pub fn new() -> Self {
        Self {
            presenter: Presenter::new(),
            session_id: None,
            workspace_id: None,
            metadata: None,
            status: None,
            revision: 0,
            pending_refresh_reason: None,
        }
    }

    /// Refresh composition by asking the Presenter to refresh its snapshot.
    ///
    /// - `view`: application-provided read-only WorkspaceView (Arc'd).
    /// - `session_id`: typed session id to query active editor/presenter.
    /// - `workspace_id`: optional workspace id (caller-supplied) to be recorded in composition.
    ///
    /// This original lightweight refresh remains available and delegates to the
    /// more featureful `refresh_with_service` with `None` for the optional service.
    pub async fn refresh(
        &mut self,
        view: Arc<dyn WorkspaceView>,
        session_id: SessionId,
        workspace_id: Option<Id>,
    ) -> Result<(), String> {
        self.refresh_with_service(view, session_id, workspace_id, None).await
    }

    /// Refresh the composition and optionally use a WorkspaceService to obtain
    /// an opened-buffer list. When `service` is `None` the method falls back to
    /// the conservative opened-buffer count projection (1 if active buffer exists).
    ///
    /// This method keeps responsibilities minimal: it reuses existing read APIs
    /// and does not add new application ports. The optional service parameter is
    /// intended to be provided by callers that already hold a concrete
    /// WorkspaceService (composition/harness), enabling the richer opened buffer
    /// projection without changing the core application or domain layers.
    pub async fn refresh_with_service(
        &mut self,
        view: Arc<dyn WorkspaceView>,
        session_id: SessionId,
        workspace_id: Option<Id>,
        service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    ) -> Result<(), String> {
        // Capture previous presenter snapshot to detect content changes.
        let prev_presenter_snapshot = self.presenter.latest();

        // 1) Refresh presenter snapshot (reuses adapter seam and existing projection).
        self.presenter.refresh(view.clone(), session_id.clone()).await?;

        // Capture the new presenter snapshot so we can detect buffer content changes
        // (shell-facing, presentation-only signal).
        let new_presenter_snapshot = self.presenter.latest();

        // 2) Attempt to read the active editor document via the WorkspaceView seam.
        let active_buf_opt = match view.get_active_editor_document(crate::ports::GetActiveEditorDocumentRequest { session_id: session_id.clone() }).await {
            Ok(resp) => Some(resp.document.buffer_id.clone()),
            Err(_) => None,
        };

        // Prepare default conservative projection values.
        let mut opened_count = if active_buf_opt.is_some() { 1 } else { 0 };
        let mut opened_list: Vec<OpenedBufferItem> = Vec::new();

        // 3) If a WorkspaceService is provided, attempt to obtain the authoritative opened buffer list.
        if let Some(svc) = &service {
            // Request list of opened buffers for the session (application-owned use-case).
            match svc.list_open_buffers(crate::ports::ListBuffersRequest { session_id: session_id.clone() }).await {
                Ok(list_res) => {
                    opened_count = list_res.buffer_ids.len();
                    // Build small projection items. Use path/display when available.
                    for bid in list_res.buffer_ids.iter() {
                        let display = bid.path().map(|p| p.to_string_lossy().to_string());
                        let is_active = list_res.active_buffer.as_ref().map(|ab| ab == bid).unwrap_or(false);
                        opened_list.push(OpenedBufferItem { buffer_id: bid.clone(), display, active: is_active });
                    }
                }
                Err(_) => {
                    // On error, fall back to conservative single-item projection when active exists.
                    if let Some(bid) = active_buf_opt.clone() {
                        let display = bid.path().map(|p| p.to_string_lossy().to_string());
                        opened_list.push(OpenedBufferItem { buffer_id: bid.clone(), display, active: true });
                    }
                }
            }
        } else {
            // No service provided: keep conservative projection (only active buffer when present).
            if let Some(bid) = active_buf_opt.clone() {
                let display = bid.path().map(|p| p.to_string_lossy().to_string());
                opened_list.push(OpenedBufferItem { buffer_id: bid.clone(), display, active: true });
            }
        }

        // 4) Update composition metadata and simple recorded ids.
        // Compute a tiny active-buffer details projection by reusing the presenter's
        // latest renderable window (this avoids duplicating application logic).
        let active_buffer_details: Option<ActiveBufferDetails> = if let Some(bid) = active_buf_opt.clone() {
            // Prefer the display label from the opened_buffers projection if available.
            let display_label = opened_list.iter().find(|i| i.buffer_id == bid).and_then(|i| i.display.clone())
                .or_else(|| bid.path().map(|p| p.to_string_lossy().to_string()));

            // Use presenter's latest window (if present) to obtain a line_count metric.
            let line_count = self.presenter.latest().map(|w| w.total_lines).unwrap_or(0usize);

            Some(ActiveBufferDetails {
                buffer_id: bid.clone(),
                display: display_label,
                line_count,
            })
        } else {
            None
        };

        // Attempt to read recent events to build a tiny AI projection when a WorkspaceService is available.
        // We intentionally use the existing `get_recent_events` port (read-only) and only surface
        // the most recent ExplainExecuted event if present. This keeps composition purely read-only
        // and avoids duplicating AI orchestration logic.
        let mut ai_proj: Option<AiProjection> = None;
        if let Some(svc) = &service {
            if let Ok(ev_res) = svc.get_recent_events(crate::ports::GetRecentEventsRequest { session_id: session_id.clone(), limit: 20 }).await {
                // Iterate from newest to oldest and pick the first ExplainExecuted we find.
                for ev in ev_res.events.iter().rev() {
                    if let crate::ports::WorkspaceEventKind::ExplainExecuted { buffer_id, result } = &ev.kind {
                        ai_proj = Some(AiProjection {
                            kind: Some("ExplainExecuted".to_string()),
                            result: Some(result.clone()),
                            target_buffer: Some(buffer_id.clone()),
                        });
                        break;
                    }
                }
            }
        }

        // --- Refresh reason detection ---
        //
        // Compute a small set of lightweight change-detections that the shell cares about.
        // Preference order:
        // 1) Explicit pending reason set by caller (actions).
        // 2) AI projection changed (new explain executed result became available).
        // 3) Active buffer changed (shell cares which buffer is active).
        //    * When a WorkspaceService was provided prefer comparing the opened-buffer
        //      projection's active marker (service authoritative for opened buffers).
        //    * Otherwise fall back to comparing the presenter's active buffer (view).
        // 4) Buffer content changed as observed by the presenter snapshot (BufferUpdated).
        // 5) InitialLoad when composition had no prior session_id.
        // 6) Generic RefreshAction otherwise.
        //
        // Note: comparisons are tiny and presentation-only (strings / buffer ids); we avoid
        // introducing an event stream or mirroring application internals.
        let prev_active = self.metadata.as_ref().and_then(|m| m.active_buffer.clone());
        let prev_opened_active = self.metadata.as_ref().and_then(|m| m.opened_buffers.iter().find(|i| i.active).map(|i| i.buffer_id.clone()));
        let prev_ai_result = self.metadata.as_ref().and_then(|m| m.ai_projection.as_ref().and_then(|a| a.result.clone()));

        // signature helper for presenter snapshots (concatenate span texts)
        let make_presenter_sig = |opt: Option<InterfaceRenderableWindow>| -> String {
            if let Some(w) = opt {
                let mut out = String::new();
                for line in w.lines.iter() {
                    for sp in line.spans.iter() {
                        out.push_str(&sp.text);
                        out.push('|');
                    }
                    out.push('\n');
                }
                out
            } else {
                String::new()
            }
        };

        let prev_sig = make_presenter_sig(prev_presenter_snapshot.clone());
        let new_sig = make_presenter_sig(new_presenter_snapshot.clone());
        let new_ai_result = ai_proj.as_ref().and_then(|a| a.result.clone());

        // If the composition consulted a WorkspaceService, prefer the service-provided
        // opened-buffer active marker as the source of truth for "ActiveBufferChanged".
        let current_opened_active = opened_list.iter().find(|i| i.active).map(|i| i.buffer_id.clone());

        let reason = if let Some(pending) = self.pending_refresh_reason.take() {
            pending
        } else if prev_ai_result != new_ai_result {
            // Prefer AI projection updates when a new ExplainExecuted result is present.
            RefreshReason::AiProjectionUpdated
        } else if current_opened_active.is_some() || prev_opened_active.is_some() {
            // When we have an opened-buffer projection (service used previously or now),
            // compare the previous opened-active against the current opened-active.
            if prev_opened_active != current_opened_active {
                RefreshReason::ActiveBufferChanged
            } else if prev_active != active_buf_opt {
                // Fallback: also consider presenter-level active buffer changes if they differ.
                RefreshReason::ActiveBufferChanged
            } else if prev_sig != new_sig {
                RefreshReason::BufferUpdated
            } else {
                if self.session_id.is_none() { RefreshReason::InitialLoad } else { RefreshReason::RefreshAction }
            }
        } else if prev_active != active_buf_opt {
            RefreshReason::ActiveBufferChanged
        } else if prev_sig != new_sig {
            RefreshReason::BufferUpdated
        } else {
            if self.session_id.is_none() { RefreshReason::InitialLoad } else { RefreshReason::RefreshAction }
        };

        self.session_id = Some(session_id.clone());
        self.workspace_id = workspace_id;

        // Compute metadata and status snapshots derived from the refresh work above.
        let metadata = DesktopMetadata {
            session_id: Some(session_id),
            workspace_id: self.workspace_id.clone(),
            active_buffer: active_buf_opt.clone(),
            opened_buffer_count: opened_count,
            opened_buffers: opened_list.clone(),
            active_buffer_details: active_buffer_details.clone(),
            ai_projection: ai_proj.clone(),
            refresh_reason: Some(reason),
        };

        // Status summarizes availability of key projections: presenter window, metadata, active details, opened list, AI projection.
        let status = DesktopStatus {
            has_render_window: self.presenter.latest().is_some(),
            has_metadata: true,
            has_active_buffer_details: active_buffer_details.is_some(),
            has_opened_buffers: !metadata.opened_buffers.is_empty(),
            has_ai_projection: ai_proj.is_some(),
        };

        self.metadata = Some(metadata);
        self.status = Some(status);

        // Increment the small, shell-facing revision counter on each successful refresh.
        self.revision = self.revision.saturating_add(1);

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

    /// Return the tiny active-buffer details projection (if present).
    ///
    /// This accessor returns a small, shell-oriented view over the active buffer.
    /// It is purely read-only and derived during refresh; callers may use it to
    /// display a concise summary without touching application logic.
    pub fn latest_active_buffer_details(&self) -> Option<ActiveBufferDetails> {
        self.metadata.as_ref().and_then(|m| m.active_buffer_details.clone())
    }

    /// Tiny read-only status snapshot indicating which composition projections are populated.
    pub fn latest_status(&self) -> Option<DesktopStatus> {
        self.status.clone()
    }

    /// Return the small, read-only AI projection (if any) obtained during the last refresh.
    pub fn latest_ai_projection(&self) -> Option<AiProjection> {
        self.metadata.as_ref().and_then(|m| m.ai_projection.clone())
    }

    /// Return the most recent composition revision (monotonic counter).
    pub fn latest_revision(&self) -> u64 {
        self.revision
    }

    /// Set a pending refresh reason which will be consumed by the next `refresh_with_service`.
    /// This allows callers (actions) to communicate a tiny, explicit reason for the refresh.
    pub fn set_pending_refresh_reason(&mut self, reason: RefreshReason) {
        self.pending_refresh_reason = Some(reason);
    }

    /// Query whether a pending refresh reason has been set.
    pub fn has_pending_refresh_reason(&self) -> bool {
        self.pending_refresh_reason.is_some()
    }

    /// Return the most recent refresh reason recorded in the composition metadata.
    pub fn latest_refresh_reason(&self) -> Option<RefreshReason> {
        self.metadata.as_ref().and_then(|m| m.refresh_reason.clone())
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
        comp.refresh(arc.clone(), sid.clone(), Some(wid.clone())).await.expect("refresh ok");

        assert_eq!(comp.get_session_id().unwrap(), sid);
        assert_eq!(comp.get_workspace_id().unwrap(), wid);
        let win = comp.latest_window().expect("window present");
        assert_eq!(win.total_lines, 1);
        assert_eq!(win.lines.len(), 1);

        // Revision should have advanced from initial 0 to 1 after the first refresh.
        assert_eq!(comp.latest_revision(), 1);

        // A subsequent refresh should advance the revision again.
        comp.refresh(arc.clone(), sid.clone(), Some(wid.clone())).await.expect("second refresh ok");
        assert_eq!(comp.latest_revision(), 2);

        // Verify tiny metadata projection populated from the application read-path.
        let meta = comp.latest_metadata().expect("metadata present");
        assert_eq!(meta.session_id.unwrap(), sid);
        assert_eq!(meta.workspace_id.unwrap(), wid);
        assert_eq!(meta.active_buffer.unwrap(), crate::ports::BufferId::from("buf:fake"));
        assert_eq!(meta.opened_buffer_count, 1);

        // New: verify active-buffer details projection is populated and consistent
        let abd = comp.latest_active_buffer_details().expect("active buffer details present");
        assert_eq!(abd.buffer_id, crate::ports::BufferId::from("buf:fake"));
        assert_eq!(abd.line_count, 1);
        assert_eq!(abd.display.unwrap(), "fake".to_string());

        // Status snapshot must be present and reflect available projections.
        let status = comp.latest_status().expect("status present");
        assert!(status.has_render_window, "presenter window should be available after refresh");
        assert!(status.has_metadata, "metadata should be present after refresh");
        assert!(status.has_active_buffer_details, "active buffer details should be present");
        assert!(status.has_opened_buffers, "opened buffers projection should be non-empty");
        assert!(!status.has_ai_projection, "AI projection should not be present in this path");
    }

    #[tokio::test]
    async fn desktop_composition_ai_projection_refreshes() {
        use std::sync::Arc;
        use uuid::Uuid;
        use chrono::Utc;

        // Build a fake view (re-use test helper above)
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let wid = zaroxi_kernel_types::Id::new();

        // Minimal fake service that returns a single opened buffer and a single ExplainExecuted event.
        struct FakeSvc {
            buf: crate::ports::BufferId,
            wid: zaroxi_kernel_types::Id,
        }

        impl FakeSvc {
            fn new(buf: crate::ports::BufferId, wid: zaroxi_kernel_types::Id) -> Self {
                Self { buf, wid }
            }
        }

        impl crate::ports::WorkspaceService for FakeSvc {
            fn boot_workspace(&self, _req: crate::ports::WorkspaceBootRequest) -> crate::BoxFuture<'static, Result<crate::ports::WorkspaceBootResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownWorkspace) })
            }
            fn open_buffer(&self, _req: crate::ports::OpenBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::OpenBufferResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn list_open_buffers(&self, _req: crate::ports::ListBuffersRequest) -> crate::BoxFuture<'static, Result<crate::ports::ListBuffersResponse, crate::ports::UseCaseError>> {
                let b = self.buf.clone();
                Box::pin(async move { Ok(crate::ports::ListBuffersResponse { buffer_ids: vec![b], active_buffer: Some(crate::ports::BufferId::from("buf:fake")) }) })
            }
            fn set_active_buffer(&self, _req: crate::ports::SetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetActiveBufferResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn get_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetActiveBufferResponse, crate::ports::UseCaseError>> {
                let bid = self.buf.clone();
                Box::pin(async move { Ok(crate::ports::GetActiveBufferResponse { buffer_id: bid }) })
            }
            fn set_editor_cursor(&self, _req: crate::ports::SetEditorCursorRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetEditorCursorResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn set_editor_selection(&self, _req: crate::ports::SetSelectionRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetSelectionResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn clear_editor_selection(&self, _req: crate::ports::ClearSelectionRequest) -> crate::BoxFuture<'static, Result<crate::ports::ClearSelectionResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn get_editor_state(&self, _req: crate::ports::GetEditorStateRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetEditorStateResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn set_viewport_state(&self, _req: crate::ports::SetViewportRequest) -> crate::BoxFuture<'static, Result<crate::ports::SetViewportResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn scroll_viewport(&self, _req: crate::ports::ScrollViewportRequest) -> crate::BoxFuture<'static, Result<crate::ports::ScrollViewportResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn explain_active_buffer(&self, _req: crate::ports::GetActiveBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::NoActiveBuffer) })
            }
            fn dispatch_command(&self, _req: crate::ports::DispatchCommandRequest) -> crate::BoxFuture<'static, Result<crate::ports::DispatchCommandResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn update_buffer(&self, _req: crate::ports::UpdateBufferRequest) -> crate::BoxFuture<'static, Result<crate::ports::UpdateBufferResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn apply_text_transaction(&self, _req: crate::ports::ApplyTextTransactionRequest) -> crate::BoxFuture<'static, Result<crate::ports::ApplyTextTransactionResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Ok(crate::ports::ApplyTextTransactionResponse { ok: true, state: crate::ports::EditorState { cursor: crate::ports::EditorCursor::zero(), selection: None }, content: None }) })
            }
            fn get_recent_commands(&self, _req: crate::ports::GetRecentCommandsRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetRecentCommandsResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Ok(crate::ports::GetRecentCommandsResponse { commands: Vec::new() }) })
            }

            fn get_recent_events(&self, req: crate::ports::GetRecentEventsRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetRecentEventsResponse, crate::ports::UseCaseError>> {
                let buf = self.buf.clone();
                let wid = self.wid.clone();
                Box::pin(async move {
                    let ev = crate::ports::WorkspaceEvent {
                        id: Uuid::new_v4(),
                        timestamp: Utc::now(),
                        session_id: req.session_id.clone(),
                        workspace_id: wid,
                        kind: crate::ports::WorkspaceEventKind::ExplainExecuted { buffer_id: buf.clone(), result: "mocked explain".to_string() },
                    };
                    Ok(crate::ports::GetRecentEventsResponse { events: vec![ev] })
                })
            }

            fn get_session_snapshot(&self, _req: crate::ports::GetSessionSnapshotRequest) -> crate::BoxFuture<'static, Result<crate::ports::GetSessionSnapshotResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }

            fn create_checkpoint(&self, _req: crate::ports::CreateCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::CreateCheckpointResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }

            fn save_checkpoint(&self, _req: crate::ports::SaveCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::SaveCheckpointResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn load_checkpoint(&self, _req: crate::ports::LoadCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::LoadCheckpointResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
            fn restore_checkpoint(&self, _req: crate::ports::RestoreCheckpointRequest) -> crate::BoxFuture<'static, Result<crate::ports::RestoreCheckpointResponse, crate::ports::UseCaseError>> {
                Box::pin(async { Err(crate::ports::UseCaseError::UnknownSession) })
            }
        }

        let fake_service = std::sync::Arc::new(FakeSvc::new(crate::ports::BufferId::from("buf:fake"), wid.clone())) as std::sync::Arc<dyn crate::ports::WorkspaceService>;

        let mut comp = DesktopComposition::new();
        // Use refresh_with_service so the composition will consult the fake service and recent events.
        comp.refresh_with_service(arc, sid.clone(), Some(wid.clone()), Some(fake_service)).await.expect("refresh ok");

        // Revision should have advanced from initial 0 to 1 after the refresh with service.
        assert_eq!(comp.latest_revision(), 1);

        let meta = comp.latest_metadata().expect("metadata present");
        assert!(meta.ai_projection.is_some(), "ai projection should be present from recent events");
        let ai = meta.ai_projection.unwrap();
        assert_eq!(ai.result.unwrap(), "mocked explain".to_string());
        assert_eq!(ai.target_buffer.unwrap(), crate::ports::BufferId::from("buf:fake"));

        // Ensure the composition recorded that the AI projection was updated.
        let rr = comp.latest_refresh_reason().expect("reason present");
        assert_eq!(rr, RefreshReason::AiProjectionUpdated);

        // Status snapshot must be present and reflect AI projection availability.
        let status = comp.latest_status().expect("status present");
        assert!(status.has_render_window, "presenter window should be available after refresh");
        assert!(status.has_metadata, "metadata should be present after refresh");
        // active buffer details available in this test too
        assert!(status.has_active_buffer_details, "active buffer details should be present");
        assert!(status.has_opened_buffers, "opened buffers projection should be non-empty");
        assert!(status.has_ai_projection, "AI projection should be reported present");
    }
}
