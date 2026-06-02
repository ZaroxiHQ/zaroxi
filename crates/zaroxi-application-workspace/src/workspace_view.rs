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
