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

    /// Convenience: render using an actual ShellDrawPlan by combining a concise
    /// engine debug summary with the view model.
    ///
    /// Note: historically this called into `render_debug_text(plan)`. To avoid
    /// brittle direct imports of engine helper symbols (which moved during the
    /// refactor), we produce a stable, concise placeholder summary here. The
    /// present() API still accepts an arbitrary engine debug string so outer
    /// layers may continue to call present() with richer debug text when needed.
    pub fn present_with_plan(
        &self,
        vm: &ShellRenderViewModel,
        _plan: &zaroxi_core_engine_render::ShellDrawPlan,
    ) -> String {
        // Keep the placeholder short and deterministic; tests within this
        // crate use present(&vm, Some(...)) directly for richer debug lines.
        let debug = "engine-draw-plan".to_string();
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
            sections: vec![SectionView {
                id: "main".to_string(),
                present: true,
                lines: vec!["RenderSection { id: \"main\", ... }".to_string()],
            }],
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
            sections: vec![SectionView {
                id: "content".to_string(),
                present: true,
                lines: vec!["hello from content".to_string()],
            }],
        };

        let out = render_terminal(&vm);

        assert!(out.contains("Layout sections (top-to-bottom):"));
        assert!(out.contains("  - content: present (1 lines)"));
        assert!(out.contains("hello from content"));
    }

    #[test]
    fn render_shell_sections_builds_structured_tree() {
        let vm = ShellRenderViewModel {
            sections: vec![SectionView {
                id: "content".to_string(),
                present: true,
                lines: vec!["line1".to_string(), "line2".to_string()],
            }],
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

            blocks.push(UiBlock { id: s.id.clone(), kind, present: s.present, lines });
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
                    SectionView { id: "status".to_string(), present: false, lines: vec![] },
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
        use super::super::ui::{UiBlock, UiLine, UiSectionKind, UiTree};
        use super::*;

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

/// Layout submodule: partition a UiTree into simple top/main/bottom regions.
///
/// This module is intentionally thin, UI-only, and deterministic. It
/// preserves ordering and presence/absence; it does not perform any styling
/// or event handling. The layout is a simple contiguous partitioning:
/// - top:    all blocks before the first Content block (exclusive)
/// - main:   the contiguous run from the first Content block up to the last Content block (inclusive)
/// - bottom: all blocks after the last Content block (exclusive)
///
/// If there are no Content blocks, trailing Status blocks (if any) are placed
/// in the bottom region; the remaining leading blocks are placed in top.
pub mod layout {
    use super::terminal_ui;
    use super::ui::{UiBlock, UiSectionKind, UiTree};
    use std::vec::Vec;

    /// Simple split layout with three named regions.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SplitLayout {
        pub top: Vec<UiBlock>,
        pub main: Vec<UiBlock>,
        pub bottom: Vec<UiBlock>,
    }

    impl SplitLayout {
        pub fn new() -> Self {
            Self { top: Vec::new(), main: Vec::new(), bottom: Vec::new() }
        }

        /// Reconstruct a UiTree by concatenating regions top->main->bottom.
        pub fn to_uitree(&self) -> UiTree {
            let mut blocks = Vec::new();
            blocks.extend(self.top.clone());
            blocks.extend(self.main.clone());
            blocks.extend(self.bottom.clone());
            UiTree { blocks }
        }

        /// Convenience: render the layout using the existing terminal UI renderer.
        pub fn render_as_terminal_surface(&self) -> terminal_ui::TerminalSurface {
            let tree = self.to_uitree();
            terminal_ui::render_terminal_ui(&tree)
        }
    }

    /// Partition the provided UiTree into a SplitLayout.
    ///
    /// Algorithm (deterministic, top-to-bottom):
    /// 1. Locate first and last indices of UiSectionKind::Content.
    /// 2. If any Content present:
    ///    - top = blocks[..first_content]
    ///    - main = blocks[first_content..=last_content]
    ///    - bottom = blocks[last_content+1..]
    /// 3. If no Content present:
    ///    - bottom = trailing Status blocks (zero or more)
    ///    - top = remaining leading blocks
    pub fn layout_ui_tree(tree: &UiTree) -> SplitLayout {
        let mut layout = SplitLayout::new();
        let len = tree.blocks.len();

        // Find first and last content indices
        let mut first_content: Option<usize> = None;
        let mut last_content: Option<usize> = None;

        for (i, b) in tree.blocks.iter().enumerate() {
            if let UiSectionKind::Content = b.kind {
                if first_content.is_none() {
                    first_content = Some(i);
                }
                last_content = Some(i);
            }
        }

        match (first_content, last_content) {
            (Some(first), Some(last)) => {
                // top
                for i in 0..first {
                    layout.top.push(tree.blocks[i].clone());
                }
                // main
                for i in first..=last {
                    layout.main.push(tree.blocks[i].clone());
                }
                // bottom
                for i in last + 1..len {
                    layout.bottom.push(tree.blocks[i].clone());
                }
            }
            (None, None) => {
                // No content blocks:
                // Find start index of trailing status blocks
                let mut split_at = len; // by default, no trailing status
                for (i, b) in tree.blocks.iter().enumerate().rev() {
                    match b.kind {
                        UiSectionKind::Status => {
                            split_at = i;
                        }
                        _ => {
                            // once we encounter a non-status from the end, stop
                            break;
                        }
                    }
                }

                if split_at == len {
                    // No trailing status blocks -> everything is top
                    for b in &tree.blocks {
                        layout.top.push(b.clone());
                    }
                } else {
                    // split_at is index of first trailing status block; everything before it is top
                    for i in 0..split_at {
                        layout.top.push(tree.blocks[i].clone());
                    }
                    for i in split_at..len {
                        layout.bottom.push(tree.blocks[i].clone());
                    }
                }
            }
            _ => unreachable!("first_content and last_content must both be Some or both None"),
        }

        layout
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::presenters::shell_render::ui::{UiBlock, UiLine, UiSectionKind, UiTree};

        #[test]
        fn layout_partitions_simple_chrome_content_status() {
            let tree = UiTree {
                blocks: vec![
                    UiBlock {
                        id: "chrome".to_string(),
                        kind: UiSectionKind::Other("chrome".to_string()),
                        present: true,
                        lines: vec![UiLine("chrome-line".to_string())],
                    },
                    UiBlock {
                        id: "content".to_string(),
                        kind: UiSectionKind::Content,
                        present: true,
                        lines: vec![UiLine("content-line".to_string())],
                    },
                    UiBlock {
                        id: "status".to_string(),
                        kind: UiSectionKind::Status,
                        present: true,
                        lines: vec![UiLine("status-line".to_string())],
                    },
                ],
            };

            let layout = layout_ui_tree(&tree);

            assert_eq!(layout.top.len(), 1);
            assert_eq!(layout.top[0].id, "chrome");
            assert_eq!(layout.main.len(), 1);
            assert_eq!(layout.main[0].id, "content");
            assert_eq!(layout.bottom.len(), 1);
            assert_eq!(layout.bottom[0].id, "status");

            // Rendering round-trip should include the same lines in order
            let surface = layout.render_as_terminal_surface();
            let out = surface.lines.join("\n");
            assert!(out.contains("chrome-line"));
            assert!(out.contains("content-line"));
            assert!(out.contains("status-line"));

            // Order: chrome before content before status
            let chrome_pos = out.find("chrome-line").unwrap();
            let content_pos = out.find("content-line").unwrap();
            let status_pos = out.find("status-line").unwrap();
            assert!(chrome_pos < content_pos && content_pos < status_pos);
        }

        #[test]
        fn layout_handles_multiple_content_and_ai_in_main_region() {
            let tree = UiTree {
                blocks: vec![
                    UiBlock {
                        id: "topbar".to_string(),
                        kind: UiSectionKind::Other("topbar".to_string()),
                        present: true,
                        lines: vec![UiLine("top".to_string())],
                    },
                    UiBlock {
                        id: "editor-a".to_string(),
                        kind: UiSectionKind::Content,
                        present: true,
                        lines: vec![UiLine("a".to_string())],
                    },
                    UiBlock {
                        id: "assistant".to_string(),
                        kind: UiSectionKind::AI,
                        present: true,
                        lines: vec![UiLine("ai".to_string())],
                    },
                    UiBlock {
                        id: "editor-b".to_string(),
                        kind: UiSectionKind::Content,
                        present: true,
                        lines: vec![UiLine("b".to_string())],
                    },
                    UiBlock {
                        id: "status".to_string(),
                        kind: UiSectionKind::Status,
                        present: true,
                        lines: vec![UiLine("st".to_string())],
                    },
                ],
            };

            let layout = layout_ui_tree(&tree);

            // Top should contain only the topbar
            assert_eq!(layout.top.len(), 1);
            assert_eq!(layout.top[0].id, "topbar");

            // Main should contain editor-a, assistant, editor-b (preserve ordering)
            assert_eq!(layout.main.len(), 3);
            assert_eq!(layout.main[0].id, "editor-a");
            assert_eq!(layout.main[1].id, "assistant");
            assert_eq!(layout.main[2].id, "editor-b");

            // Bottom should contain status
            assert_eq!(layout.bottom.len(), 1);
            assert_eq!(layout.bottom[0].id, "status");
        }
    }
}
