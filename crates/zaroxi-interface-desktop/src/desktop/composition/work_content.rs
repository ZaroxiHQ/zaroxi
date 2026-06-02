/// Thin desktop adapter: gathers DTOs from `DesktopComposition` accessors
/// and delegates to the shared `build_work_content()` in `zaroxi-application-workspace`.
use super::DesktopComposition;
use zaroxi_application_workspace::workspace_view::build_work_content;
use zaroxi_core_engine_ui::ShellWorkContent;

impl DesktopComposition {
    pub fn build_work_content(&self) -> ShellWorkContent {
        let opened = self.latest_opened_buffers_summary();
        let doc = self.latest_active_document_summary();
        let ctx = self.latest_shell_context();
        let visible_window = self.latest_metadata().and_then(|md| md.visible_window);

        build_work_content(&opened, doc.as_ref(), ctx.as_ref(), visible_window.as_ref())
    }
}
