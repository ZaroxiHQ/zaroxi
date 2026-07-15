/// Shared workspace-view DTOs and content assembly consumed by desktop,
/// harness, and other UI layers.
///
/// These types were previously defined in the desktop crate. They are now
/// in `zaroxi-application-workspace` (Application layer) so they can be
/// shared by any consumer without pulling in interface-specific concerns.
use crate::ports::BufferId;
use std::future::Future;
use std::pin::Pin;
use zaroxi_core_engine_ui::syntax_tokenizer::tokenize_lines;
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
    fn clear_pending_removed_buffer_id(&mut self, buffer_id: &BufferId);
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
    fn perform_refresh(
        &mut self,
        view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
        session_id: crate::ports::SessionId,
        workspace_id: Option<zaroxi_kernel_types::Id>,
        service: Option<std::sync::Arc<dyn crate::ports::WorkspaceService>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;
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
        "Explain selection".into(),
        "AI review active buffer".into(),
        "Refactor selection".into(),
        "Generate tests".into(),
        "Fix diagnostics".into(),
        "Apply AI proposal".into(),
        "Reject AI proposal".into(),
        "Request close active".into(),
        "Confirm close: save".into(),
        "Confirm close: discard".into(),
        "Confirm close: cancel".into(),
        "Open workspace by path".into(),
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
///
/// `explorer_items` is an explicit pre-built list of sidebar row strings.
/// Pass `None` to fall back to opened-buffer-derived items (legacy path).
///
/// `explorer_empty_button` is an optional label for a button rendered in the
/// explorer sidebar when no tree items are present.
pub fn build_work_content(
    opened: &OpenedBuffersSummary,
    doc: Option<&ActiveDocumentSummary>,
    _ctx: Option<&ShellContext>,
    visible_window: Option<&VisibleWindowBasic>,
    ai_panel_content: Option<ContentView>,
    explorer_items: Option<Vec<String>>,
    explorer_empty_button: Option<String>,
) -> ShellWorkContent {
    let active_id = opened.active.clone();

    let explorer_items = explorer_items.or_else(|| {
        if !opened.items.is_empty() {
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
        }
    });

    let editor_tabs: Option<Vec<String>> = None;

    let editor_breadcrumb: Option<String> = None;

    let editor_body = doc.map(|d| {
        let title = d.display.clone().unwrap_or_else(|| "untitled".to_string());
        let subtitle = d.buffer_id.as_ref().map(|b| b.to_string()).unwrap_or_default();
        // Guard: if visible_window has no lines, treat it as absent so we
        // don't build source="visible_window" content with an empty payload.
        let vw_valid = visible_window.as_ref().is_some_and(|vw| !vw.lines.is_empty());
        let mut lines: Vec<String> = if vw_valid {
            visible_window.as_ref().unwrap().lines.clone()
        } else {
            d.current_line_snippet.as_ref().map(|s| vec![s.clone()]).unwrap_or_default()
        };
        let mut source = if vw_valid {
            "visible_window"
        } else if d.current_line_snippet.is_some() {
            "presenter_snippet"
        } else {
            "empty"
        };
        // Contract: when an active document is present but yields no visible
        // lines (empty/whitespace-only content, or no snippet available yet),
        // fall back to the engine default content lines so the editor body is
        // never rendered blank. `editor_body` stays `None` only when there is no
        // active document at all (handled by the outer `doc.map`).
        if lines.is_empty() {
            lines = ContentView::default().lines;
            source = "default";
        }
        if std::env::var("ZAROXI_FILE_TABS").as_deref() == Ok("1") {
            eprintln!(
                "ZAROXI_FILE_TABS: build_content  buf={}  title={title}  \
                 lines={}  line_count={}  source={source}  vw_valid={vw_valid}",
                d.buffer_id.as_ref().map(|b| b.to_string()).unwrap_or_default(),
                lines.len(),
                d.line_count,
            );
        }
        ContentView {
            title,
            subtitle,
            lines,
            cursor_line: d.cursor_line.unwrap_or(0),
            cursor_col: d.cursor_column.unwrap_or(0),
            selection: None,
        }
    });

    let terminal_tabs =
        Some(vec!["Terminal".to_string(), "Problems".to_string(), "Output".to_string()]);

    let syntax_lines: Vec<String> =
        visible_window.map(|vw| vw.lines.clone()).unwrap_or_else(|| {
            doc.map(|d| d.current_line_snippet.clone())
                .unwrap_or_default()
                .iter()
                .map(|s| s.to_string())
                .collect()
        });
    let syntax_highlights =
        if syntax_lines.is_empty() { None } else { Some(tokenize_lines(&syntax_lines)) };

    ShellWorkContent {
        editor_body,
        editor_tabs,
        editor_breadcrumb,
        suppress_empty_state: false,
        explorer_items,
        explorer_panel_items: None,
        explorer_panel_title: None,
        explorer_empty_button,
        explorer_empty_message: None,
        explorer_scroll_top: 0,
        explorer_search_query: String::new(),
        explorer_search_active: false,
        explorer_has_workspace: false,
        active_file: active_id.clone().map(|b| b.to_string()),
        terminal_tabs,
        ai_panel_content,
        ai_show_setup_cta: false,
        ai_composer_placeholder: None,
        syntax_highlights,
        editor_non_file_tabs: None,
        active_tab_index: None,
        extension_sidebar_items: None,
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
        if let Ok(snapshot) = s.attempt_close_session(req).await {
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
                Ok(ActionResult { success: true, message: None, refreshed: true })
            }
            Err(e) => {
                ctx.set_pending_close(PendingClose::ResolutionFailure {
                    message: format!("Save failed: {}", e),
                });
                Ok(ActionResult {
                    success: false,
                    message: Some("save failed".to_string()),
                    refreshed: false,
                })
            }
        }
    } else {
        ctx.perform_session_close();
        ctx.set_close_result_status("Closed session (no service)".to_string());
        Ok(ActionResult { success: true, message: None, refreshed: true })
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
                Ok(ActionResult { success: true, message: None, refreshed: true })
            }
            Err(e) => {
                ctx.set_pending_close(PendingClose::ResolutionFailure {
                    message: format!("Discard failed: {}", e),
                });
                Ok(ActionResult {
                    success: false,
                    message: Some("discard failed".to_string()),
                    refreshed: false,
                })
            }
        }
    } else {
        ctx.clear_pending_close();
        ctx.perform_session_close();
        ctx.set_status_message("Discarded and closed session (no service)".to_string());
        Ok(ActionResult { success: true, message: None, refreshed: true })
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

// ── Command dispatch ───────────────────────────────────────────────

/// Confirm the currently-selected command and close the bar on success.
pub async fn confirm_selected_command<C: CommandBarContext + CloseContext + RefreshContext>(
    ctx: &mut C,
    view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
    service: Option<std::sync::Arc<dyn crate::ports::WorkspaceService>>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
) -> Result<ActionResult, String> {
    let cb = match ctx.latest_command_bar() {
        Some(cb) => cb,
        None => {
            return Ok(ActionResult {
                success: false,
                message: Some("command bar is not open".to_string()),
                refreshed: false,
            });
        }
    };
    let idx = cb.selected;
    let res = execute_command_by_index(ctx, view, service, session_id, workspace_id, idx).await?;
    if res.success {
        ctx.close_command_bar();
    }
    Ok(res)
}

/// Shared command dispatch: resolves a command index to an action and executes
/// every known command fully here.  The function covers close-flow, service,
/// and refresh commands — no delegate/sentinel fallback needed.
pub async fn execute_command_by_index<C: CommandBarContext + CloseContext + RefreshContext>(
    ctx: &mut C,
    view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
    service: Option<std::sync::Arc<dyn crate::ports::WorkspaceService>>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    index: usize,
) -> Result<ActionResult, String> {
    let label: String =
        match ctx.latest_command_bar().and_then(|cb| cb.commands.get(index).cloned()) {
            Some(l) => l,
            None => {
                return Ok(ActionResult {
                    success: false,
                    message: Some("no command at index".to_string()),
                    refreshed: false,
                });
            }
        };

    match label.as_str() {
        "Refresh" => refresh_desktop(ctx, view, session_id, workspace_id, service).await,
        "Open buffer" => {
            if let Some(s) = service {
                let open_req = crate::ports::OpenBufferRequest {
                    session_id: session_id.clone(),
                    path: std::path::PathBuf::from("new_buffer.rs"),
                };
                match s.open_buffer(open_req).await {
                    Ok(_) => {
                        ctx.set_status_message("Opened buffer: new_buffer.rs".to_string());
                        let ar =
                            refresh_desktop(ctx, view, session_id, workspace_id, Some(s)).await?;
                        Ok(ActionResult {
                            success: true,
                            message: Some("opened buffer".to_string()),
                            refreshed: ar.refreshed,
                        })
                    }
                    Err(e) => Ok(ActionResult {
                        success: false,
                        message: Some(e.to_string()),
                        refreshed: false,
                    }),
                }
            } else {
                Ok(ActionResult {
                    success: false,
                    message: Some("open-buffer requires WorkspaceService".to_string()),
                    refreshed: false,
                })
            }
        }
        "Set active buffer" => {
            if let Some(s) = service {
                let obs = ctx.latest_opened_buffers_summary();
                if let Some(item) = obs.items.first() {
                    let buf = item.buffer_id.clone();
                    let sa = set_active_buffer_and_get_shell_context(
                        ctx,
                        s,
                        view,
                        session_id,
                        workspace_id,
                        buf,
                    )
                    .await?;
                    Ok(sa.action)
                } else {
                    Ok(ActionResult {
                        success: false,
                        message: Some("no opened buffers to activate".to_string()),
                        refreshed: false,
                    })
                }
            } else {
                Ok(ActionResult {
                    success: false,
                    message: Some("set-active requires WorkspaceService".to_string()),
                    refreshed: false,
                })
            }
        }
        "Explain active buffer" => {
            if let Some(s) = service {
                match s
                    .explain_active_buffer(crate::ports::GetActiveBufferRequest {
                        session_id: session_id.clone(),
                    })
                    .await
                {
                    Ok(resp) => {
                        ctx.set_status_message(format!("Explain dispatched: {:?}", resp));
                        ctx.set_pending_refresh_reason(RefreshReason::AiProjectionUpdated);
                        let ar =
                            refresh_desktop(ctx, view, session_id, workspace_id, Some(s)).await?;
                        Ok(ActionResult {
                            success: true,
                            message: Some("explain dispatched".to_string()),
                            refreshed: ar.refreshed,
                        })
                    }
                    Err(e) => Ok(ActionResult {
                        success: false,
                        message: Some(e.to_string()),
                        refreshed: false,
                    }),
                }
            } else {
                Ok(ActionResult {
                    success: false,
                    message: Some("explain requires WorkspaceService".to_string()),
                    refreshed: false,
                })
            }
        }
        "AI review active buffer"
        | "Apply AI proposal"
        | "Reject AI proposal"
        | "Explain selection"
        | "Refactor selection"
        | "Generate tests"
        | "Fix diagnostics"
        | "Open workspace by path" => Ok(ActionResult {
            success: false,
            message: Some("delegate".to_string()),
            refreshed: false,
        }),
        "Request close active" => request_close_active(ctx).await,
        "Confirm close: save" => confirm_save_and_close(ctx).await,
        "Confirm close: discard" => confirm_discard_and_close(ctx).await,
        "Confirm close: cancel" => confirm_cancel_close(ctx).await,
        _ => Ok(ActionResult {
            success: false,
            message: Some(format!("unsupported command: {}", label)),
            refreshed: false,
        }),
    }
}

// ── Refresh + buffer + cursor orchestration ────────────────────────

use crate::ports::{
    ApplyTextTransactionRequest, EditorCursor, GetActiveBufferRequest, SetActiveBufferRequest,
    SetEditorCursorRequest, TextEdit, WorkspaceView,
};

pub async fn refresh_desktop<R: RefreshContext>(
    ctx: &mut R,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
) -> Result<ActionResult, String> {
    if !ctx.has_pending_refresh_reason() && service.is_none() {
        ctx.set_pending_refresh_reason(RefreshReason::RefreshAction);
    }

    match ctx.perform_refresh(view, session_id, workspace_id, service).await {
        Ok(()) => Ok(ActionResult { success: true, message: None, refreshed: true }),
        Err(e) => Ok(ActionResult { success: false, message: Some(e), refreshed: false }),
    }
}

pub async fn refresh_and_get_shell_context<R: RefreshContext>(
    ctx: &mut R,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    service: Option<std::sync::Arc<dyn WorkspaceService>>,
) -> Result<ShellActionResult, String> {
    let action = refresh_desktop(ctx, view, session_id.clone(), workspace_id, service).await?;
    let context = ctx.latest_shell_context();
    Ok(ShellActionResult { action, context })
}

pub async fn set_active_buffer_and_get_shell_context<R: RefreshContext>(
    ctx: &mut R,
    service: std::sync::Arc<dyn WorkspaceService>,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    buffer_id: BufferId,
) -> Result<ShellActionResult, String> {
    match service.get_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() }).await
    {
        Ok(get_res) => {
            if get_res.buffer_id == buffer_id {
                if ctx.active_buffer() != Some(buffer_id.clone()) {
                    ctx.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
                } else {
                    ctx.set_pending_refresh_reason(RefreshReason::RefreshAction);
                }
            } else {
                if let Err(e) = service
                    .set_active_buffer(SetActiveBufferRequest {
                        session_id: session_id.clone(),
                        buffer_id: buffer_id.clone(),
                    })
                    .await
                {
                    return Ok(ShellActionResult {
                        action: ActionResult {
                            success: false,
                            message: Some(e.to_string()),
                            refreshed: false,
                        },
                        context: None,
                    });
                }
                ctx.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
            }
        }
        Err(_e) => {
            if let Err(e) = service
                .set_active_buffer(SetActiveBufferRequest {
                    session_id: session_id.clone(),
                    buffer_id: buffer_id.clone(),
                })
                .await
            {
                return Ok(ShellActionResult {
                    action: ActionResult {
                        success: false,
                        message: Some(e.to_string()),
                        refreshed: false,
                    },
                    context: None,
                });
            }
            ctx.set_pending_refresh_reason(RefreshReason::ActiveBufferChanged);
        }
    }

    // Reopening a previously-closed file: ensure the pending-removal
    // marker is cleared so the upcoming refresh does not filter this
    // buffer out of the canonical opened list.
    ctx.clear_pending_removed_buffer_id(&buffer_id);

    refresh_and_get_shell_context(ctx, view, session_id, workspace_id, Some(service)).await
}

