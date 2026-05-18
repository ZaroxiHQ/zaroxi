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

/// Small, structured, non-graphical render-tree types for shell sections.
///
/// These types live in the `zaroxi-interface-desktop` crate and are debug-only
/// helpers used by harnesses/tests to reason about section identity, ordering,
/// presence and textual lines. They purposely avoid any UI/graphics crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderSectionKind {
    Content,
    Status,
    AI,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSection {
    pub id: String,
    pub kind: RenderSectionKind,
    pub present: bool,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderTree {
    pub sections: Vec<RenderSection>,
}

fn kind_from_section_id(id: &str) -> RenderSectionKind {
    // Derive a small set of canonical kinds from common section ids, preserving
    // fidelity to real section names where possible.
    match id.to_ascii_lowercase().as_str() {
        "content" | "main" | "editor" => RenderSectionKind::Content,
        "status" | "statusbar" | "status_bar" => RenderSectionKind::Status,
        "ai" | "assistant" | "ai-assistant" => RenderSectionKind::AI,
        other => RenderSectionKind::Other(other.to_string()),
    }
}

/// Build a simple, top-to-bottom render tree from a ShellRenderViewModel.
///
/// - Preserves section ordering.
/// - Preserves presence/absence and the original textual lines.
/// - Classifies sections into small set of Kinds derived from the section id.
/// - This function intentionally does NOT compute geometry, layout boxes, or
///   emit any terminal/GUI drawing commands.
pub fn render_shell_sections(vm: &ShellRenderViewModel) -> RenderTree {
    let mut sections = Vec::with_capacity(vm.sections.len());
    for s in &vm.sections {
        let kind = kind_from_section_id(&s.id);
        sections.push(RenderSection {
            id: s.id.clone(),
            kind,
            present: s.present,
            lines: s.lines.clone(),
        });
    }
    RenderTree { sections }
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

    #[test]
    fn render_shell_sections_builds_structured_tree() {
        let vm = ShellRenderViewModel {
            sections: vec![
                SectionView {
                    id: "content".to_string(),
                    present: true,
                    lines: vec!["line1".to_string(), "line2".to_string()],
                },
            ],
        };

        let tree = render_shell_sections(&vm);

        // exactly one section preserved in ordering
        assert_eq!(tree.sections.len(), 1);

        let sec = &tree.sections[0];
        // kind derived from section id
        assert_eq!(sec.kind, RenderSectionKind::Content);
        // presence preserved
        assert!(sec.present);
        // lines preserved, in order
        assert_eq!(sec.lines, vec!["line1".to_string(), "line2".to_string()]);
        // id preserved
        assert_eq!(sec.id, "content".to_string());
    }
}

pub mod ui {
    //! Minimal UI-only mapper for terminal widgets.
    //!
    //! This module lives in the `zaroxi-interface-desktop` crate and provides a
    //! tiny, non-interactive mapping from the structured RenderTree produced by
    //! `render_shell_sections` into UI-like nodes (UiBlock/UiLine). It intentionally
    //! does not perform layout, styling, or event handling — only a faithful,
    //! top-to-bottom mapping of section identity, kind, presence and textual lines.
    //!
    //! The module is deliberately small so tests can assert mapping fidelity
    //! without pulling in any real UI toolkit.

    use super::*;

    /// UI-layer section kinds mirroring RenderSectionKind.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum UiSectionKind {
        Content,
        Status,
        AI,
        Other(String),
    }

    /// A single rendered text line in the UI tree.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UiLine(pub String);

    /// A block-like UI node representing a shell section.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UiBlock {
        pub id: String,
        pub kind: UiSectionKind,
        pub present: bool,
        pub lines: Vec<UiLine>,
    }

    /// Top-level UI tree containing blocks in top-to-bottom order.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UiTree {
        pub blocks: Vec<UiBlock>,
    }

    /// Map a RenderTree (from `render_shell_sections`) into a UiTree.
    ///
    /// - Preserves ordering (top-to-bottom).
    /// - Preserves presence/absence.
    /// - Maps RenderSectionKind -> UiSectionKind.
    /// - Propagates textual lines as UiLine entries.
    pub fn render_ui(tree: &RenderTree) -> UiTree {
        let mut blocks = Vec::with_capacity(tree.sections.len());

        for s in &tree.sections {
            let kind = match &s.kind {
                RenderSectionKind::Content => UiSectionKind::Content,
                RenderSectionKind::Status => UiSectionKind::Status,
                RenderSectionKind::AI => UiSectionKind::AI,
                RenderSectionKind::Other(t) => UiSectionKind::Other(t.clone()),
            };

            let lines = s.lines.iter().cloned().map(UiLine).collect::<Vec<_>>();

            blocks.push(UiBlock {
                id: s.id.clone(),
                kind,
                present: s.present,
                lines,
            });
        }

        UiTree { blocks }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use zaroxi_interface_app::shell_render_view::SectionView;

        #[test]
        fn ui_mapper_preserves_kind_presence_ordering_and_content() {
            let vm = ShellRenderViewModel {
                sections: vec![
                    SectionView {
                        id: "content".to_string(),
                        present: true,
                        lines: vec!["line1".to_string()],
                    },
                    SectionView {
                        id: "status".to_string(),
                        present: false,
                        lines: vec![],
                    },
                    SectionView {
                        id: "ai".to_string(),
                        present: true,
                        lines: vec!["ai-line".to_string()],
                    },
                ],
            };

            // Build the structured render tree using the existing function under test.
            let tree = render_shell_sections(&vm);

            // Map into UI nodes.
            let ui = render_ui(&tree);

            // Expect three blocks preserved in ordering.
            assert_eq!(ui.blocks.len(), 3);

            // Content block -> UiSectionKind::Content, present, content preserved.
            assert_eq!(ui.blocks[0].kind, UiSectionKind::Content);
            assert!(ui.blocks[0].present);
            assert_eq!(ui.blocks[0].lines.len(), 1);
            assert_eq!(ui.blocks[0].lines[0].0, "line1");

            // Status block -> UiSectionKind::Status, absent preserved.
            assert_eq!(ui.blocks[1].kind, UiSectionKind::Status);
            assert!(!ui.blocks[1].present);

            // AI block -> UiSectionKind::AI, present, content preserved.
            assert_eq!(ui.blocks[2].kind, UiSectionKind::AI);
            assert!(ui.blocks[2].present);
            assert_eq!(ui.blocks[2].lines.len(), 1);
            assert_eq!(ui.blocks[2].lines[0].0, "ai-line");

            // Ordering: ids preserved top-to-bottom.
            let ids: Vec<String> = ui.blocks.iter().map(|b| b.id.clone()).collect();
            assert_eq!(ids, vec!["content".to_string(), "status".to_string(), "ai".to_string()]);
        }
    }
}

