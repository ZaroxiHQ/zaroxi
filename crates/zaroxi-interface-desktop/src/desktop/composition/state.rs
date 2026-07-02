/*!
Composition state module.

Responsibilities:
- Define composition-facing DTOs stored in-memory (metadata/status/summary types).
- Define DesktopComposition (presenter + stored fields).
- Implement small, side-effect free accessors and thin delegations to focused submodules.

Shared workspace-view DTOs (OpenedBufferItemSummary, OpenedBuffersSummary,
ActiveDocumentSummary, ViewportSummary, ShellContext, RefreshReason,
VisibleWindowBasic) live in `zaroxi-application-workspace::workspace_view`
and are re-exported here for backward compatibility.
*/

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use crate::presenter::Presenter;
use crate::view_adapter::InterfaceRenderableWindow;
use zaroxi_application_workspace::WorkspaceExplorer;
use zaroxi_application_workspace::ports::SessionId;
use zaroxi_domain_workspace::file_tree::ExplorerItemView;
use zaroxi_kernel_types::Id;

// Imports and re-exports: these types live in zaroxi-application-workspace
// but downstream code references them via super::* / crate::desktop::*.
pub use zaroxi_application_workspace::workspace_view::{
    ActiveBufferDetails, ActiveDocumentSummary, OpenedBufferItemSummary, OpenedBuffersSummary,
    RefreshReason, ShellContext, ViewportAnchoring, ViewportSummary, VisibleWindowBasic,
};

/// Single opened-buffer projection item exposed to the shell.
#[derive(Clone, Debug)]
pub struct OpenedBufferItem {
    pub buffer_id: crate::ports::BufferId,
    pub display: Option<String>,
    pub active: bool,
}

/// Canonical filesystem path for an opened-buffer identity — the `buf:`
/// transport prefix stripped.  Single source of truth for comparing two
/// opened-buffer entries so the `buf:path` and plain `path` forms can
/// never be treated as different documents.
pub fn canonical_buffer_path(bid: &crate::ports::BufferId) -> String {
    let s = bid.to_string();
    s.strip_prefix("buf:").unwrap_or(&s).to_string()
}

/// Whether two buffer ids refer to the same document (canonical-path equal).
pub fn same_opened_document(a: &crate::ports::BufferId, b: &crate::ports::BufferId) -> bool {
    canonical_buffer_path(a) == canonical_buffer_path(b)
}

/// Enforce the hard invariant `one canonical path == one opened_buffers
/// entry`.  The first entry for a path wins; later duplicates are dropped
/// but their `active` flag is preserved onto the surviving entry so the
/// active tab is never lost by dedupe.  Emits a structured diagnostic for
/// every duplicate collapsed (gated behind `ZAROXI_DEBUG_VISIBLE_TABS`).
pub fn dedupe_opened_buffers(list: &mut Vec<OpenedBufferItem>) {
    let before = list.len();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut i = 0;
    while i < list.len() {
        let key = canonical_buffer_path(&list[i].buffer_id);
        if let Some(&keep) = seen.get(&key) {
            if list[i].active {
                list[keep].active = true;
            }
            if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
                eprintln!("ZAROXI_VISIBLE_TAB_MODEL: duplicate_open_detected path={key}");
            }
            list.remove(i);
        } else {
            seen.insert(key, i);
            i += 1;
        }
    }
    if before != list.len() && std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
        eprintln!(
            "ZAROXI_VISIBLE_TAB_MODEL: opened_buffers_normalized before={before} after={}",
            list.len(),
        );
    }
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
    pub diagnostics_snapshot: Option<crate::diagnostics::DiagnosticsSnapshot>,
    pub visible_window: Option<VisibleWindowBasic>,
    /// Editor viewport height expressed as the number of text lines that fit.
    /// Updated by the GUI whenever the editor layout/viewport changes.
    pub editor_viewport_line_count: Option<usize>,
    /// Local vertical scroll top_line for gui_shell (no workspace refresh loop).
    /// Updated by apply_pending_scrolls, consumed to set content_offset_y.
    pub editor_scroll_top_line: usize,
    /// Sub-pixel vertical scroll offset (logical pixels) for smooth scrolling.
    /// Used directly as content_offset_y on render blocks.
    pub editor_scroll_px: f32,
    /// Tracks the last window_height synced to the workspace to avoid
    /// redundant set_viewport_state calls that would reset top_line.
    pub last_synced_window_height: Option<usize>,
    /// Horizontal scroll offset in logical pixels for the editor viewport.
    pub editor_horizontal_offset_px: Option<f32>,
    pub last_command_line: Option<String>,
    pub refresh_reason: Option<RefreshReason>,
}

