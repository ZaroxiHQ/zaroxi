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

// ── Pure policy functions (moved from interface-desktop) ───────────

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
