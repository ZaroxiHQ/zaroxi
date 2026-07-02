/// Thin desktop adapter: gathers DTOs from `DesktopComposition` accessors
/// and delegates to the shared `build_work_content()` in `zaroxi-application-workspace`.
/// AI panel content is computed inline from the current `ai_projection`.
/// Diagnostics for the active buffer are merged into the AI panel content view.
/// Explorer panel content is built via the explorer_panel module.
use super::DesktopComposition;
use crate::gui::window::explorer_panel::ExplorerPanelViewModel;
use zaroxi_application_ai::panel;
use zaroxi_application_workspace::workspace_view::build_work_content;
use zaroxi_core_engine_ui::{ContentView, ShellWorkContent};

impl DesktopComposition {
    pub fn build_work_content(&self) -> ShellWorkContent {
        let opened = self.latest_opened_buffers_summary();
        let doc = self.latest_active_document_summary();
        let ctx = self.latest_shell_context();
        let visible_window = self.latest_metadata().and_then(|md| md.visible_window);

        let explorer_items = self.format_cached_explorer_items();

        // Build explorer panel data from the shared view model.
        let explorer_vm = ExplorerPanelViewModel::build(self);
        let explorer_panel_items =
            if explorer_vm.items.is_empty() { None } else { Some(explorer_vm.items.clone()) };
        let explorer_panel_title = if !explorer_vm.items.is_empty() {
            explorer_vm.title.clone().or(Some("PROJECT".to_string()))
        } else {
            None
        };
        let explorer_empty_button = explorer_vm.primary_action_label.clone();
        let explorer_empty_message = explorer_vm.empty_message.clone();

        let mut ai_panel =
            self.latest_metadata().and_then(|md| md.ai_projection.clone()).map(|proj| {
                let target = proj
                    .target_buffer
                    .as_ref()
                    .map(|b| b.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let is_applied =
                    proj.state.as_ref().is_some_and(|s| matches!(s, super::AiState::Applied));

                if is_applied && proj.result.is_some() {
                    panel::applied_content_view(proj.result.as_deref().unwrap_or(""), &target)
                } else if let Some(proposal_text) = &proj.proposal_text {
                    panel::proposal_content_view(
                        proposal_text,
                        &target,
                        proj.result.as_deref().unwrap_or(""),
                    )
                } else if let Some(result) = &proj.result {
                    panel::explain_content_view(result, &target)
                } else {
                    panel::idle_content_view()
                }
            });

        // Merge diagnostics into the AI panel when present.
        if let Some(diag) = self.latest_metadata().and_then(|md| md.diagnostics_snapshot.clone()) {
            let total = diag.errors + diag.warnings + diag.infos + diag.hints;
            if total > 0 {
                // Fetch individual diagnostic messages for the active buffer.
                let detail_lines: Vec<String> =
                    crate::diagnostics::diagnostics_details_for_uri(&diag.active_buffer)
                        .unwrap_or_default()
                        .iter()
                        .map(|d| {
                            if let Some(ln) = d.line {
                                format!("{} (line {}): {}", d.severity.as_str(), ln, d.message)
                            } else {
                                format!("{}: {}", d.severity.as_str(), d.message)
                            }
                        })
                        .collect();

                let diag_view = panel::diagnostics_content_view(
                    diag.errors,
                    diag.warnings,
                    diag.infos,
                    diag.hints,
                    &diag.active_buffer,
                    &detail_lines,
                );
                if let Some(ref mut ai) = ai_panel {
                    // Merge diagnostics lines into existing AI content.
                    let mut combined = ai.lines.clone();
                    combined.push(String::new());
                    combined.extend(diag_view.lines);
                    *ai = ContentView::new(&ai.title, &ai.subtitle, combined);
                } else {
                    ai_panel = Some(diag_view);
                }
            }
        }

        let mut shell_wc = build_work_content(
            &opened,
            doc.as_ref(),
            ctx.as_ref(),
            visible_window.as_ref(),
            ai_panel,
            explorer_items,
            explorer_empty_button,
        );

        shell_wc.explorer_panel_items = explorer_panel_items;
        shell_wc.explorer_panel_title = explorer_panel_title;
        shell_wc.explorer_empty_message = explorer_empty_message;
        shell_wc.explorer_search_query = self.explorer_search_query.clone();
        shell_wc.explorer_has_workspace = self.workspace_root_path.is_some();

        shell_wc
    }
}