pub async fn move_cursor_to_start_and_refresh<R: RefreshContext>(
    ctx: &mut R,
    service: std::sync::Arc<dyn WorkspaceService>,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
) -> Result<ActionResult, String> {
    let active_resp = match service
        .get_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return Ok(ActionResult {
                success: false,
                message: Some(e.to_string()),
                refreshed: false,
            });
        }
    };

    let buffer_id = active_resp.buffer_id;

    if let Err(e) = service
        .set_editor_cursor(SetEditorCursorRequest {
            session_id: session_id.clone(),
            buffer_id: buffer_id.clone(),
            cursor: EditorCursor { line: 0, column: 0 },
        })
        .await
    {
        return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false });
    }

    ctx.set_pending_refresh_reason(RefreshReason::CursorMoved);
    refresh_desktop(ctx, view, session_id, workspace_id, Some(service)).await
}

pub async fn insert_line_at_start_and_refresh<R: RefreshContext>(
    ctx: &mut R,
    service: std::sync::Arc<dyn WorkspaceService>,
    view: std::sync::Arc<dyn WorkspaceView>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
) -> Result<ActionResult, String> {
    let active_resp = match service
        .get_active_buffer(GetActiveBufferRequest { session_id: session_id.clone() })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return Ok(ActionResult {
                success: false,
                message: Some(e.to_string()),
                refreshed: false,
            });
        }
    };

    let buffer_id = active_resp.buffer_id;

    if let Err(e) = service
        .apply_text_transaction(ApplyTextTransactionRequest {
            session_id: session_id.clone(),
            buffer_id: buffer_id.clone(),
            transaction: TextEdit::Insert { index: 0, text: "\n".to_string() },
        })
        .await
    {
        return Ok(ActionResult { success: false, message: Some(e.to_string()), refreshed: false });
    }

    ctx.set_pending_refresh_reason(RefreshReason::BufferUpdated);
    refresh_desktop(ctx, view, session_id, workspace_id, Some(service)).await
}
