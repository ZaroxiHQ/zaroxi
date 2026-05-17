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

/// Helper: convert CommandKind to short name string for tiny shell-facing LastCommandLine.
fn command_kind_short_name(kind: &crate::ports::CommandKind) -> &'static str {
    match kind {
        crate::ports::CommandKind::BootWorkspace { .. } => "BootWorkspace",
        crate::ports::CommandKind::OpenBuffer { .. } => "OpenBuffer",
        crate::ports::CommandKind::UpdateBuffer { .. } => "UpdateBuffer",
        crate::ports::CommandKind::SetActiveBuffer { .. } => "SetActiveBuffer",
        crate::ports::CommandKind::ExplainActiveBuffer => "ExplainActiveBuffer",
        crate::ports::CommandKind::DispatchAppCommand { .. } => "DispatchAppCommand",
        crate::ports::CommandKind::ExplainActiveBuffer => "ExplainActiveBuffer",
        _ => "Command",
    }
}
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

/// Tiny read-only active document summary exposed to shells.
///
/// Purpose: a minimal, derived, shell-facing projection answering:
/// - active buffer display/name,
/// - line count,
/// - current cursor position (1-based line, 0-based column),
/// - whether a selection exists,
/// - optionally a current-line snippet when available.
///
/// This is intentionally small and purely read-only: derived from DesktopComposition
/// projections (metadata + presenter's latest window). No mutation, no new ports.
#[derive(Clone, Debug)]
pub struct ActiveDocumentSummary {
    /// Canonical active buffer id when available.
    pub buffer_id: Option<crate::ports::BufferId>,
    /// Optional human-friendly display label for the active buffer.
    pub display: Option<String>,
    /// Number of lines in the buffer snapshot (0 if unknown).
    pub line_count: usize,
    /// Cursor line number in 1-based coordinates (None when absent).
    pub cursor_line: Option<usize>,
    /// Cursor column (0-based character index) within the line (None when absent).
    pub cursor_column: Option<usize>,
    /// Whether any selection exists in the visible/projected window.
    pub selection_present: bool,
    /// Optional current line snippet (truncated, Unicode-safe).
    pub current_line_snippet: Option<String>,
}

/// Small, read-only viewport anchoring hint. This is intentionally a tiny, best-effort
/// indicator derived from the presenter's renderable window and the observed cursor line.
/// It is heuristic-only and purely advisory for shells; Unknown is returned when no
/// reliable inference can be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewportAnchoring {
    Top,
    Centered,
    Unknown,
}

/// Tiny readonly viewport summary for the active editor surface.
///
/// Purpose:
/// - Expose a compact, deterministic summary of the presenter's window useful to
///   shells/harnesses (top visible line, visible count, total projected lines).
/// - Indicate whether the caret/cursor is visible in the current window.
/// - Provide a best-effort anchoring hint (Top / Centered / Unknown) derived from
///   the window and cursor location when available.
///
/// Constraints:
/// - Purely read-only and derived from the presenter's InterfaceRenderableWindow.
/// - No mutation, no scrolling or viewport control, and no rendering semantics.
#[derive(Clone, Debug)]
pub struct ViewportSummary {
    /// 1-based top visible line number.
    pub top_visible_line: usize,
    /// Number of visible lines currently in the window.
    pub visible_line_count: usize,
    /// Total projected/rendered line count when available (0 if unknown).
    pub total_lines: usize,
    /// Whether the cursor (caret) is visible somewhere in the current window.
    pub cursor_visible: bool,
    /// Best-effort anchoring hint for the window (top-anchored, centered, or unknown).
    pub anchoring: ViewportAnchoring,
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

/// Tiny, enumerated AI projection kind used by shell-facing summaries.
///
/// This enum intentionally covers a very small set of well-known kinds and
/// falls back to Other(String) for unrecognized labels. It is strictly a
/// presentation aid for shells/harnesses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiKind {
    Explain,
    Suggest,
    Refactor,
    Other(String),
}

/// Small, coarse AI projection state exposed to shells.
///
/// We do not implement a runtime state machine here; this is a tiny hint
/// derived from whether an AI result text is present. It keeps the surface
/// minimal but useful for shell diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiState {
    Ready,
    Running,
    Failed,
}

