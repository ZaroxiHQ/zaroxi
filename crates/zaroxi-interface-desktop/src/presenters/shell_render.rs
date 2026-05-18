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
- fn present_with_plan(&self, &zaroxi_core_engine_render::ShellDrawPlan) -> String
- fn render_terminal(vm: &ShellRenderViewModel) -> String

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
    /// This presenter aims to faithfully surface semantic section ids, presence,
    /// stable ordering and simple line counts so harness output reads like real
    /// shell sections (non-visual, debug-only).
    pub fn present(&self, vm: &ShellRenderViewModel, engine_debug: Option<&str>) -> String {
        let mut out = Vec::new();
        out.push(format!("ShellRenderViewModel: {} section(s)", vm.sections.len()));

        // Compact, layout-oriented list of sections (top-to-bottom) with presence and optional line counts.
        out.push("Layout sections (top-to-bottom):".to_string());
        for s in &vm.sections {
            if s.present {
                out.push(format!("  - {}: present ({} lines)", s.id, s.lines.len()));
            } else {
                out.push(format!("  - {}: absent", s.id));
            }
        }

        // Detailed per-section dump (still debug-only, non-visual).
        for (i, s) in vm.sections.iter().enumerate() {
            if s.present {
                out.push(format!(
                    "  Section[{}] id=\"{}\" present=true lines={}",
                    i,
                    s.id,
                    s.lines.len()
                ));
                out.push(format!("  >> content ({} lines):", s.lines.len()));
                for line in &s.lines {
                    out.push(format!("    {}", line));
                }
            } else {
                out.push(format!("  Section[{}] id=\"{}\" present=false", i, s.id));
                out.push("  >> <absent>".to_string());
            }
        }

        // Engine debug text is secondary: include it for diagnostics but keep it clearly separated.
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

/// Render a terminal-like plaintext representation of the ShellRenderViewModel.
///
/// This function is intentionally tiny and non-graphical. It produces a
/// deterministic, plaintext layout suitable for debug logs or harness output.
/// It first emits a compact "Layout sections (top-to-bottom):" block, then
/// prints each section's content lines in order. No styling or interactivity.
pub fn render_terminal(vm: &ShellRenderViewModel) -> String {
    let mut out = Vec::new();

    // Layout summary (compact)
    out.push("Layout sections (top-to-bottom):".to_string());
    for s in &vm.sections {
        if s.present {
            out.push(format!("  - {}: present ({} lines)", s.id, s.lines.len()));
        } else {
            out.push(format!("  - {}: absent", s.id));
        }
    }

    // Then print section contents in the same ordering (plain text lines).
    // Use a simple header per section to keep output stable and readable.
    for s in &vm.sections {
        out.push(format!("Section[{}] content ({} lines):", s.id, s.lines.len()));
        if s.present {
            for line in &s.lines {
                out.push(line.clone());
            }
        } else {
            out.push("<absent>".to_string());
        }
    }

    out.join("\n")
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
                    id: "main".to_string(),
                    present: true,
                    lines: vec!["RenderSection { id: \"main\", ... }".to_string()],
                },
            ],
        };

        let presenter = ShellRenderPresenter::new();
        let rendered = presenter.present(&vm, Some("render debug text:\n  plan-line"));

        assert!(rendered.contains("ShellRenderViewModel: 1 section(s)"));
        assert!(rendered.contains("Layout sections (top-to-bottom):"));
        assert!(rendered.contains("main: present (1 lines)"));
        assert!(rendered.contains("Section[0] id=\"main\""));
        assert!(rendered.contains("RenderSection"));
        assert!(rendered.contains("Engine debug:"));
        assert!(rendered.contains("plan-line"));
    }

    #[test]
    fn render_terminal_outputs_layout_and_content() {
        let vm = ShellRenderViewModel {
            sections: vec![
                SectionView {
                    id: "content".to_string(),
                    present: true,
                    lines: vec!["hello from content".to_string()],
                },
            ],
        };

        let out = render_terminal(&vm);

        assert!(out.contains("Layout sections (top-to-bottom):"));
        assert!(out.contains("  - content: present (1 lines)"));
        assert!(out.contains("hello from content"));
    }
}
