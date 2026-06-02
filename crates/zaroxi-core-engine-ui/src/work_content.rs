use crate::ContentView;

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
    pub active_file: Option<String>,
    pub terminal_tabs: Option<Vec<String>>,
    /// AI assistant panel content view — built from the current AI projection
    /// in `DesktopComposition::build_work_content()`.
    pub ai_panel_content: Option<ContentView>,
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
            active_file,
            terminal_tabs,
            ai_panel_content: None,
        }
    }
}
