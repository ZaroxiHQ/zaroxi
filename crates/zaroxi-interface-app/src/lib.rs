#![doc = "Application-facing facade for small, pure view models exposed to the runtime.\n\nThis crate intentionally avoids pulling in unrelated workspace crates while the\ninterface layer is migrated in phases. For Phase 48 we only expose the\nShellFrameViewModel wrapper which owns the desktop ShellFrameModel."]

pub mod shell_frame;
pub use shell_frame::ShellFrameViewModel;

// Simple desktop-oriented smoke tests to validate the tiny app-facing view models.
// These tests do not pull in desktop presenter internals; they ensure the app-facing
// models remain stable for the presenter/desktop harness to consume.
#[cfg(test)]
mod tests {
    use super::ShellRenderViewModel;

    #[test]
    fn shell_render_view_model_basic() {
        let vm = ShellRenderViewModel {
            sections: vec![crate::shell_render_view::SectionView {
                id: "content".to_string(),
                present: true,
                lines: vec!["line1".to_string()],
            }],
        };
        assert_eq!(vm.sections.len(), 1);
        assert!(vm.sections[0].present);
        assert_eq!(vm.sections[0].lines[0], "line1");
    }
}

// Tiny UI-facing semantic view model for renderer outputs.
//
// See `crate::shell_render_view` for details. This module intentionally stays
// minimal and non-visual: it carries only section ids, presence markers, order,
// and simple textual lines (no geometry, colors, or layout).
pub mod shell_render_view;
pub use shell_render_view::ShellRenderViewModel;
