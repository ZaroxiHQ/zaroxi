/*!
ShellRenderPresenter — tiny, non-visual, debug-only presenter.

Architectural rationale (short):
- Presenter lives in the `zaroxi-interface-desktop` crate (interface layer).
- It consumes the UI-facing ShellRenderViewModel (from the application crate)
  and may incorporate engine debug text produced by `render_debug_text`.
- Output is intentionally debug-only: a String combining high-level view-model
  semantics and the optional engine transcript for easier logs and harness use.
- Keeps engine/app layers UI-ignorant (no geometry, no widgets, no graphics APIs).

Public API:
- type: ShellRenderPresenter
- fn new() -> Self
- fn present(&self, &ShellRenderViewModel, Option<&str>) -> String
- fn present_with_plan(&self, &ShellRenderViewModel, &zaroxi_core_engine_render::ShellDrawPlan) -> String

Output type: String (multi-line, debug-only).

This module is intentionally small and deterministic so it can be used in tests
and harness logs without pulling in real UI toolkits.
*/

use crate::render_debug_text::render_debug_text;
use zaroxi_interface_app::ShellRenderViewModel;

/// Tiny, stateless presenter producing a debug-only String representation.
#[derive(Debug, Clone, Copy)]
pub struct ShellRenderPresenter;

impl ShellRenderPresenter {
    /// Create a new presenter instance.
    pub fn new() -> Self {
        Self
    }

    /// Present a ShellRenderViewModel combined with optional engine debug text.
    ///
    /// The returned String is multi-line and intended for logs or test harnesses.
    pub fn present(&self, vm: &ShellRenderViewModel, engine_debug: Option<&str>) -> String {
        let mut out = Vec::new();
        out.push(format!("ShellRenderViewModel: {} section(s)", vm.sections.len()));
        for (i, s) in vm.sections.iter().enumerate() {
            out.push(format!("  Section[{}] id=\"{}\" present={}", i, s.id, s.present));
            if s.present {
                for line in &s.lines {
                    out.push(format!("    {}", line));
                }
            }
        }

        if let Some(debug) = engine_debug {
            out.push("Engine debug:".to_string());
            for line in debug.lines() {
                out.push(format!("  {}", line));
            }
        }

        out.join("\n")
    }

    /// Convenience: render using an actual ShellDrawPlan by invoking the existing
    /// `render_debug_text` adapter and combining the result with the view model.
    pub fn present_with_plan(
        &self,
        vm: &ShellRenderViewModel,
        plan: &zaroxi_core_engine_render::ShellDrawPlan,
    ) -> String {
        let debug = render_debug_text(plan);
        self.present(vm, Some(&debug))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaroxi_interface_app::shell_render_view::SectionView;

    #[test]
    fn presenter_formats_viewmodel_and_debug_text() {
        let vm = ShellRenderViewModel {
            sections: vec![
                SectionView {
                    id: "debug".to_string(),
                    present: true,
                    lines: vec!["one".to_string(), "two".to_string()],
                },
            ],
        };

        let presenter = ShellRenderPresenter::new();
        let rendered = presenter.present(&vm, Some("render debug text:\n  plan-line"));

        assert!(rendered.contains("ShellRenderViewModel: 1 section(s)"));
        assert!(rendered.contains("Section[0] id=\"debug\""));
        assert!(rendered.contains("one"));
        assert!(rendered.contains("Engine debug:"));
        assert!(rendered.contains("plan-line"));
    }
}
