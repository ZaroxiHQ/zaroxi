/*!
Tiny semantic render intent.

Purpose:
- Provide a read-only, semantic view of "what to draw" produced from
  `zaroxi_core_engine_layout::ShellLayoutInput`.
- Keeps ordering and presence semantics only. No geometry, metrics, colors,
  or any drawing-facing concepts are contained here.
*/

use zaroxi_core_engine_layout::{ShellLayoutInput, LayoutBlock};

/// Top-level semantic render intent for a shell view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRenderIntent {
    /// Ordered sections the renderer should consider (semantic only).
    pub sections: Vec<RenderSection>,

    /// Convenience flags to quickly test for selection/status presence.
    pub selection_present: bool,
    pub status_present: bool,
}

/// Per-section semantic kinds preserved from the layout input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderSection {
    /// Main text block (cloned lines only).
    Text { lines: Vec<String> },

    /// Logical selection/cursor marker (line is 1-based to match layout model).
    Selection { line: u32, column: u32 },

    /// Small status summary block (semantic string only).
    Status { summary: String },

    /// Placeholder for non-text chrome sections.
    Chrome,
}

impl From<ShellLayoutInput> for ShellRenderIntent {
    fn from(input: ShellLayoutInput) -> Self {
        let mut sections: Vec<RenderSection> = Vec::new();

        // Consume blocks preserving order and semantics.
        for block in input.blocks.into_iter() {
            match block {
                LayoutBlock::Text(tb) => {
                    sections.push(RenderSection::Text { lines: tb.lines });
                }
                LayoutBlock::Selection(sb) => {
                    sections.push(RenderSection::Selection {
                        line: sb.line,
                        column: sb.column,
                    });
                }
                LayoutBlock::Status(st) => {
                    sections.push(RenderSection::Status {
                        summary: st.summary,
                    });
                }
                LayoutBlock::Chrome => {
                    sections.push(RenderSection::Chrome);
                }
            }
        }

        let selection_present = sections.iter().any(|s| matches!(s, RenderSection::Selection { .. }));
        let status_present = sections.iter().any(|s| matches!(s, RenderSection::Status { .. }));

        ShellRenderIntent {
            sections,
            selection_present,
            status_present,
        }
    }
}