pub mod terminal_ui {
    //! Minimal terminal-only UI layer.
    //!
    //! This thin module consumes the UiTree produced by the presenters/ui mapper
    //! and emits a deterministic, top-to-bottom textual "terminal surface"
    //! (wireframe) without any styling, interaction or state mutation.
    //!
    //! Purpose:
    //! - Preserve section ordering and presence/absence.
    //! - Produce a simple TerminalSurface useful for harnesses and tests.
    //! - Keep strictly UI-only: no engine or application logic here.

    use super::ui::UiTree;

    /// Simple terminal surface representation returned by the renderer.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TerminalSurface {
        pub lines: Vec<String>,
    }

    /// Render the given UiTree into a top-to-bottom terminal surface.
    ///
    /// Semantics:
    /// - Each block yields a header line "Section: <id> (<kind>)".
    /// - Present blocks emit their textual lines (prefixed by two spaces).
    /// - Absent blocks emit a single "  <absent>" placeholder line.
    /// - The renderer is intentionally tiny and deterministic for tests/harnesses.
    pub fn render_terminal_ui(tree: &UiTree) -> TerminalSurface {
        let mut lines = Vec::new();

        lines.push("TerminalSurface start".to_string());

        for block in &tree.blocks {
            lines.push(format!("Section: {} ({:?})", block.id, block.kind));
            if block.present {
                for line in &block.lines {
                    lines.push(format!("  {}", line.0));
                }
            } else {
                lines.push("  <absent>".to_string());
            }
        }

        lines.push("TerminalSurface end".to_string());

        TerminalSurface { lines }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use super::super::ui::{UiTree, UiBlock, UiLine, UiSectionKind};

        #[test]
        fn terminal_ui_renders_order_and_presence() {
            let tree = UiTree {
                blocks: vec![
                    UiBlock {
                        id: "content".to_string(),
                        kind: UiSectionKind::Content,
                        present: true,
                        lines: vec![UiLine("hello".to_string())],
                    },
                    UiBlock {
                        id: "status".to_string(),
                        kind: UiSectionKind::Status,
                        present: false,
                        lines: vec![],
                    },
                ],
            };

            let surface = render_terminal_ui(&tree);
            let out = surface.lines.join("\n");

            // Basic structural assertions
            assert!(out.contains("Section: content"));
            assert!(out.contains("  hello"));
            assert!(out.contains("Section: status"));
            assert!(out.contains("<absent>"));

            // Ordering: content header must appear before status header.
            let content_pos = out.find("Section: content").unwrap();
            let status_pos = out.find("Section: status").unwrap();
            assert!(content_pos < status_pos);

            // Surface wrappers present
            assert!(out.starts_with("TerminalSurface start"));
            assert!(out.ends_with("TerminalSurface end"));
        }
    }
}
