/// Thin desktop adapter: gathers DTOs from `DesktopComposition` accessors
/// and delegates to the shared `build_work_content()` in `zaroxi-application-workspace`.
/// AI panel content is computed inline from the current `ai_projection`.
use super::DesktopComposition;
use zaroxi_application_ai::panel;
use zaroxi_application_workspace::workspace_view::build_work_content;
use zaroxi_core_engine_ui::ShellWorkContent;

impl DesktopComposition {
    pub fn build_work_content(&self) -> ShellWorkContent {
        let opened = self.latest_opened_buffers_summary();
        let doc = self.latest_active_document_summary();
        let ctx = self.latest_shell_context();
        let visible_window = self.latest_metadata().and_then(|md| md.visible_window);

        let ai_panel = self.latest_metadata().and_then(|md| md.ai_projection.clone()).map(|proj| {
            let target = proj
                .target_buffer
                .as_ref()
                .map(|b| b.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            if let Some(proposal_text) = &proj.proposal_text {
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

        build_work_content(&opened, doc.as_ref(), ctx.as_ref(), visible_window.as_ref(), ai_panel)
    }
}
