/// Adapter that assembles a `ShellWorkContent` snapshot from desktop DTOs.
///
/// The `ShellWorkContent` struct lives in Core (`zaroxi-core-engine-ui`).
/// This module is the thin desktop adapter that maps desktop-owned DTOs
/// (OpenedBuffersSummary, ActiveDocumentSummary, ShellContext, etc.) into
/// the engine-owned content model so the interface layer only places and
/// renders already-assembled content.
use zaroxi_core_engine_ui::ShellWorkContent;

use super::DesktopComposition;

impl DesktopComposition {
    pub fn build_work_content(&self) -> ShellWorkContent {
        let opened = self.latest_opened_buffers_summary();
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

        let editor_breadcrumb = self.latest_shell_context().and_then(|ctx| ctx.active_display);

        let editor_body = self.latest_active_document_summary().map(|doc| {
            let title = doc.display.unwrap_or_else(|| "untitled".to_string());
            let subtitle = doc.buffer_id.map(|b| b.to_string()).unwrap_or_default();
            let lines: Vec<String> = self
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
}