/// Very small, read-only summary of the AI projection intended for shells.
///
/// - `present`: whether an AI projection exists at all.
/// - `kind`: interpreted kind when available (small enum).
/// - `target_buffer`: buffer the projection refers to when available.
/// - `state`: a tiny readiness/running/failed hint derived from projection shape.
#[derive(Clone, Debug)]
pub struct AiProjectionSummary {
    pub present: bool,
    pub kind: Option<AiKind>,
    pub target_buffer: Option<crate::ports::BufferId>,
    pub state: AiState,
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
    /// Tiny, read-only textual last command line (shell-facing): short command name + success marker.
    pub last_command_line: Option<String>,
    /// New: the reason the composition was refreshed most recently (shell-facing).
    pub refresh_reason: Option<RefreshReason>,
}

/// Tiny read-only status snapshot indicating which composition projections are currently populated.
///
/// Purpose:
/// - Very small, shell-facing struct summarizing presence/availability of
///   key projections without exposing their full contents.
/// - Values are booleans to remain compact and deterministic for the harness.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Tiny read-only summary item for a single opened buffer exposed to shells.
///
/// Purpose:
/// - Small, shell-facing immutable DTO used by OpenedBuffersSummary.
/// - Reuses BufferId canonical type from core via `crate::ports`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedBufferItemSummary {
    /// Canonical buffer id (core BufferId).
    pub buffer_id: crate::ports::BufferId,
    /// Optional display label (e.g. path or file name).
    pub display: Option<String>,
    /// Optional line-count when available (0 when unknown).
    pub line_count: usize,
    /// Whether this buffer is currently active.
    pub active: bool,
}

/// Tiny read-only projection summarizing opened buffers for shell consumption.
///
/// Purpose:
/// - Provide a compact, deterministic view of opened buffers:
///   - total count
///   - per-buffer id/display/line-count/active flag
///   - optional currently active buffer id for quick shell checks
/// - Constructed from existing composition metadata; purely read-only and local to the interface crate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedBuffersSummary {
    /// Number of opened buffers (conservative projection from metadata).
    pub count: usize,
    /// Small per-buffer items.
    pub items: Vec<OpenedBufferItemSummary>,
    /// Optional active buffer id when available.
    pub active: Option<crate::ports::BufferId>,
}
 
/// Small, shell-facing summary of the composition.
///
/// Purpose:
/// - Provide a compact, read-only projection that combines a few frequently used
///   composition facts for outer shells / harnesses.
/// - This is intentionally derivative (reads existing composition fields) and
///   tiny: revision, refresh_reason, optional status snapshot and active buffer id.
///
/// The summary is not a replacement for the richer metadata/status APIs; it is
/// a convenience accessor for small shells that only need a compact snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopSummary {
    /// Monotonic composition revision.
    pub revision: u64,
    /// The most recent refresh reason recorded in metadata (when available).
    pub refresh_reason: Option<RefreshReason>,
    /// A compact status snapshot (may be None if not populated).
    pub status: Option<DesktopStatus>,
    /// Optional active buffer id (when available).
    pub active_buffer: Option<crate::ports::BufferId>,
}

/// Tiny, shell-facing current context accessor used by simple UI shells and the harness.
///
/// Purpose:
/// - Very small, read-only, derived view aggregating the most immediately useful
///   facts for a shell: active buffer id, a display label when available, last revision,
///   latest refresh reason, and a quick flag indicating whether an AI projection exists.
/// - Kept intentionally minimal to remain shell-facing and presentation-only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellContext {
    /// Canonical active buffer id when available.
    pub active_buffer: Option<crate::ports::BufferId>,
    /// Optional human-friendly display label for the active buffer (when available).
    pub active_display: Option<String>,
    /// Latest composition revision (monotonic).
    pub latest_revision: u64,
    /// Latest recorded refresh reason (when available).
    pub latest_refresh_reason: Option<RefreshReason>,
    /// Whether the composition currently contains an AI projection.
    pub has_ai_projection: bool,
    /// Tiny, shell-facing last-command-line string when available (short command name + success marker).
    pub last_command_line: Option<String>,
}

