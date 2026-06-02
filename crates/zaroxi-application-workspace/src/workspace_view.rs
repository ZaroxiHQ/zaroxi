/// Shared workspace-view DTOs and content assembly consumed by desktop,
/// harness, and other UI layers.
///
/// These types were previously defined in the desktop crate. They are now
/// in `zaroxi-application-workspace` (Application layer) so they can be
/// shared by any consumer without pulling in interface-specific concerns.
use crate::ports::BufferId;
use zaroxi_core_engine_ui::{ContentView, ShellWorkContent};

/// Small opened-buffer summary item exposed to interface layers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedBufferItemSummary {
    pub buffer_id: BufferId,
    pub display: Option<String>,
    pub line_count: usize,
    pub active: bool,
}

/// Aggregate opened-buffers summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedBuffersSummary {
    pub count: usize,
    pub items: Vec<OpenedBufferItemSummary>,
    pub active: Option<BufferId>,
}

/// Small active-document summary derived from presenter/workspace state.
#[derive(Clone, Debug)]
pub struct ActiveDocumentSummary {
    pub buffer_id: Option<BufferId>,
    pub display: Option<String>,
    pub line_count: usize,
    pub cursor_line: Option<usize>,
    pub cursor_column: Option<usize>,
    pub selection_present: bool,
    pub current_line_snippet: Option<String>,
}

/// Small active-buffer details from the composition metadata.
#[derive(Clone, Debug)]
pub struct ActiveBufferDetails {
    pub buffer_id: BufferId,
    pub display: Option<String>,
    pub line_count: usize,
}

/// Normalized action result returned by interface actions.
#[derive(Clone, Debug)]
pub struct ActionResult {
    pub success: bool,
    pub message: Option<String>,
    pub refreshed: bool,
}

/// Convenience result containing ActionResult plus latest ShellContext.
#[derive(Clone, Debug)]
pub struct ShellActionResult {
    pub action: ActionResult,
    pub context: Option<ShellContext>,
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

/// Minimal visible-window projection populated during refresh.
#[derive(Clone, Debug)]
pub struct VisibleWindowBasic {
    pub top_line: usize,
    pub total_lines: usize,
    pub lines: Vec<String>,
    pub cursor_line: Option<usize>,
    pub cursor_column: Option<usize>,
    pub selection_present: bool,
}

/// Lightweight shell-facing context derived from composition metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellContext {
    pub active_buffer: Option<BufferId>,
    pub active_display: Option<String>,
    pub latest_revision: u64,
    pub latest_refresh_reason: Option<RefreshReason>,
    pub has_ai_projection: bool,
    pub last_command_line: Option<String>,
}

/// Causes tracked by the composition refresh loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefreshReason {
    InitialLoad,
    RefreshAction,
    CursorMoved,
    BufferUpdated,
    ActiveBufferChanged,
    AiProjectionUpdated,
}

// ── Close-flow orchestration trait ──────────────────────────────────

/// Application-facing capability set needed by close-flow orchestration.
///
/// Desktop implements this on `DesktopComposition`. The close-flow action
/// functions in this module are generic over `C: CloseContext` so they can
/// live in the application layer without depending on interface types.
pub trait CloseContext {
    fn latest_active_buffer_details(&self) -> Option<ActiveBufferDetails>;
    fn latest_opened_buffers_summary(&self) -> OpenedBuffersSummary;
    fn latest_pending_close(&self) -> Option<PendingClose>;
    fn set_pending_close(&mut self, pending: PendingClose);
    fn clear_pending_close(&mut self);
    fn close_opened_buffer(&mut self, buffer_id: &BufferId) -> bool;
    fn set_status_message(&mut self, message: String);
    fn set_close_result_status(&mut self, message: String);
    fn clear_close_result_status(&mut self);
    fn perform_session_close(&mut self);
}

/// Application-facing capability set for refresh orchestration.
pub trait RefreshContext: CloseContext {
    fn has_pending_refresh_reason(&self) -> bool;
    fn set_pending_refresh_reason(&mut self, reason: RefreshReason);
    fn active_buffer(&self) -> Option<BufferId>;
    fn latest_shell_context(&self) -> Option<ShellContext>;
}

