use crate::ContentView;
use zaroxi_core_engine_scene::SpanKind;

/// A syntax highlight span within a single line (column range + kind).
#[derive(Debug, Clone)]
pub struct LineHighlight {
    pub start_col: usize,
    pub end_col: usize,
    pub kind: HighlightKind,
}

/// Highlight categories used for color resolution in the renderer.
///
/// Phase 39: Adds `From<HighlightKind>` impl mapping each variant to the
/// app-neutral `SpanKind` in `zaroxi-core-engine-scene`. This allows syntax
/// highlight data to flow through the engine extraction seam without
/// IDE-specific concepts leaking into the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    Comment,
    String,
    Keyword,
    Function,
    Type,
    Number,
    Constant,
    Variable,
    Operator,
    Attribute,
    Plain,
}

impl From<HighlightKind> for SpanKind {
    fn from(k: HighlightKind) -> Self {
        match k {
            HighlightKind::Comment => SpanKind::Comment,
            HighlightKind::String => SpanKind::String,
            HighlightKind::Keyword => SpanKind::Keyword,
            HighlightKind::Function => SpanKind::Function,
            HighlightKind::Type => SpanKind::Type,
            HighlightKind::Number => SpanKind::Number,
            HighlightKind::Constant => SpanKind::Constant,
            HighlightKind::Variable => SpanKind::Variable,
            HighlightKind::Operator => SpanKind::Operator,
            HighlightKind::Attribute => SpanKind::Attribute,
            HighlightKind::Plain => SpanKind::Plain,
        }
    }
}

/// Per-line syntax highlights for editor content.
#[derive(Debug, Clone, Default)]
pub struct SyntaxHighlights {
    pub highlights: Vec<Vec<LineHighlight>>,
}

/// A single explorer tree item for the sidebar panel.
///
/// Carries enough structure for the widget builder to render indentation,
/// directory/file glyphs, active/open markers, and click targets.
#[derive(Debug, Clone)]
pub struct ExplorerPanelItem {
    /// Stable identifier (path string) used for action dispatch.
    pub id: String,
    /// Display label for the row.
    pub label: String,
    /// Indentation level (0 = top-level sibling of root).
    pub depth: usize,
    /// Directory nodes are expandable; file nodes are activatable.
    pub is_dir: bool,
    /// Only meaningful for directories.
    pub expanded: bool,
    /// Highlight this row as the active buffer.
    pub is_active: bool,
}

/// Lightweight workspace content snapshot carried by `ShellFrame` so the GPU
/// draw path can render live session data without depending on DesktopComposition.
///
/// Populated by `DesktopComposition::build_work_content()` before each render.
/// `None` fields mean no live session data is available; draw functions fall
/// back to placeholders.
///
/// This struct lives in `zaroxi-core-engine-ui` (Core layer) because it carries
/// only engine-owned content types (`ContentView`) and `String`/`Vec<String>`
/// primitives. The builder logic (`DesktopComposition::build_work_content`) stays
/// in `zaroxi-interface-desktop` where the desktop DTOs live.
#[derive(Debug, Clone, Default)]
pub struct ShellWorkContent {
    pub editor_body: Option<ContentView>,
    pub editor_tabs: Option<Vec<String>>,
    pub editor_breadcrumb: Option<String>,
    pub explorer_items: Option<Vec<String>>,
    /// Structured explorer tree items for the widget builder (drive ListItem widgets).
    pub explorer_panel_items: Option<Vec<ExplorerPanelItem>>,
    /// Panel header title shown above the tree (None hides the header).
    pub explorer_panel_title: Option<String>,
    /// Primary action button label (e.g. "Open Workspace"). Shown when panel
    /// items are empty and no workspace is loaded.
    pub explorer_empty_button: Option<String>,
    /// Empty-state message shown when panel is empty without a primary action.
    pub explorer_empty_message: Option<String>,
    /// First visible explorer row (vertical scroll offset, in rows). Both the
    /// widget tree (hit targets) and the render blocks read this so scrolling
    /// stays consistent across the two consumers.
    pub explorer_scroll_top: usize,
    /// Current explorer search/filter query (empty = no filter). Drives the
    /// filtered `explorer_panel_items` set and the rendered search box text.
    pub explorer_search_query: String,
    /// Whether the explorer search box currently holds keyboard focus (drives
    /// its focus ring / placeholder treatment).
    pub explorer_search_active: bool,
    /// Whether a workspace is currently loaded. Drives whether the explorer
    /// renders its search box (even when a filter yields no matches).
    pub explorer_has_workspace: bool,
    pub active_file: Option<String>,
    /// Non-file editor tabs (Settings, Extension Details, etc.). Each entry
    /// is `(label, kind_index)` where kind_index routes activation.
    pub editor_non_file_tabs: Option<Vec<(String, usize)>>,
    /// Index of the currently active tab (0-based across file + non-file tabs).
    pub active_tab_index: Option<usize>,
    /// Extension sidebar items (replaces explorer when Some). Each entry is
    /// `(name, id)` — id is used for opening extension detail tabs.
    pub extension_sidebar_items: Option<Vec<(String, String)>>,
    pub terminal_tabs: Option<Vec<String>>,
    /// AI assistant panel content view — built from the current AI projection
    /// in `DesktopComposition::build_work_content()`.
    pub ai_panel_content: Option<ContentView>,
    /// Per-line syntax highlight spans for the editor content.
    pub syntax_highlights: Option<SyntaxHighlights>,
}

impl ShellWorkContent {
    pub fn new(
        editor_body: Option<ContentView>,
        editor_tabs: Option<Vec<String>>,
        editor_breadcrumb: Option<String>,
        explorer_items: Option<Vec<String>>,
        active_file: Option<String>,
        terminal_tabs: Option<Vec<String>>,
    ) -> Self {
        Self {
            editor_body,
            editor_tabs,
            editor_breadcrumb,
            explorer_items,
            explorer_panel_items: None,
            explorer_panel_title: None,
            explorer_empty_button: None,
            explorer_empty_message: None,
            explorer_scroll_top: 0,
            explorer_search_query: String::new(),
            explorer_search_active: false,
            explorer_has_workspace: false,
            active_file,
            terminal_tabs,
            ai_panel_content: None,
            syntax_highlights: None,
            editor_non_file_tabs: None,
            active_tab_index: Some(0),
            extension_sidebar_items: None,
        }
    }
}