/// Tiny, one-line shell-facing status bar line.
///
/// Purpose:
/// - Provide a minimal, read-only single-line status suitable for tiny shells/harnesses.
/// - Prefer composing existing projections: AI projection result (preferred), then the
///   last refresh reason. Optionally expose a small "sticky" hint (e.g. active buffer display).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBarLine {
    /// Primary single-line status text (human readable).
    pub text: String,
    /// Optional small sticky hint (e.g. active buffer display) shown alongside the text.
    pub sticky: Option<String>,
}

/// Tiny, read-only aggregate snapshot aimed at shells and harnesses.
///
/// Purpose:
/// - Provide a single convenient read-only projection that composes the existing
///   shell-facing summaries already present on DesktopComposition:
///     - ShellContext
///     - ActiveDocumentSummary
///     - ViewportSummary
///     - AiProjectionSummary
///     - OpenedBuffersSummary
/// - The ShellSnapshot is purely an adapter-local convenience. It does not
///   duplicate logic; it simply calls the existing latest_* accessors and
///   packages their results. It is small and shallow.
#[derive(Clone, Debug)]
pub struct ShellSnapshot {
    /// Small shell context (required for the snapshot; snapshot absent when no context).
    pub context: ShellContext,

    /// Active document summary (when available).
    pub active_document: Option<ActiveDocumentSummary>,

    /// Viewport summary (when available).
    pub viewport: Option<ViewportSummary>,

    /// AI projection summary (when available).
    pub ai_summary: Option<AiProjectionSummary>,

    /// Opened buffers summary (always present as a small projection).
    pub opened_buffers: OpenedBuffersSummary,
}

/// Small, shell-facing consistency report for a DesktopComposition.
///
/// Purpose:
/// - Provide a tiny read-only report derived from existing composition fields.
/// - Allow harnesses and simple shells to assert basic invariants without
///   introducing a validation/telemetry subsystem.
/// - Keep semantics conservative: when data is absent we consider the check
///   satisfied unless an inconsistency can be observed.
///
/// Checks included:
/// - status_present_matches_summary: whether summary.status.is_some() aligns with `composition.status`.
/// - active_buffer_matches_details: when metadata exposes an active_buffer, does the active-buffer-details projection match it?
/// - active_buffer_in_opened_buffers: when opened_buffers is non-empty and active_buffer present, is the active_buffer one of the opened_buffers?
/// - presenter_window_matches_status: whether the presenter's window presence aligns with the status.has_render_window flag.
#[derive(Clone, Debug)]
pub struct DesktopConsistencyReport {
    /// Whether the status presence recorded in latest_summary() matches `composition.status` presence.
    pub status_present_matches_summary: bool,
    /// When metadata exposes an active buffer, whether that aligns with active-buffer-details buffer id.
    pub active_buffer_matches_details: bool,
    /// When opened_buffers is non-empty and an active buffer exists, whether the active buffer is among opened_buffers.
    pub active_buffer_in_opened_buffers: bool,
    /// Whether the presenter's window presence equals status.has_render_window (when status present).
    pub presenter_window_matches_status: bool,
    /// Overall ok (all checks true).
    pub overall_ok: bool,
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

