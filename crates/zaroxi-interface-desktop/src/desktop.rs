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
mod composition;
mod projections;
mod consistency;
pub use projections::VisibleWindowBasic;
pub use consistency::DesktopConsistencyReport;

/// Helper: convert CommandKind to short name string for tiny shell-facing LastCommandLine.
fn command_kind_short_name(kind: &crate::ports::CommandKind) -> &'static str {
    match kind {
        crate::ports::CommandKind::BootWorkspace { .. } => "BootWorkspace",
        crate::ports::CommandKind::OpenBuffer { .. } => "OpenBuffer",
        crate::ports::CommandKind::UpdateBuffer { .. } => "UpdateBuffer",
        crate::ports::CommandKind::SetActiveBuffer { .. } => "SetActiveBuffer",
        crate::ports::CommandKind::ExplainActiveBuffer => "ExplainActiveBuffer",
        crate::ports::CommandKind::DispatchAppCommand { .. } => "DispatchAppCommand",
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
    /// New: best-effort visible-window projection when available from WorkspaceView.
    /// This strengthens the editor viewport path by preferring direct VisibleLinesWindow
    /// data over presentation transcripts when present.
    pub visible_window: Option<VisibleWindowBasic>,
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

/* DesktopConsistencyReport moved to desktop/consistency.rs */
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
        composition::refresh_with_service(self, view, session_id, workspace_id, service).await
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
        composition::latest_active_document_summary(self)
    }

    pub fn latest_viewport_summary(&self) -> Option<ViewportSummary> {
        projections::latest_viewport_summary(self)
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
        composition::latest_opened_buffers_summary(self)
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
        composition::latest_shell_context(self)
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

    pub fn latest_consistency_report(&self) -> DesktopConsistencyReport {
        consistency::latest_consistency_report(self)
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
