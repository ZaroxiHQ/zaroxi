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
    /// When set and `explorer_items` is empty/None, the sidebar renders a
    /// button with this label instead of an empty-state message.
    pub explorer_empty_button: Option<String>,
    pub active_file: Option<String>,
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
            explorer_empty_button: None,
            active_file,
            terminal_tabs,
            ai_panel_content: None,
            syntax_highlights: None,
        }
    }
}