                    // If the service reports an active_buffer that is not present in the
                    // returned buffer_ids, include it in the projection and mark it active.
                    // This covers lightweight service implementations that may set active
                    // without also adding the buffer to their opened list (test doubles).
                    if let Some(active_bid) = list_res.active_buffer.clone() {
                        if !list_res.buffer_ids.iter().any(|b| b == &active_bid) {
                            let display = active_bid.path().map(|p| p.to_string_lossy().to_string());
                            opened_list.push(OpenedBufferItem { buffer_id: active_bid.clone(), display, active: true });
                            opened_count = opened_count.saturating_add(1);
                        }
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
        // Compute authoritative active buffer: prefer service-provided opened-buffer active marker when present.
        // `opened_list` is already built above and is authoritative when `service` was provided.
        let current_opened_active = opened_list.iter().find(|i| i.active).map(|i| i.buffer_id.clone());

        // Determine authoritative active buffer for metadata and details: service (opened list) wins, else presenter-derived active.
        let authoritative_active = current_opened_active.clone().or(active_buf_opt.clone());

        // Compute a tiny active-buffer details projection using the authoritative active buffer.
        let active_buffer_details: Option<ActiveBufferDetails> = if let Some(bid) = authoritative_active.clone() {
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
        // Tiny shell-facing last-command-line string (computed below when service present).
        let mut last_command_line: Option<String> = None;

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

            // Attempt to obtain the most recent command (limit=1) and render a tiny one-line string.
            if let Ok(cmd_res) = svc.get_recent_commands(crate::ports::GetRecentCommandsRequest { session_id: session_id.clone(), limit: 1 }).await {
                if let Some(rec) = cmd_res.commands.last() {
                    let kind_name = command_kind_short_name(&rec.kind);
                    let suffix = if rec.success { " ✓" } else { " ✗" };
                    last_command_line = Some(format!("{}{}", kind_name, suffix));
                }
            }
        }

        // --- Refresh reason detection ---
        //
        // Compute a small set of lightweight change-detections that the shell cares about.
        // Preference order:
        // 1) Explicit pending reason set by caller (actions).
        // 2) AI projection changed (new explain executed result became available).
        // 3) First-ever refresh should be reported as InitialLoad (stable shell expectation).
        // 4) Active buffer changed (shell cares which buffer is active).
        //    * When a WorkspaceService was provided prefer comparing the opened-buffer
        //      projection's active marker (service authoritative for opened buffers).
        //    * Otherwise fall back to comparing the presenter's active buffer (view).
        // 5) Buffer content changed as observed by the presenter snapshot (BufferUpdated).
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
            // 1) Explicit caller-supplied reason wins.
            pending
        } else if prev_ai_result != new_ai_result {
            // 2) AI projection updates take precedence.
            RefreshReason::AiProjectionUpdated
        } else if self.session_id.is_none() {
            // 3) If this composition has never been refreshed before, treat this as InitialLoad.
            //    This aligns the status bar semantics with shell/harness expectations for the
            //    first refresh lifecycle event.
            RefreshReason::InitialLoad
        } else if current_opened_active.is_some() || prev_opened_active.is_some() {
            // 4) When we have an opened-buffer projection (service used previously or now),
            //    compare the previous opened-active against the current opened-active.
            if prev_opened_active != current_opened_active {
                RefreshReason::ActiveBufferChanged
            } else if prev_active != active_buf_opt {
                // Fallback: also consider presenter-level active buffer changes if they differ.
                RefreshReason::ActiveBufferChanged
            } else if prev_sig != new_sig {
                RefreshReason::BufferUpdated
            } else {
                RefreshReason::RefreshAction
            }
        } else if prev_active != active_buf_opt {
            RefreshReason::ActiveBufferChanged
        } else if prev_sig != new_sig {
            RefreshReason::BufferUpdated
        } else {
            RefreshReason::RefreshAction
        };

        self.session_id = Some(session_id.clone());
        self.workspace_id = workspace_id;