/// Application-facing capability set for command-bar UI state.
pub trait CommandBarContext {
    fn open_command_bar(&mut self);
    fn close_command_bar(&mut self);
    fn latest_command_bar(&self) -> Option<CommandBarState>;
    fn select_next_command(&mut self);
    fn select_prev_command(&mut self);
}

/// Command-bar UI state (moved from desktop).
#[derive(Clone, Debug)]
pub struct CommandBarState {
    pub open: bool,
    pub commands: Vec<String>,
    pub selected: usize,
    pub staged_arg: Option<String>,
}

/// Small, single-source-of-truth model describing an in-progress close
/// resolution flow that the UI will present to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingClose {
    BufferClose { buffer_id: BufferId, display: Option<String>, dirty: bool },
    SessionClose { dirty_buffers: Vec<BufferId>, summary: String },
    ResolutionFailure { message: String },
}

impl PendingClose {
    /// Render a compact single-line summary suitable for shell banners/tests.
    pub fn render_summary(&self) -> String {
        match self {
            PendingClose::BufferClose { display, dirty, .. } => {
                if *dirty {
                    format!(
                        "Close buffer '{}' (unsaved changes)",
                        display.clone().unwrap_or_else(|| "<unnamed>".to_string())
                    )
                } else {
                    format!(
                        "Close buffer '{}'",
                        display.clone().unwrap_or_else(|| "<unnamed>".to_string())
                    )
                }
            }
            PendingClose::SessionClose { summary, .. } => {
                format!("Close session: {}", summary)
            }
            PendingClose::ResolutionFailure { message } => {
                format!("Close failed: {}", message)
            }
        }
    }

    /// Human-friendly buffer label from a BufferClose variant.
    /// Prefers display name, falls back to buffer path, then buffer id.
    pub fn close_buffer_label(buffer_id: &BufferId, display: &Option<String>) -> String {
        display
            .clone()
            .or_else(|| buffer_id.path().map(|p| p.to_string_lossy().to_string()))
            .unwrap_or_else(|| buffer_id.to_string())
    }
}

/// Deterministic human-readable label for a `RefreshReason`.
pub fn refresh_reason_label(reason: &RefreshReason) -> &'static str {
    match reason {
        RefreshReason::InitialLoad => "initial load",
        RefreshReason::RefreshAction => "refreshed",
        RefreshReason::CursorMoved => "cursor moved",
        RefreshReason::BufferUpdated => "buffer updated",
        RefreshReason::ActiveBufferChanged => "active buffer changed",
        RefreshReason::AiProjectionUpdated => "AI projection updated",
    }
}

/// The canonical set of command-bar command labels.
pub fn command_bar_labels() -> Vec<String> {
    vec![
        "Refresh".into(),
        "Open buffer".into(),
        "Set active buffer".into(),
        "Explain active buffer".into(),
        "Request close active".into(),
        "Confirm close: save".into(),
        "Confirm close: discard".into(),
        "Confirm close: cancel".into(),
    ]
}

/// Navigation: next command index with wrap-around.
pub fn select_next_command_index(current: usize, len: usize) -> usize {
    if len == 0 {
        return current;
    }
    (current + 1) % len
}

/// Navigation: previous command index with wrap-around.
pub fn select_prev_command_index(current: usize, len: usize) -> usize {
    if len == 0 {
        return current;
    }
    if current == 0 { len - 1 } else { current - 1 }
}

