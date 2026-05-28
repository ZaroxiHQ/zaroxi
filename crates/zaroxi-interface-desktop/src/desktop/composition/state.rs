/*!
Composition state module.

Responsibilities:
- Define composition-facing DTOs stored in-memory (metadata/status/summary types).
- Define DesktopComposition (presenter + stored fields).
- Implement small, side-effect free accessors and thin delegations to focused submodules.
*/

use std::sync::Arc;

use crate::presenter::Presenter;
use crate::view_adapter::InterfaceRenderableWindow;
use zaroxi_application_workspace::ports::SessionId;
use zaroxi_kernel_types::Id;

/// Single opened-buffer projection item exposed to the shell.
#[derive(Clone, Debug)]
pub struct OpenedBufferItem {
    pub buffer_id: crate::ports::BufferId,
    pub display: Option<String>,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct ActiveBufferDetails {
    pub buffer_id: crate::ports::BufferId,
    pub display: Option<String>,
    pub line_count: usize,
}

#[derive(Clone, Debug)]
pub struct ActiveDocumentSummary {
    pub buffer_id: Option<crate::ports::BufferId>,
    pub display: Option<String>,
    pub line_count: usize,
    pub cursor_line: Option<usize>,
    pub cursor_column: Option<usize>,
    pub selection_present: bool,
    pub current_line_snippet: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewportAnchoring {
    Top,
    Centered,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct ViewportSummary {
    pub top_visible_line: usize,
    pub visible_line_count: usize,
    pub total_lines: usize,
    pub cursor_visible: bool,
    pub anchoring: ViewportAnchoring,
}

#[derive(Clone, Debug)]
pub struct AiProjection {
    pub kind: Option<String>,
    pub result: Option<String>,
    pub target_buffer: Option<crate::ports::BufferId>,
    /// Optional full proposal payload (when an edit is proposed).
    pub proposal_text: Option<String>,
    /// Current state of the AI projection (idle / proposed / applied / failed).
    pub state: Option<AiState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiKind {
    Explain,
    Suggest,
    Refactor,
    /// New: Edit kind for AI-produced edits that may be applied to a buffer.
    Edit,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiState {
    /// No AI activity present.
    Idle,
    /// Provider ready (idle-but-ready).
    Ready,
    /// Provider is running/generating.
    Running,
    /// A proposal has been produced and is awaiting explicit apply.
    Proposed,
    /// Proposal has been applied to the buffer.
    Applied,
    /// Provider or apply failed.
    Failed,
}

#[derive(Clone, Debug)]
pub struct AiProjectionSummary {
    pub present: bool,
    pub kind: Option<AiKind>,
    pub target_buffer: Option<crate::ports::BufferId>,
    pub state: AiState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefreshReason {
    InitialLoad,
    RefreshAction,
    CursorMoved,
    BufferUpdated,
    ActiveBufferChanged,
    AiProjectionUpdated,
}

#[allow(dead_code)]
pub(crate) fn command_kind_short_name(kind: &crate::ports::CommandKind) -> &'static str {
    // Prefer concise variant names for small status lines (avoid Debug output with fields).
    match kind {
        crate::ports::CommandKind::BootWorkspace { .. } => "BootWorkspace",
        crate::ports::CommandKind::OpenBuffer { .. } => "OpenBuffer",
        crate::ports::CommandKind::UpdateBuffer { .. } => "UpdateBuffer",
        crate::ports::CommandKind::SetActiveBuffer { .. } => "SetActiveBuffer",
        crate::ports::CommandKind::ExplainActiveBuffer => "ExplainActiveBuffer",
        crate::ports::CommandKind::DispatchAppCommand { .. } => "DispatchAppCommand",
    }
}

#[derive(Clone, Debug)]
pub struct DesktopMetadata {
    pub session_id: Option<SessionId>,
    pub workspace_id: Option<Id>,
    pub active_buffer: Option<crate::ports::BufferId>,
    pub opened_buffer_count: usize,
    pub opened_buffers: Vec<OpenedBufferItem>,
    pub active_buffer_details: Option<ActiveBufferDetails>,
    pub ai_projection: Option<AiProjection>,
    pub visible_window: Option<crate::desktop::projections::VisibleWindowBasic>,
    pub last_command_line: Option<String>,
    pub refresh_reason: Option<RefreshReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopStatus {
    pub has_render_window: bool,
    pub has_metadata: bool,
    pub has_active_buffer_details: bool,
    pub has_opened_buffers: bool,
    pub has_ai_projection: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedBufferItemSummary {
    pub buffer_id: crate::ports::BufferId,
    pub display: Option<String>,
    pub line_count: usize,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedBuffersSummary {
    pub count: usize,
    pub items: Vec<OpenedBufferItemSummary>,
    pub active: Option<crate::ports::BufferId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopSummary {
    pub revision: u64,
    pub refresh_reason: Option<RefreshReason>,
    pub status: Option<DesktopStatus>,
    pub active_buffer: Option<crate::ports::BufferId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellContext {
    pub active_buffer: Option<crate::ports::BufferId>,
    pub active_display: Option<String>,
    pub latest_revision: u64,
    pub latest_refresh_reason: Option<RefreshReason>,
    pub has_ai_projection: bool,
    pub last_command_line: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBarLine {
    pub text: String,
    pub sticky: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Command {
    Refresh,
    OpenBuffer,
    SetActiveBuffer,
    ExplainActive,
    RequestCloseActive,
    ConfirmSaveAndClose,
    ConfirmDiscardAndClose,
    ConfirmCancelClose,
}

#[derive(Clone, Debug)]
pub struct CommandBarState {
    pub open: bool,
    pub commands: Vec<String>,
    pub selected: usize,
    pub staged_arg: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ShellSnapshot {
    pub context: ShellContext,
    pub active_document: Option<ActiveDocumentSummary>,
    pub viewport: Option<ViewportSummary>,
    pub ai_summary: Option<AiProjectionSummary>,
    pub opened_buffers: OpenedBuffersSummary,
}

/* DesktopConsistencyReport is provided by crate::desktop::consistency */
#[derive(Clone, Debug)]
pub struct DesktopComposition {
    pub(crate) presenter: Presenter,
    pub(crate) session_id: Option<SessionId>,
    pub(crate) workspace_id: Option<Id>,
    pub(crate) metadata: Option<DesktopMetadata>,
    pub(crate) status: Option<DesktopStatus>,
    pub(crate) revision: u64,
    pub(crate) pending_refresh_reason: Option<RefreshReason>,
    pub(crate) pending_close: Option<crate::PendingClose>,
    pub(crate) command_bar: Option<CommandBarState>,
    /// When set, this explicit close-result status should be preferred by
    /// visible status helpers over transient refresh/update messages.
    pub(crate) close_result_status: Option<String>,
}

impl DesktopComposition {
    pub fn new() -> Self {
        Self {
            presenter: Presenter::new(),
            session_id: None,
            workspace_id: None,
            metadata: None,
            status: None,
            revision: 0,
            pending_refresh_reason: None,
            pending_close: None,
            command_bar: None,
            close_result_status: None,
        }
    }

    pub async fn refresh(
        &mut self,
        view: Arc<dyn crate::ports::WorkspaceView>,
        session_id: SessionId,
        workspace_id: Option<Id>,
    ) -> Result<(), String> {
        self.refresh_with_service(view, session_id, workspace_id, None).await
    }

    pub async fn refresh_with_service(
        &mut self,
        view: Arc<dyn crate::ports::WorkspaceView>,
        session_id: SessionId,
        workspace_id: Option<Id>,
        service: Option<Arc<dyn crate::ports::WorkspaceService>>,
    ) -> Result<(), String> {
        // Delegate to the focused refresh module.
        super::refresh::refresh_with_service(self, view, session_id, workspace_id, service).await
    }

    pub fn latest_window(&self) -> Option<InterfaceRenderableWindow> {
        let win_opt = self.presenter.latest();
        win_opt.map(|mut w| {
            for line in w.lines.iter_mut() {
                for sp in line.spans.iter_mut() {
                    match sp.kind {
                        crate::view_adapter::InterfaceSpanKind::SelectionCursor
                        | crate::view_adapter::InterfaceSpanKind::Cursor
                        | crate::view_adapter::InterfaceSpanKind::Selection => {
                            sp.text.clear();
                        }
                        _ => {
                            if sp.text.contains("|^|") || sp.text.contains("|/|/") {
                                sp.text = sp.text.replace("|^|", "").replace("|/|/", "");
                            }
                        }
                    }
                }

                let mut combined = String::new();
                for sp in line.spans.iter() {
                    if matches!(sp.kind, crate::view_adapter::InterfaceSpanKind::Normal) {
                        combined.push_str(&sp.text);
                    }
                }

                let mut cleaned = combined.replace("|^|", "").replace("|/|/", "");
                if cleaned.starts_with("/ ") {
                    cleaned = cleaned.replacen("/ ", "", 1);
                }

                if cleaned != combined {
                    if let Some(first_normal_idx) = line.spans.iter().position(|s| {
                        matches!(s.kind, crate::view_adapter::InterfaceSpanKind::Normal)
                    }) {
                        line.spans[first_normal_idx].text = cleaned.clone();
                        for sp in line.spans.iter_mut().skip(first_normal_idx + 1) {
                            if matches!(sp.kind, crate::view_adapter::InterfaceSpanKind::Normal) {
                                sp.text.clear();
                            }
                        }
                    }
                }
            }
            w
        })
    }

    pub fn get_session_id(&self) -> Option<SessionId> {
        self.session_id.clone()
    }

    pub fn get_workspace_id(&self) -> Option<Id> {
        self.workspace_id.clone()
    }

    pub fn latest_metadata(&self) -> Option<DesktopMetadata> {
        self.metadata.clone()
    }

    pub fn latest_active_buffer_details(&self) -> Option<ActiveBufferDetails> {
        self.metadata.as_ref().and_then(|m| m.active_buffer_details.clone())
    }

    pub fn latest_active_document_summary(&self) -> Option<ActiveDocumentSummary> {
        super::projections::latest_active_document_summary(self)
    }

    pub fn latest_viewport_summary(&self) -> Option<ViewportSummary> {
        // The small visible-window projection lives at crate::desktop::projections.
        crate::desktop::projections::latest_viewport_summary(self)
    }

    pub fn latest_status(&self) -> Option<DesktopStatus> {
        self.status.clone()
    }

    pub fn latest_opened_buffers_summary(&self) -> OpenedBuffersSummary {
        super::projections::latest_opened_buffers_summary(self)
    }

    pub fn latest_ai_projection(&self) -> Option<AiProjection> {
        self.metadata.as_ref().and_then(|m| m.ai_projection.clone())
    }

    pub fn latest_ai_projection_summary(&self) -> Option<AiProjectionSummary> {
        super::summary::latest_ai_projection_summary(self)
    }

    pub fn latest_revision(&self) -> u64 {
        self.revision
    }

    pub fn set_pending_refresh_reason(&mut self, reason: RefreshReason) {
        self.pending_refresh_reason = Some(reason);
    }

    pub fn has_pending_refresh_reason(&self) -> bool {
        self.pending_refresh_reason.is_some()
    }

    // Pending-close helpers delegated to the small pending_close module (desktop-level).
    pub fn set_pending_close(&mut self, pending: crate::PendingClose) {
        // When entering a new pending-close flow, any previously preserved explicit
        // close-result status must not remain visible. Clear it here so the pending
        // close banner can take precedence immediately.
        self.clear_close_result_status();
        crate::desktop::pending_close::set_pending_close(self, pending);
    }

    pub fn clear_pending_close(&mut self) {
        crate::desktop::pending_close::clear_pending_close(self);
    }

    pub fn has_pending_close(&self) -> bool {
        crate::desktop::pending_close::has_pending_close(self)
    }

    pub fn latest_pending_close(&self) -> Option<crate::PendingClose> {
        crate::desktop::pending_close::latest_pending_close(self)
    }

    /// Remove an opened buffer from the composition's opened-buffer projection.
    ///
    /// Returns true when a buffer was found and removed; false otherwise.
    ///
    /// Deterministic active-buffer selection policy:
    /// - If the removed buffer was active, prefer the previous neighbor (index-1),
    ///   otherwise pick the first buffer (index 0) when any remain.
    /// - If no buffers remain, clear active buffer and active buffer details.
    pub fn close_opened_buffer(&mut self, buffer_id: &crate::ports::BufferId) -> bool {
        if let Some(m) = self.metadata.as_mut() {
            if let Some(pos) = m.opened_buffers.iter().position(|it| &it.buffer_id == buffer_id) {
                let was_active = m.active_buffer.as_ref().map(|b| b == buffer_id).unwrap_or(false);
                m.opened_buffers.remove(pos);
                m.opened_buffer_count = m.opened_buffers.len();

                if was_active {
                    if m.opened_buffers.is_empty() {
                        m.active_buffer = None;
                        m.active_buffer_details = None;
                    } else {
                        let new_idx = if pos > 0 { pos - 1 } else { 0 };
                        let new_buf = m.opened_buffers[new_idx].buffer_id.clone();
                        m.active_buffer = Some(new_buf.clone());
                        let display = m
                            .opened_buffers
                            .iter()
                            .find(|it| it.buffer_id == new_buf)
                            .and_then(|it| it.display.clone());
                        m.active_buffer_details = Some(ActiveBufferDetails {
                            buffer_id: new_buf,
                            display,
                            line_count: 0,
                        });
                    }
                } else {
                    // Ensure active_buffer still refers to a present buffer; if not, clear details.
                    if let Some(ab) = &m.active_buffer {
                        if !m.opened_buffers.iter().any(|it| &it.buffer_id == ab) {
                            m.active_buffer = None;
                            m.active_buffer_details = None;
                        }
                    }
                }

                return true;
            }
        }
        false
    }

    pub fn set_status_message(&mut self, text: String) {
        if let Some(m) = self.metadata.as_mut() {
            m.last_command_line = Some(text);
        } else {
            self.metadata = Some(DesktopMetadata {
                session_id: self.session_id.clone(),
                workspace_id: self.workspace_id.clone(),
                active_buffer: None,
                opened_buffer_count: 0,
                opened_buffers: Vec::new(),
                active_buffer_details: None,
                ai_projection: None,
                visible_window: None,
                last_command_line: Some(text),
                refresh_reason: None,
            });
        }
    }

    /// Set a final close-result status message and clear any pending-close state.
    ///
    /// This centralizes the visible outcome reported to the shell after a confirm action
    /// (save-and-close or discard-and-close). It ensures the pending-close banner is removed
    /// and the same message is available via the status-bar helpers and any harness readers.
    pub fn set_close_result_status(&mut self, text: String) {
        // Ensure pending-close is cleared before setting the final status.
        self.clear_pending_close();

        // Preserve an explicit close-result status separately so it will survive
        // an immediately-following refresh that may update the generic
        // metadata.last_command_line. We still populate metadata.last_command_line
        // for backward compatibility with any consumers that read that field.
        self.close_result_status = Some(text.clone());
        self.set_status_message(text);
    }

    pub fn clear_status_message(&mut self) {
        if let Some(m) = self.metadata.as_mut() {
            m.last_command_line = None;
        }
    }

    /// Clear any preserved explicit close-result status. Call this to allow
    /// subsequent generic refresh/update messages to be surfaced normally.
    pub fn clear_close_result_status(&mut self) {
        self.close_result_status = None;
    }

    /// Perform a local in-process "close" of the composition to reflect a session/window
    /// being closed. This clears session metadata and presenter snapshots so callers
    /// (harnesses/shells) may observe a closed state without performing a process exit.
    /// This intentionally stays UI-facing and does not attempt to tear down application
    /// state (that should be owned by the orchestrator).
    pub fn perform_session_close(&mut self) {
        self.pending_close = None;
        self.command_bar = None;
        self.metadata = None;
        self.status = None;
        self.presenter = Presenter::new();
        self.session_id = None;
        self.workspace_id = None;
        self.revision = 0;
    }

    // Command-bar helpers delegated to command_bar module (desktop-level).
    pub fn open_command_bar(&mut self) {
        crate::desktop::command_bar::open_command_bar(self);
    }

    pub fn close_command_bar(&mut self) {
        crate::desktop::command_bar::close_command_bar(self);
    }

    pub fn toggle_command_bar(&mut self) {
        crate::desktop::command_bar::toggle_command_bar(self);
    }

    pub fn is_command_bar_open(&self) -> bool {
        crate::desktop::command_bar::is_command_bar_open(self)
    }

    pub fn latest_command_bar(&self) -> Option<CommandBarState> {
        crate::desktop::command_bar::latest_command_bar(self)
    }

    pub fn select_next_command(&mut self) {
        crate::desktop::command_bar::select_next_command(self)
    }

    pub fn select_prev_command(&mut self) {
        crate::desktop::command_bar::select_prev_command(self)
    }

    pub fn set_command_bar_staged_arg(&mut self, arg: Option<String>) {
        crate::desktop::command_bar::set_command_bar_staged_arg(self, arg);
    }

    pub fn latest_refresh_reason(&self) -> Option<RefreshReason> {
        self.metadata.as_ref().and_then(|m| m.refresh_reason.clone())
    }

    pub fn latest_summary(&self) -> Option<DesktopSummary> {
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

    pub fn latest_shell_context(&self) -> Option<ShellContext> {
        super::projections::latest_shell_context(self)
    }

    pub fn latest_status_bar_line(&self) -> Option<StatusBarLine> {
        // If a close-result status is currently preserved prefer it over any
        // transient refresh/update status. This makes explicit close results
        // survive the immediate refresh path as required by the harness.
        if let Some(cr) = self.close_result_status.clone() {
            return Some(StatusBarLine { text: cr, sticky: None });
        }
        crate::desktop::status::latest_status_bar_line(self)
    }

    pub fn latest_shell_snapshot(&self) -> Option<ShellSnapshot> {
        crate::desktop::snapshot::latest_shell_snapshot(self)
    }

    pub fn latest_consistency_report(&self) -> crate::desktop::consistency::DesktopConsistencyReport {
        crate::desktop::consistency::latest_consistency_report(self)
    }
}