        // Compute metadata and status snapshots derived from the refresh work above.
        let metadata = DesktopMetadata {
            session_id: Some(session_id),
            workspace_id: self.workspace_id.clone(),
            // Prefer service-provided opened-buffer active marker when present; fall back to presenter's active buffer.
            active_buffer: authoritative_active.clone(),
            opened_buffer_count: opened_count,
            opened_buffers: opened_list.clone(),
            active_buffer_details: active_buffer_details.clone(),
            ai_projection: ai_proj.clone(),
            last_command_line: last_command_line.clone(),
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

    /// Tiny read-only projection summarizing the active document for shells.
    ///
    /// Derived from `metadata.active_buffer_details` and the presenter's latest window.
    /// Returns None when no active buffer/details are available.
    pub fn latest_active_document_summary(&self) -> Option<ActiveDocumentSummary> {
        let meta = self.metadata.as_ref()?;
        let abd = meta.active_buffer_details.clone()?;

        // Derive cursor and selection info from the presenter's latest renderable window.
        let win_opt = self.presenter.latest();
        let mut cursor_line: Option<usize> = None;
        let mut cursor_column: Option<usize> = None;
        let mut selection_present = false;
        let mut current_line_snippet: Option<String> = None;

        if let Some(win) = win_opt {
            // Scan spans to find a cursor or selection.
            for line in win.lines.iter() {
                for sp in line.spans.iter() {
                    match sp.kind {
                        crate::view_adapter::InterfaceSpanKind::SelectionCursor | crate::view_adapter::InterfaceSpanKind::Cursor => {
                            cursor_line = Some(line.line_number);
                            cursor_column = Some(sp.start_col);
                        }
                        crate::view_adapter::InterfaceSpanKind::Selection => {
                            selection_present = true;
                        }
                        _ => {}
                    }
                    // stop early if we found both
                    if cursor_line.is_some() && selection_present {
                        break;
                    }
                }
                if cursor_line.is_some() && selection_present {
                    break;
                }
            }

            // If we didn't detect selection while scanning for cursor, do a secondary lightweight check.
            if !selection_present {
                'outer: for line in win.lines.iter() {
                    for sp in line.spans.iter() {
                        if let crate::view_adapter::InterfaceSpanKind::Selection = sp.kind {
                            selection_present = true;
                            break 'outer;
                        }
                    }
                }
            }

            // Determine a reasonable current-line snippet: prefer cursor line, else top_line.
            let snippet_line_no = cursor_line.unwrap_or(win.top_line);
            if let Some(l) = win.lines.iter().find(|l| l.line_number == snippet_line_no) {
                let mut s = String::new();
                for sp in l.spans.iter() {
                    s.push_str(&sp.text);
                }
                // Truncate to 120 Unicode scalars for compactness.
                let snippet: String = s.chars().take(120).collect();
                current_line_snippet = Some(snippet);
            }
        }

        Some(ActiveDocumentSummary {
            buffer_id: meta.active_buffer.clone(),
            display: abd.display,
            line_count: abd.line_count,
            cursor_line,
            cursor_column,
            selection_present,
            current_line_snippet,
        })
    }

    /// Tiny read-only viewport summary derived from the presenter's latest renderable window.
    ///
    /// - Returns None when the presenter has no latest window snapshot.
    /// - The method performs a small deterministic scan of the InterfaceRenderableWindow
    ///   to compute the top line, visible count, total lines, and whether any Cursor /
    ///   SelectionCursor span is present in the visible lines (cursor_visible).
    /// - Anchoring is a best-effort hint: when a cursor line is present and lies strictly
    ///   inside the visible window (not equal to top and not equal to bottom) we prefer
    ///   Centered; if the cursor is exactly at the top we report Top; otherwise Unknown.
    pub fn latest_viewport_summary(&self) -> Option<ViewportSummary> {
        let win = self.presenter.latest()?;
        let top = win.top_line;
        let visible_count = win.lines.len();
        let total = win.total_lines;

        // Determine if any span marks a cursor in the visible window and record its line number.
        let mut cursor_visible = false;
        let mut cursor_line_opt: Option<usize> = None;
        for line in win.lines.iter() {
            for sp in line.spans.iter() {
                match sp.kind {
                    crate::view_adapter::InterfaceSpanKind::Cursor | crate::view_adapter::InterfaceSpanKind::SelectionCursor => {
                        cursor_visible = true;
                        cursor_line_opt = Some(line.line_number);
                        break;
                    }
                    _ => {}
                }
            }
            if cursor_visible {
                break;
            }
        }

        // Heuristic anchoring inference
        let anchoring = if let Some(cursor_line) = cursor_line_opt {
            let bottom = top.saturating_add(visible_count.saturating_sub(1));
            if cursor_line == top {
                ViewportAnchoring::Top
            } else if cursor_line > top && cursor_line < bottom {
                ViewportAnchoring::Centered
            } else {
                ViewportAnchoring::Unknown
            }
        } else {
            ViewportAnchoring::Unknown
        };

        Some(ViewportSummary {
            top_visible_line: top,
            visible_line_count: visible_count,
            total_lines: total,
            cursor_visible,
            anchoring,
        })
    }

    /// Tiny read-only status snapshot indicating which composition projections are populated.
    pub fn latest_status(&self) -> Option<DesktopStatus> {
        self.status.clone()
    }