/// Assemble a `ShellWorkContent` snapshot from workspace-view DTOs.
///
/// This is the shared content-assembly function — any interface (desktop,
/// harness, CLI) can call it after gathering the required DTOs from their
/// own composition/session state. Desktop calls this via
/// `DesktopComposition::build_work_content()`.
pub fn build_work_content(
    opened: &OpenedBuffersSummary,
    doc: Option<&ActiveDocumentSummary>,
    ctx: Option<&ShellContext>,
    visible_window: Option<&VisibleWindowBasic>,
) -> ShellWorkContent {
    let active_id = opened.active.clone();

    let explorer_items = if !opened.items.is_empty() {
        Some(
            opened
                .items
                .iter()
                .map(|it| {
                    let disp = it.display.clone().unwrap_or_else(|| "untitled".to_string());
                    if Some(&it.buffer_id) == active_id.as_ref() {
                        format!("{} *", disp)
                    } else {
                        disp
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    let editor_tabs = if !opened.items.is_empty() {
        Some(
            opened
                .items
                .iter()
                .map(|it| it.display.clone().unwrap_or_else(|| "untitled".to_string()))
                .collect(),
        )
    } else {
        None
    };

    let editor_breadcrumb = ctx.and_then(|c| c.active_display.clone());

    let editor_body = doc.map(|d| {
        let title = d.display.clone().unwrap_or_else(|| "untitled".to_string());
        let subtitle = d.buffer_id.as_ref().map(|b| b.to_string()).unwrap_or_default();
        let lines: Vec<String> = visible_window
            .map(|vw| vw.lines.clone())
            .unwrap_or_else(|| d.current_line_snippet.iter().map(|s| s.to_string()).collect());
        let mut cv = ContentView::new(&title, &subtitle, lines);
        if cv.lines.is_empty() {
            cv = ContentView::default();
        }
        cv
    });

    let terminal_tabs =
        Some(vec!["Terminal".to_string(), "Problems".to_string(), "Output".to_string()]);

    ShellWorkContent {
        editor_body,
        editor_tabs,
        editor_breadcrumb,
        explorer_items,
        active_file: active_id.clone().map(|b| b.to_string()),
        terminal_tabs,
    }
}

// ── Close-flow action orchestration ────────────────────────────────

use crate::ports::{GetSessionSnapshotRequest, SaveCheckpointRequest, SessionId, WorkspaceService};
use std::sync::Arc;

pub async fn request_close_active<C: CloseContext>(ctx: &mut C) -> Result<ActionResult, String> {
    if let Some(details) = ctx.latest_active_buffer_details() {
        let pending = PendingClose::BufferClose {
            buffer_id: details.buffer_id.clone(),
            display: details.display.clone(),
            dirty: true,
        };
        ctx.set_pending_close(pending);
        Ok(ActionResult { success: true, message: None, refreshed: false })
    } else {
        Ok(ActionResult {
            success: false,
            message: Some("no active buffer".to_string()),
            refreshed: false,
        })
    }
}

pub async fn request_close_session<C: CloseContext>(
    ctx: &mut C,
    session_id: SessionId,
    service: Option<Arc<dyn WorkspaceService>>,
) -> Result<ActionResult, String> {
    if let Some(s) = service {
        let req = GetSessionSnapshotRequest { session_id: session_id.clone(), recent_limit: 0 };
        match s.attempt_close_session(req).await {
            Ok(snapshot) => {
                if snapshot.snapshot.opened_buffers.is_empty() {
                    ctx.perform_session_close();
                    return Ok(ActionResult { success: true, message: None, refreshed: true });
                } else {
                    let dirty_ids = snapshot.snapshot.opened_buffers.clone();
                    let summary = format!("{} buffers may have unsaved changes", dirty_ids.len());
                    let pending = PendingClose::SessionClose { dirty_buffers: dirty_ids, summary };
                    ctx.set_pending_close(pending);
                    return Ok(ActionResult { success: true, message: None, refreshed: false });
                }
            }
            Err(_) => {}
        }
    }

    let obs = ctx.latest_opened_buffers_summary();
    if obs.count == 0 {
        ctx.perform_session_close();
        Ok(ActionResult { success: true, message: None, refreshed: true })
    } else {
        let ids: Vec<BufferId> = obs.items.iter().map(|i| i.buffer_id.clone()).collect();
        let summary = format!("{} open buffers", ids.len());
        let pending = PendingClose::SessionClose { dirty_buffers: ids, summary };
        ctx.set_pending_close(pending);
        Ok(ActionResult { success: true, message: None, refreshed: false })
    }
}

pub async fn confirm_save_all_and_close<C: CloseContext>(
    ctx: &mut C,
    service: Option<Arc<dyn WorkspaceService>>,
    session_id: SessionId,
) -> Result<ActionResult, String> {
    if let Some(s) = service {
        let save_req = SaveCheckpointRequest { session_id: session_id.clone() };
        match s.save_checkpoint(save_req).await {
            Ok(_) => {
                ctx.perform_session_close();
                ctx.set_close_result_status("Saved and closed session".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            Err(e) => {
                ctx.set_pending_close(PendingClose::ResolutionFailure {
                    message: format!("Save failed: {}", e),
                });
                return Ok(ActionResult {
                    success: false,
                    message: Some("save failed".to_string()),
                    refreshed: false,
                });
            }
        }
    } else {
        ctx.perform_session_close();
        ctx.set_close_result_status("Closed session (no service)".to_string());
        return Ok(ActionResult { success: true, message: None, refreshed: true });
    }
}

pub async fn confirm_discard_all_and_close<C: CloseContext>(
    ctx: &mut C,
    service: Option<Arc<dyn WorkspaceService>>,
    session_id: SessionId,
) -> Result<ActionResult, String> {
    if let Some(s) = service {
        let req = SaveCheckpointRequest { session_id: session_id.clone() };
        match s.resolve_close_session_discard_all(req).await {
            Ok(_) => {
                ctx.perform_session_close();
                ctx.set_close_result_status("Discarded and closed session".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            Err(e) => {
                ctx.set_pending_close(PendingClose::ResolutionFailure {
                    message: format!("Discard failed: {}", e),
                });
                return Ok(ActionResult {
                    success: false,
                    message: Some("discard failed".to_string()),
                    refreshed: false,
                });
            }
        }
    } else {
        ctx.clear_pending_close();
        ctx.perform_session_close();
        ctx.set_status_message("Discarded and closed session (no service)".to_string());
        return Ok(ActionResult { success: true, message: None, refreshed: true });
    }
}

pub async fn confirm_save_and_close<C: CloseContext>(ctx: &mut C) -> Result<ActionResult, String> {
    if let Some(pc) = ctx.latest_pending_close() {
        match pc {
            PendingClose::BufferClose { buffer_id, display, .. } => {
                let label = PendingClose::close_buffer_label(&buffer_id, &display);
                let _removed = ctx.close_opened_buffer(&buffer_id);
                let id_str = format!("{}", buffer_id);
                ctx.set_close_result_status(format!("Saved and closed {} ({})", label, id_str));
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            _ => {
                ctx.set_close_result_status("Saved and closed".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
        }
    }

    ctx.set_status_message("Saved and closed".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: true })
}

pub async fn confirm_discard_and_close<C: CloseContext>(
    ctx: &mut C,
) -> Result<ActionResult, String> {
    if let Some(pc) = ctx.latest_pending_close() {
        match pc {
            PendingClose::BufferClose { buffer_id, display, .. } => {
                let label = PendingClose::close_buffer_label(&buffer_id, &display);
                let _removed = ctx.close_opened_buffer(&buffer_id);
                let id_str = format!("{}", buffer_id);
                ctx.set_close_result_status(format!(
                    "Discarded changes and closed {} ({})",
                    label, id_str
                ));
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
            _ => {
                ctx.set_close_result_status("Discarded and closed".to_string());
                return Ok(ActionResult { success: true, message: None, refreshed: true });
            }
        }
    }

    ctx.set_status_message("Discarded and closed".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: true })
}

pub async fn confirm_cancel_close<C: CloseContext>(ctx: &mut C) -> Result<ActionResult, String> {
    ctx.clear_close_result_status();
    ctx.clear_pending_close();
    ctx.set_status_message("Close cancelled".to_string());
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

// ── Command-bar orchestration ──────────────────────────────────────

pub async fn open_command_bar<C: CommandBarContext>(ctx: &mut C) -> Result<ActionResult, String> {
    ctx.open_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn close_command_bar<C: CommandBarContext>(ctx: &mut C) -> Result<ActionResult, String> {
    ctx.close_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn navigate_command_bar_next<C: CommandBarContext>(
    ctx: &mut C,
) -> Result<ActionResult, String> {
    ctx.select_next_command();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn navigate_command_bar_prev<C: CommandBarContext>(
    ctx: &mut C,
) -> Result<ActionResult, String> {
    ctx.select_prev_command();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}

pub async fn cancel_command_bar<C: CommandBarContext>(ctx: &mut C) -> Result<ActionResult, String> {
    ctx.close_command_bar();
    Ok(ActionResult { success: true, message: None, refreshed: false })
}
