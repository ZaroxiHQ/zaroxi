/// Lightweight content snapshot carried by `ShellFrame` so the GPU draw
/// path can render live session data without depending on DesktopComposition.
///
/// Populated from `DesktopComposition` before each render. `None` means
/// no live session data is available; draw functions fall back to placeholders.
#[derive(Debug, Clone, Default)]
pub struct ShellWorkContent {
    pub editor_body: Option<zaroxi_core_engine_ui::ContentView>,
    pub editor_tabs: Option<Vec<String>>,
    pub editor_breadcrumb: Option<String>,
    pub explorer_items: Option<Vec<String>>,
    pub active_file: Option<String>,
    pub terminal_tabs: Option<Vec<String>>,
}

impl ShellWorkContent {
    pub fn from_composition(comp: &crate::desktop::DesktopComposition) -> Self {
        let opened = comp.latest_opened_buffers_summary();
        let active_id = opened.active.clone();

        // Explorer tree: opened buffers as file entries.
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

        // Editor tabs from opened buffers.
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

        // Editor breadcrumb from shell context.
        let editor_breadcrumb = comp.latest_shell_context().and_then(|ctx| ctx.active_display);

        // Editor body content from active document + visible window.
        let editor_body = comp.latest_active_document_summary().map(|doc| {
            let title = doc.display.unwrap_or_else(|| "untitled".to_string());
            let subtitle = doc.buffer_id.map(|b| b.to_string()).unwrap_or_default();
            let lines: Vec<String> = comp
                .latest_metadata()
                .and_then(|md| md.visible_window)
                .map(|vw| vw.lines.clone())
                .unwrap_or_else(|| {
                    doc.current_line_snippet.iter().map(|s| s.to_string()).collect()
                });
            let mut cv = zaroxi_core_engine_ui::ContentView::new(&title, &subtitle, lines);
            if cv.lines.is_empty() {
                cv = zaroxi_core_engine_ui::ContentView::default();
            }
            cv
        });

        // Terminal tabs.
        let terminal_tabs =
            Some(vec!["Terminal".to_string(), "Problems".to_string(), "Output".to_string()]);

        Self {
            editor_body,
            editor_tabs,
            editor_breadcrumb,
            explorer_items,
            active_file: active_id.clone().map(|b| b.to_string()),
            terminal_tabs,
        }
    }
}