    /// Tiny, read-only opened-buffers summary derived from the composition metadata.
    ///
    /// Characteristics:
    /// - Always returns an OpenedBuffersSummary (empty when metadata absent).
    /// - Prefers data already present in `metadata.opened_buffers` and `metadata.active_buffer_details`.
    /// - Does not perform any IO or call application ports; purely projection-only.
    pub fn latest_opened_buffers_summary(&self) -> OpenedBuffersSummary {
        if let Some(meta) = &self.metadata {
            // Build per-item summaries. Prefer line_count from active_buffer_details when it matches.
            let mut items: Vec<OpenedBufferItemSummary> = Vec::with_capacity(meta.opened_buffers.len());
            for it in meta.opened_buffers.iter() {
                // Try to obtain line_count from active_buffer_details when it matches the buffer id.
                let mut line_count: usize = 0;
                if let Some(abd) = &meta.active_buffer_details {
                    if abd.buffer_id == it.buffer_id {
                        line_count = abd.line_count;
                    }
                }
                items.push(OpenedBufferItemSummary {
                    buffer_id: it.buffer_id.clone(),
                    display: it.display.clone(),
                    line_count,
                    active: it.active,
                });
            }
            OpenedBuffersSummary {
                count: meta.opened_buffer_count,
                items,
                active: meta.active_buffer.clone(),
            }
        } else {
            OpenedBuffersSummary {
                count: 0,
                items: Vec::new(),
                active: None,
            }
        }
    }

    /// Return the small, read-only AI projection (if any) obtained during the last refresh.
    pub fn latest_ai_projection(&self) -> Option<AiProjection> {
        self.metadata.as_ref().and_then(|m| m.ai_projection.clone())
    }