impl Default for DesktopMetadata {
    fn default() -> Self {
        Self {
            session_id: None,
            workspace_id: None,
            active_buffer: None,
            opened_buffer_count: 0,
            opened_buffers: Vec::new(),
            active_buffer_details: None,
            ai_projection: None,
            diagnostics_snapshot: None,
            visible_window: None,
            editor_viewport_line_count: None,
            editor_scroll_top_line: 0,
            editor_scroll_px: 0.0,
            last_synced_window_height: None,
            editor_horizontal_offset_px: None,
            last_command_line: None,
            refresh_reason: None,
        }
    }
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
pub struct DesktopSummary {
    pub revision: u64,
    pub refresh_reason: Option<RefreshReason>,
    pub status: Option<DesktopStatus>,
    pub active_buffer: Option<crate::ports::BufferId>,
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

// Re-export CommandBarState from app-workspace (moved in Phase 11).
pub use zaroxi_application_workspace::workspace_view::CommandBarState;

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
    pub metadata: Option<DesktopMetadata>,
    pub(crate) status: Option<DesktopStatus>,
    pub(crate) pending_removed_buffer_ids: Vec<String>,
    /// BufferIds that were added directly (not via the workspace service),
    /// e.g. large files loaded through PieceTable.  The refresh cycle merges
    /// these back into `opened_buffers` so they survive service-issued
    /// metadata rebuilds.  Entries are removed when `close_opened_buffer`
    /// processes the corresponding id.
    pub(crate) direct_buffer_ids: Vec<crate::ports::BufferId>,
    pub(crate) revision: u64,
    pub(crate) pending_refresh_reason: Option<RefreshReason>,
    pub(crate) pending_close: Option<crate::PendingClose>,
    pub(crate) command_bar: Option<CommandBarState>,
    /// When set, this explicit close-result status should be preferred by
    /// visible status helpers over transient refresh/update messages.
    pub(crate) close_result_status: Option<String>,
    /// Workspace root path used to load the explorer tree.
    pub workspace_root_path: Option<PathBuf>,
    /// In-memory explorer tree loaded from the workspace root.
    pub maybe_explorer: Option<WorkspaceExplorer>,
    /// Cache of visible explorer items for activation dispatch mapping.
    pub(crate) cached_explorer_items: Vec<ExplorerItemView>,
    /// Active explorer search/filter query (empty = no filter). Drives whether
    /// `refresh_cached_explorer_items` produces the normal tree or a full-tree
    /// filtered + auto-expanded view.
    pub(crate) explorer_search_query: String,
    /// Pending vertical scroll delta in lines. Consumed by refresh_with_service
    /// to call scroll_viewport on the workspace. Negative = scroll up, positive = down.
    pub pending_scroll_lines: isize,
    /// Pending vertical scroll delta in logical pixels.  Accumulated from
    /// wheel/trackpad events and consumed by apply_pending_scrolls each frame
    /// to produce editor_scroll_px (sub-pixel smooth scrolling).
    pub pending_vscroll_px: f32,
    /// Pending horizontal scroll delta in pixels. Consumed by refresh_with_service
    /// to update the editor horizontal offset for long-line scrolling.
    pub pending_hscroll_px: f32,
}

impl Default for DesktopComposition {
    fn default() -> Self {
        Self::new()
    }
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
            workspace_root_path: None,
            maybe_explorer: None,
            cached_explorer_items: Vec::new(),
            explorer_search_query: String::new(),
            pending_scroll_lines: 0,
            pending_vscroll_px: 0.0,
            pending_hscroll_px: 0.0,
            pending_removed_buffer_ids: Vec::new(),
            direct_buffer_ids: Vec::new(),
        }
    }

    /// Store the editor viewport visible line count so the refresh loop
    /// can sync it to the workspace's ViewportState on next refresh.
    pub fn set_editor_viewport_lines(&mut self, lines: usize) {
        if self.metadata.is_none() {
            self.metadata = Some(DesktopMetadata::default());
        }
        if let Some(ref mut meta) = self.metadata {
            meta.editor_viewport_line_count = Some(lines);
        }
    }

    /// Process pending scroll deltas synchronously (for GUI event-loop use).
    ///
    /// Canonical scroll architecture:
    /// - `editor_scroll_top_line` is the single source of truth for vertical position.
    /// - `editor_scroll_px` is ALWAYS derived: `top_line * LINE_HEIGHT` (line-snapped).
    /// - Input is accumulated as pixel deltas for smooth feel, then snapped to whole
    ///   lines on apply so text/gutter rows stay line-aligned (no partial-line shifts).
    /// - Normalized offset (for scrollbar thumb) is derived from top_line / max_scroll.
    pub fn apply_pending_scrolls(&mut self) {
        let vscroll = self.pending_scroll_lines;
        self.pending_scroll_lines = 0;
        let vscroll_px = self.pending_vscroll_px;
        self.pending_vscroll_px = 0.0;
        let hscroll = self.pending_hscroll_px;
        self.pending_hscroll_px = 0.0;

        if self.metadata.is_none() {
            self.metadata = Some(DesktopMetadata::default());
        }
        let meta = self.metadata.as_mut().unwrap();

        let visible = meta.editor_viewport_line_count.unwrap_or(10).max(1);
        let total = meta.active_buffer_details.as_ref().map(|d| d.line_count).unwrap_or(0);
        let max_scroll = total.saturating_sub(visible);

        let scroll_trace = std::env::var("ZAROXI_DEBUG_SCROLL").as_deref() == Ok("1");
        let wants_scroll = vscroll_px.abs() > 0.01 || vscroll != 0;
        if scroll_trace && wants_scroll {
            let path = meta
                .active_buffer_details
                .as_ref()
                .map(|d| d.buffer_id.to_string())
                .unwrap_or_else(|| "<none>".to_string());
            eprintln!(
                "ZAROXI_SCROLL: scroll_target path={} line_count={} viewport_lines={} top={}",
                path.strip_prefix("buf:").unwrap_or(&path),
                total,
                visible,
                meta.editor_scroll_top_line,
            );
            if max_scroll == 0 {
                let reason = if total == 0 { "zero_line_count" } else { "clamped_zero" };
                eprintln!("ZAROXI_SCROLL: scroll_blocked reason={reason}");
            }
        }

        if vscroll_px.abs() > 0.01 {
            let line_delta = (-vscroll_px / 16.0).round() as isize;
            let current = meta.editor_scroll_top_line as isize;
            let new_unclamped = (current + line_delta).max(0);
            let new = new_unclamped.min(max_scroll as isize).max(0) as usize;
            meta.editor_scroll_top_line = new;
            meta.editor_scroll_px = new as f32 * 16.0;
            if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_SCROLL: applied vscroll_px={:.1} line_delta={} top_line={}",
                    vscroll_px, line_delta, meta.editor_scroll_top_line
                );
            }
        } else if vscroll != 0 {
            let current = meta.editor_scroll_top_line as isize;
            let new_unclamped = (current + vscroll).max(0);
            let new = new_unclamped.min(max_scroll as isize).max(0) as usize;
            meta.editor_scroll_top_line = new;
            meta.editor_scroll_px = new as f32 * 16.0;
            if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_SCROLL: applied vscroll={} top_line={} visible={}",
                    vscroll, new, visible
                );
            }
        }

        if hscroll != 0.0 {
            let current = meta.editor_horizontal_offset_px.unwrap_or(0.0);
            let new = (current + hscroll).max(0.0);
            meta.editor_horizontal_offset_px = Some(new);
            if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref() == Ok("1") {
                eprintln!("ZAROXI_SCROLL: applied hscroll={:.1} offset={:.1}", hscroll, new);
            }
        }
    }

    /// Reset vertical scroll state to the top of the document.
    /// Call this when a new file is opened or content is replaced so that
    /// stale scroll offsets from a previous document do not persist.
    pub fn reset_scroll_state(&mut self) {
        self.pending_scroll_lines = 0;
        self.pending_vscroll_px = 0.0;
        if let Some(ref mut meta) = self.metadata {
            meta.editor_scroll_top_line = 0;
            meta.editor_scroll_px = 0.0;
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

                if cleaned != combined
                    && let Some(first_normal_idx) = line.spans.iter().position(|s| {
                        matches!(s.kind, crate::view_adapter::InterfaceSpanKind::Normal)
                    })
                {
                    line.spans[first_normal_idx].text = cleaned.clone();
                    for sp in line.spans.iter_mut().skip(first_normal_idx + 1) {
                        if matches!(sp.kind, crate::view_adapter::InterfaceSpanKind::Normal) {
                            sp.text.clear();
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
        self.workspace_id
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
        if let Some(m) = self.metadata.as_mut()
            && let Some(pos) = m.opened_buffers.iter().position(|it| &it.buffer_id == buffer_id)
        {
            let was_active = m.active_buffer.as_ref().map(|b| b == buffer_id).unwrap_or(false);
            let bid_str = buffer_id.to_string();
            m.opened_buffers.remove(pos);
            m.opened_buffer_count = m.opened_buffers.len();
            self.pending_removed_buffer_ids.push(bid_str);
            // Also remove from directly-added tracking if present.
            self.direct_buffer_ids.retain(|b| b != buffer_id);

            if was_active {
                m.visible_window = None;
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
                    m.active_buffer_details =
                        Some(ActiveBufferDetails { buffer_id: new_buf, display, line_count: 0 });
                }
            } else {
                if let Some(ab) = &m.active_buffer
                    && !m.opened_buffers.iter().any(|it| &it.buffer_id == ab)
                {
                    m.active_buffer = None;
                    m.active_buffer_details = None;
                }
            }

            return true;
        }
        false
    }

    /// Register a buffer directly in the compositor's opened-buffer list,
    /// bypassing the workspace service.  Used for large files where the
    /// service is intentionally skipped to avoid loading the full file
    /// into InMemoryBufferStore.  A tab appears immediately.
    pub fn add_opened_buffer_direct(
        &mut self,
        buffer_id: crate::ports::BufferId,
        display: Option<String>,
    ) {
        // Track this buffer as directly-added so the refresh cycle
        // merges it back into opened_buffers after service rebuilds.
        // Compare by CANONICAL path, not raw id, so two `buf:`/plain
        // forms of the same file never both get tracked.
        if !self.direct_buffer_ids.iter().any(|b| super::same_opened_document(b, &buffer_id)) {
            self.direct_buffer_ids.push(buffer_id.clone());
        }
        let md = self.metadata.get_or_insert_with(|| DesktopMetadata {
            session_id: self.session_id.clone(),
            workspace_id: self.workspace_id,
            active_buffer: None,
            opened_buffer_count: 0,
            opened_buffers: Vec::new(),
            active_buffer_details: None,
            ai_projection: None,
            diagnostics_snapshot: None,
            visible_window: None,
            last_command_line: None,
            editor_viewport_line_count: None,
            editor_scroll_top_line: 0,
            editor_scroll_px: 0.0,
            last_synced_window_height: None,
            editor_horizontal_offset_px: None,
            refresh_reason: None,
        });
        // Deactivate all existing tabs so only the new one is active.
        for it in &mut md.opened_buffers {
            it.active = false;
        }
        // Hard invariant: opening an already-open file must ACTIVATE/REUSE
        // the existing entry, never append a duplicate.  A preview→pinned
        // promotion (or re-open of an open file) is a state transition on
        // the existing entry, not a second logical open.
        if let Some(existing) = md
            .opened_buffers
            .iter_mut()
            .find(|it| super::same_opened_document(&it.buffer_id, &buffer_id))
        {
            existing.active = true;
            if existing.display.is_none() {
                existing.display = display;
            }
        } else {
            md.opened_buffers.push(super::OpenedBufferItem {
                buffer_id: buffer_id.clone(),
                display,
                active: true,
            });
        }
        md.opened_buffer_count = md.opened_buffers.len();
        md.active_buffer = Some(buffer_id.clone());
        // Preserve the existing line_count when this is the SAME document that
        // is already active (e.g. preview→pin promotion or re-registration).
        // Resetting it to 0 would zero the scroll `max_scroll` input and kill
        // wheel scrolling after pinning — a same-document transition must
        // never weaken the scroll math. A genuinely new document has no prior
        // details and correctly starts at 0 until its backend reports the total.
        let preserved_line_count = md
            .active_buffer_details
            .as_ref()
            .filter(|d| super::same_opened_document(&d.buffer_id, &buffer_id))
            .map(|d| d.line_count)
            .unwrap_or(0);
        md.active_buffer_details = Some(ActiveBufferDetails {
            buffer_id,
            line_count: preserved_line_count,
            display: None,
        });
    }

    /// Activate an already-registered direct buffer (e.g. large-file tab
    /// click).  Only modifies the active flag; does not add duplicates
    /// or reset `active_buffer_details.line_count`.
    pub fn set_direct_buffer_active(&mut self, buffer_id: crate::ports::BufferId) {
        let Some(md) = self.metadata.as_mut() else {
            return;
        };
        // If this buffer is already the active one, do nothing.
        if md.active_buffer.as_ref() == Some(&buffer_id) {
            return;
        }
        // Only act on direct (non-service) buffers.
        if !self.direct_buffer_ids.iter().any(|b| b == &buffer_id) {
            return;
        }
        // Mark this buffer active in the opened list; deactivate others.
        for it in &mut md.opened_buffers {
            it.active = it.buffer_id == buffer_id;
        }
        md.active_buffer = Some(buffer_id);
    }

    /// Clear any active direct buffer so the service-reported active
    /// buffer wins on the next refresh.  Used when switching to a
    /// service-backed buffer from a direct (large-file) buffer.
    pub fn clear_direct_active(&mut self) {
        if let Some(ref mut md) = self.metadata
            && let Some(ref active) = md.active_buffer
            && self.direct_buffer_ids.contains(active)
        {
            md.active_buffer = None;
            for it in &mut md.opened_buffers {
                it.active = false;
            }
        }
    }

    pub fn set_status_message(&mut self, text: String) {
        if let Some(m) = self.metadata.as_mut() {
            m.last_command_line = Some(text);
        } else {
            self.metadata = Some(DesktopMetadata {
                session_id: self.session_id.clone(),
                workspace_id: self.workspace_id,
                active_buffer: None,
                opened_buffer_count: 0,
                opened_buffers: Vec::new(),
                active_buffer_details: None,
                ai_projection: None,
                diagnostics_snapshot: None,
                visible_window: None,
                last_command_line: Some(text),
                editor_viewport_line_count: None,
                editor_scroll_top_line: 0,
                editor_scroll_px: 0.0,
                last_synced_window_height: None,
                editor_horizontal_offset_px: None,
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

    pub fn latest_consistency_report(
        &self,
    ) -> crate::desktop::consistency::DesktopConsistencyReport {
        crate::desktop::consistency::latest_consistency_report(self)
    }

    pub fn load_or_refresh_explorer(&mut self) {
        if let Some(root) = self.workspace_root_path.clone()
            && self.maybe_explorer.is_none()
        {
            let mut explorer = WorkspaceExplorer::new();
            if explorer.load_workspace(&root).is_ok() {
                self.maybe_explorer = Some(explorer);
            }
        }
        self.refresh_cached_explorer_items();
    }

    pub fn explorer_item_count(&self) -> usize {
        self.cached_explorer_items.len()
    }

    pub fn get_explorer_item_at(&self, idx: usize) -> Option<&ExplorerItemView> {
        self.cached_explorer_items.get(idx)
    }

    pub fn refresh_cached_explorer_items(&mut self) {
        let opened_paths: HashSet<String> = self
            .metadata
            .as_ref()
            .map(|m| {
                m.opened_buffers
                    .iter()
                    .filter_map(|b| b.buffer_id.path())
                    .map(|p| p.to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();

        let active_path: Option<String> = self
            .metadata
            .as_ref()
            .and_then(|m| m.active_buffer.as_ref())
            .and_then(|b| b.path())
            .map(|p| p.to_string_lossy().to_string());

        let query = self.explorer_search_query.clone();

        let explorer = match self.maybe_explorer.as_mut() {
            Some(e) => e,
            None => {
                self.cached_explorer_items.clear();
                return;
            }
        };

        self.cached_explorer_items = if query.trim().is_empty() {
            explorer.visible_items(&opened_paths, active_path.as_deref())
        } else {
            // Full-tree search: reveal matches inside collapsed folders,
            // auto-expanding their ancestors (see `filtered_visible_items`).
            explorer.filtered_visible_items(&query, &opened_paths, active_path.as_deref())
        };
    }

    /// Set the explorer search query and immediately recompute the visible
    /// (filtered) item set so the new filter takes effect this frame.
    pub fn set_explorer_search_query(&mut self, query: String) {
        self.explorer_search_query = query;
        self.refresh_cached_explorer_items();
    }

    pub fn format_cached_explorer_items(&self) -> Option<Vec<String>> {
        if self.cached_explorer_items.is_empty() {
            return None;
        }

        let strings: Vec<String> = self
            .cached_explorer_items
            .iter()
            .map(|it| {
                let indent = "  ".repeat(it.depth);
                let glyph =
                    if it.is_dir { if it.expanded { "\u{25BC}" } else { "\u{25B6}" } } else { " " };
                let marker = if it.is_active { " *" } else { "" };
                format!(
                    "{}{}{} {}{}",
                    indent,
                    glyph,
                    if it.is_dir { "" } else { " " },
                    it.name,
                    marker
                )
            })
            .collect();

        Some(strings)
    }

    pub fn get_explorer_item_id_at(&self, idx: usize) -> Option<String> {
        self.cached_explorer_items.get(idx).map(|it| it.id.clone())
    }

    pub fn is_explorer_item_dir(&self, idx: usize) -> bool {
        self.cached_explorer_items.get(idx).map(|it| it.is_dir).unwrap_or(false)
    }
}

// CloseContext trait impl: enables close-flow action functions in
// zaroxi-application-workspace to operate on DesktopComposition.
use zaroxi_application_workspace::workspace_view::CloseContext;

impl CloseContext for DesktopComposition {
    fn latest_active_buffer_details(&self) -> Option<ActiveBufferDetails> {
        self.metadata.as_ref().and_then(|m| m.active_buffer_details.clone())
    }

    fn latest_opened_buffers_summary(&self) -> OpenedBuffersSummary {
        self.latest_opened_buffers_summary()
    }

    fn latest_pending_close(
        &self,
    ) -> Option<zaroxi_application_workspace::workspace_view::PendingClose> {
        crate::desktop::pending_close::latest_pending_close(self)
    }

    fn set_pending_close(
        &mut self,
        pending: zaroxi_application_workspace::workspace_view::PendingClose,
    ) {
        self.set_pending_close(pending);
    }

    fn clear_pending_close(&mut self) {
        self.clear_pending_close();
    }

    fn close_opened_buffer(&mut self, buffer_id: &crate::ports::BufferId) -> bool {
        self.close_opened_buffer(buffer_id)
    }

    fn clear_pending_removed_buffer_id(&mut self, buffer_id: &crate::ports::BufferId) {
        let bid_str = buffer_id.to_string();
        self.pending_removed_buffer_ids.retain(|b| b != &bid_str);
    }

    fn set_status_message(&mut self, message: String) {
        self.set_status_message(message);
    }

    fn set_close_result_status(&mut self, message: String) {
        self.set_close_result_status(message);
    }

    fn clear_close_result_status(&mut self) {
        self.clear_close_result_status();
    }

    fn perform_session_close(&mut self) {
        self.perform_session_close();
    }
}

use zaroxi_application_workspace::workspace_view::{CommandBarContext, RefreshContext};

impl CommandBarContext for DesktopComposition {
    fn open_command_bar(&mut self) {
        self.open_command_bar();
    }
    fn close_command_bar(&mut self) {
        self.close_command_bar();
    }
    fn latest_command_bar(&self) -> Option<CommandBarState> {
        self.latest_command_bar()
    }
    fn select_next_command(&mut self) {
        self.select_next_command();
    }
    fn select_prev_command(&mut self) {
        self.select_prev_command();
    }
}

impl RefreshContext for DesktopComposition {
    fn has_pending_refresh_reason(&self) -> bool {
        self.has_pending_refresh_reason()
    }
    fn set_pending_refresh_reason(&mut self, reason: RefreshReason) {
        self.set_pending_refresh_reason(reason);
    }
    fn active_buffer(&self) -> Option<crate::ports::BufferId> {
        self.metadata.as_ref().and_then(|m| m.active_buffer.clone())
    }
    fn latest_shell_context(&self) -> Option<ShellContext> {
        self.latest_shell_context()
    }
    fn perform_refresh(
        &mut self,
        view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
        session_id: crate::ports::SessionId,
        workspace_id: Option<zaroxi_kernel_types::Id>,
        service: Option<std::sync::Arc<dyn crate::ports::WorkspaceService>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(self.refresh_with_service(view, session_id, workspace_id, service))
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;
    use crate::ports::BufferId;

    fn item(id: &str, display: &str, active: bool) -> OpenedBufferItem {
        OpenedBufferItem {
            buffer_id: BufferId(id.to_string()),
            display: Some(display.to_string()),
            active,
        }
    }

    #[test]
    fn canonical_path_strips_transport_prefix() {
        assert_eq!(canonical_buffer_path(&BufferId("buf:/x/y.rs".into())), "/x/y.rs");
        assert_eq!(canonical_buffer_path(&BufferId("/x/y.rs".into())), "/x/y.rs");
    }

    #[test]
    fn same_document_ignores_prefix_form() {
        let a = BufferId("buf:/x/y.rs".into());
        let b = BufferId("/x/y.rs".into());
        let c = BufferId("buf:/x/z.rs".into());
        assert!(same_opened_document(&a, &b));
        assert!(!same_opened_document(&a, &c));
    }

    #[test]
    fn dedupe_collapses_same_path_across_id_forms_and_keeps_active() {
        // Same file, two id forms (service `buf:` + direct plain) — the
        // exact bug that produced a duplicate tab.  After dedupe only one
        // entry survives, and the active flag is preserved.
        let mut list = vec![
            item("buf:/x/y.rs", "y.rs", false),
            item("/x/y.rs", "y.rs", true),
            item("buf:/x/z.rs", "z.rs", false),
        ];
        dedupe_opened_buffers(&mut list);
        assert_eq!(list.len(), 2, "one canonical path == one entry");
        let y = list.iter().find(|i| canonical_buffer_path(&i.buffer_id) == "/x/y.rs").unwrap();
        assert!(y.active, "active flag must survive dedupe");
    }

    #[test]
    fn dedupe_noop_when_unique() {
        let mut list = vec![item("buf:/a.rs", "a.rs", true), item("buf:/b.rs", "b.rs", false)];
        dedupe_opened_buffers(&mut list);
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn add_direct_preserves_line_count_for_same_document() {
        // Regression: preview→pin of the SAME document must not zero the
        // scroll `line_count`. Re-registering an already-active direct buffer
        // preserves the previously reported total (else wheel scroll dies).
        let mut comp = DesktopComposition::new();
        let bid = BufferId("buf:/big.log".into());
        comp.add_opened_buffer_direct(bid.clone(), Some("big.log".into()));
        // Simulate the backend reporting the real total after mapping.
        comp.metadata.as_mut().unwrap().active_buffer_details.as_mut().unwrap().line_count = 5000;
        // Re-register the SAME document (promotion path).
        comp.add_opened_buffer_direct(bid.clone(), Some("big.log".into()));
        assert_eq!(
            comp.metadata.as_ref().unwrap().active_buffer_details.as_ref().unwrap().line_count,
            5000,
            "same-document re-registration must preserve line_count",
        );
        // No duplicate opened_buffers entry was created.
        assert_eq!(comp.metadata.as_ref().unwrap().opened_buffers.len(), 1);
    }

    #[test]
    fn add_direct_new_document_starts_zero() {
        let mut comp = DesktopComposition::new();
        comp.add_opened_buffer_direct(BufferId("buf:/a.rs".into()), Some("a.rs".into()));
        comp.metadata.as_mut().unwrap().active_buffer_details.as_mut().unwrap().line_count = 10;
        // A genuinely different document has no prior details → starts at 0.
        comp.add_opened_buffer_direct(BufferId("buf:/b.rs".into()), Some("b.rs".into()));
        assert_eq!(
            comp.metadata.as_ref().unwrap().active_buffer_details.as_ref().unwrap().line_count,
            0,
        );
    }
}