    /// Return a tiny, read-only AI projection summary intended for shell consumption.
    ///
    /// This function composes the existing AiProjection (if present) into a small,
    /// stable shape suitable for printing and simple diagnostics in shells/harnesses.
    /// - Maps free-form `kind` strings to the small `AiKind` enum using a best-effort,
    ///   case-insensitive substring match.
    /// - Sets `AiState::Ready` when `result` is present; `Running` when a kind is
    ///   declared but no result text is present; otherwise `Failed`.
    ///
    /// Returns None when no AI projection exists in the composition metadata.
    pub fn latest_ai_projection_summary(&self) -> Option<AiProjectionSummary> {
        let ap = self.latest_ai_projection()?;
        // Map kind string to small enum
        let kind_opt = ap.kind.as_ref().map(|k| {
            let kl = k.to_lowercase();
            if kl.contains("explain") {
                AiKind::Explain
            } else if kl.contains("suggest") || kl.contains("suggestion") {
                AiKind::Suggest
            } else if kl.contains("refactor") || kl.contains("refactoring") {
                AiKind::Refactor
            } else {
                AiKind::Other(k.clone())
            }
        });

        // Determine a minimal state hint
        let state = if ap.result.is_some() {
            AiState::Ready
        } else if ap.kind.is_some() {
            AiState::Running
        } else {
            AiState::Failed
        };

        Some(AiProjectionSummary {
            present: true,
            kind: kind_opt,
            target_buffer: ap.target_buffer.clone(),
            state,
        })
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

    /// Return a compact, read-only summary of the composition suitable for shells.
    ///
    /// The summary is derived from existing composition fields and is intentionally
    /// small and readonly. It does not duplicate underlying state — it merely
    /// projects a few commonly-used values into a convenient struct.
    pub fn latest_summary(&self) -> Option<DesktopSummary> {
        // Always return a summary after at least one refresh; we base presence on revision > 0.
        if self.revision == 0 && self.metadata.is_none() && self.status.is_none() {
            return None;
        }

        Some(DesktopSummary {
            revision: self.revision,
            refresh_reason: self.latest_refresh_reason(),
            status: self.status.clone(),
            active_buffer: self.metadata.as_ref().and_then(|m| m.active_buffer.clone()),
        })
    }

    /// Return a tiny, derived shell-facing context object containing the most
    /// immediately useful composition facts for simple shells or UI consumers.
    ///
    /// This accessor is intentionally read-only and derived from existing composition
    /// projections (`metadata`, `status`, and `revision`). It never mutates state.
    pub fn latest_shell_context(&self) -> Option<ShellContext> {
        // Mirror latest_summary presence semantics: require at least one refresh to return a context.
        if self.revision == 0 && self.metadata.is_none() && self.status.is_none() {
            return None;
        }

        // Determine active_display: prefer active_buffer_details.display, fall back to opened_buffers item display.
        let active_display = self
            .metadata
            .as_ref()
            .and_then(|m| {
                m.active_buffer_details
                    .as_ref()
                    .and_then(|d| d.display.clone())
                    .or_else(|| {
                        m.opened_buffers
                            .iter()
                            .find(|i| i.active)
                            .and_then(|i| i.display.clone())
                    })
            });

        let has_ai = self.metadata.as_ref().and_then(|m| m.ai_projection.as_ref()).is_some();

        Some(ShellContext {
            active_buffer: self.metadata.as_ref().and_then(|m| m.active_buffer.clone()),
            active_display,
            latest_revision: self.revision,
            latest_refresh_reason: self.metadata.as_ref().and_then(|m| m.refresh_reason.clone()),
            has_ai_projection: has_ai,
            last_command_line: self.metadata.as_ref().and_then(|m| m.last_command_line.clone()),
        })
    }

    /// Build a tiny, one-line StatusBarLine suitable for shells/harnesses.
    ///
    /// Composition policy (minimal, adapter-local):
    /// - Prefer an AI projection textual result when present: "AI: <result (truncated)>".
    /// - Otherwise present a short text mapped from the latest RefreshReason (e.g. "buffer updated").
    /// - Optionally populate `sticky` with the active-buffer display label (when available).
    /// - Return None when no meaningful status is available (composition not yet refreshed).
    pub fn latest_status_bar_line(&self) -> Option<StatusBarLine> {
        // Require metadata or presenter to have been populated to return a status.
        let meta = match &self.metadata {
            Some(m) => m,
            None => return None,
        };

        // Helper to build sticky display (prefer active_buffer_details.display).
        let sticky = meta
            .active_buffer_details
            .as_ref()
            .and_then(|d| d.display.clone())
            .or_else(|| {
                meta.opened_buffers
                    .iter()
                    .find(|it| it.active)
                    .and_then(|it| it.display.clone())
            });

        // Prefer AI projection result when present.
        if let Some(ai) = meta.ai_projection.as_ref() {
            if let Some(result) = ai.result.as_ref() {
                // Truncate to keep status short and stable.
                let snippet: String = if result.chars().count() > 120 {
                    result.chars().take(120).collect::<String>() + "..."
                } else {
                    result.clone()
                };
                return Some(StatusBarLine { text: format!("AI: {}", snippet), sticky });
            }
        }

        // Fallback to mapping refresh reason to a concise single-line message.
        if let Some(rr) = meta.refresh_reason.as_ref() {
            let text = match rr {
                RefreshReason::InitialLoad => "initial load".to_string(),
                RefreshReason::RefreshAction => "refreshed".to_string(),
                RefreshReason::CursorMoved => "cursor moved".to_string(),
                RefreshReason::BufferUpdated => "buffer updated".to_string(),
                RefreshReason::ActiveBufferChanged => "active buffer changed".to_string(),
                RefreshReason::AiProjectionUpdated => "AI projection updated".to_string(),
            };
            return Some(StatusBarLine { text, sticky });
        }

        None
    }

    /// Build a small, convenience ShellSnapshot that aggregates existing shell-facing projections.
    ///
    /// Notes:
    /// - This method is intentionally tiny and calls the existing accessors:
    ///   latest_shell_context(), latest_active_document_summary(), latest_viewport_summary(),
    ///   latest_ai_projection_summary(), latest_opened_buffers_summary().
    /// - Returns None when no shell context is available (mirrors latest_shell_context semantics).
    /// - The ShellSnapshot is a read-only convenience for shells and harnesses; it does not
    ///   duplicate or re-derive any projection logic.
    pub fn latest_shell_snapshot(&self) -> Option<ShellSnapshot> {
        // Require at least the shell context to produce a snapshot.
        let ctx = self.latest_shell_context()?;
        let active_document = self.latest_active_document_summary();
        let viewport = self.latest_viewport_summary();
        let ai_summary = self.latest_ai_projection_summary();
        let opened_buffers = self.latest_opened_buffers_summary();

        Some(ShellSnapshot {
            context: ctx,
            active_document,
            viewport,
            ai_summary,
            opened_buffers,
        })
    }

    /// Produce a tiny, read-only consistency report derived from the current composition state.
    ///
    /// This function intentionally performs only a few conservative checks that are cheap
    /// and deterministic to compute from existing fields. It is meant to be shell-facing
    /// and to aid harnesses in printing or asserting composition coherence.
    pub fn latest_consistency_report(&self) -> DesktopConsistencyReport {
        // 1) summary status presence vs actual status presence
        let summary_has_status = self.latest_summary().and_then(|s| s.status).is_some();
        let status_present = self.status.is_some();
        let status_present_matches_summary = summary_has_status == status_present;

        // 2) active buffer alignment with active-buffer-details
        let meta_active = self.metadata.as_ref().and_then(|m| m.active_buffer.clone());
        let abd_opt = self.latest_active_buffer_details();
        let active_buffer_matches_details = if meta_active.is_some() {
            match abd_opt {
                Some(abd) => abd.buffer_id == meta_active.unwrap(),
                None => false,
            }
        } else {
            // Nothing asserted by metadata -> treat as OK
            true
        };

        // 3) active buffer is among opened buffers when opened list non-empty
        let active_buffer_in_opened_buffers = match &self.metadata {
            Some(meta) => {
                if meta.opened_buffers.is_empty() {
                    true
                } else {
                    match meta.active_buffer.clone() {
                        Some(act) => meta.opened_buffers.iter().any(|i| i.buffer_id == act),
                        None => true,
                    }
                }
            }
            None => true,
        };

        // 4) presenter window presence aligns with status.has_render_window
        let presenter_has = self.presenter.latest().is_some();
        let status_has_render = self.status.as_ref().map(|s| s.has_render_window).unwrap_or(false);
        let presenter_window_matches_status = presenter_has == status_has_render;

        let overall_ok = status_present_matches_summary
            && active_buffer_matches_details
            && active_buffer_in_opened_buffers
            && presenter_window_matches_status;

        DesktopConsistencyReport {
            status_present_matches_summary,
            active_buffer_matches_details,
            active_buffer_in_opened_buffers,
            presenter_window_matches_status,
            overall_ok,
        }
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

    #[tokio::test]
    async fn latest_summary_reflects_composition_state() {
        let v = FakeView::new();
        let arc: Arc<dyn WorkspaceView> = Arc::new(v);
        let sid = SessionId(zaroxi_kernel_types::Id::new());
        let wid = zaroxi_kernel_types::Id::new();

        let mut comp = DesktopComposition::new();
        comp.refresh(arc.clone(), sid.clone(), Some(wid.clone())).await.expect("refresh ok");

        let summary = comp.latest_summary().expect("summary present");
        assert_eq!(summary.revision, comp.latest_revision());
        assert_eq!(summary.refresh_reason, comp.latest_refresh_reason());
        let status = comp.latest_status().expect("status present");
        assert!(summary.status.is_some());
        assert_eq!(summary.status.unwrap().has_render_window, status.has_render_window);
        assert_eq!(summary.active_buffer, comp.latest_metadata().and_then(|m| m.active_buffer));
    }

    #[tokio::test]
    async fn desktop_composition_consistency_report_is_valid() {
        use std::sync::Arc;
        use uuid::Uuid;
        use chrono::Utc;

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

        let report = comp.latest_consistency_report();
        assert!(report.overall_ok, "consistency report should be OK in this basic happy path");
        assert!(report.status_present_matches_summary);
        assert!(report.active_buffer_matches_details);
        assert!(report.active_buffer_in_opened_buffers);
        assert!(report.presenter_window_matches_status);
    }

    #[tokio::test]
    async fn latest_shell_context_is_composed() {
        use std::sync::Arc;
        use uuid::Uuid;
        use chrono::Utc;

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
                        kind: crate::ports::WorkspaceEventKind::ExplainExecuted { buffer_id: buf.clone(), result: "ctx-explain".to_string() },
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

        let ctx = comp.latest_shell_context().expect("context present");
        assert_eq!(ctx.latest_revision, comp.latest_revision());
        assert_eq!(ctx.active_buffer.unwrap(), crate::ports::BufferId::from("buf:fake"));
        assert_eq!(ctx.active_display.unwrap(), "fake".to_string());
        assert_eq!(ctx.latest_refresh_reason.unwrap(), RefreshReason::AiProjectionUpdated);
        assert!(ctx.has_ai_projection);
    }
}
