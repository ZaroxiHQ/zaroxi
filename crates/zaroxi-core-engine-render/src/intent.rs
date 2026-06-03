/*!
Tiny semantic render intent.

Purpose:
- Provide a read-only, semantic view of "what to draw" produced from
  `zaroxi_core_engine_layout::ShellLayoutInput`.
- Keeps ordering and presence semantics only. No geometry, metrics, colors,
  or any drawing-facing concepts are contained here.
*/

use zaroxi_core_engine_layout::{LayoutBlock, ShellLayoutInput};
use zaroxi_core_engine_scene::scene::ShellChrome;

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

    /// Engine-side chrome primitive carrying tab semantics.
    Chrome { chrome: ChromePrimitive },
}

/// Minimal representation of a single panel tab suitable for render-stage semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelTab {
    /// 1-based tab index (keeps presenter's convention).
    pub index: u32,
    /// Stable identifier for the tab (presenter-provided).
    pub id: String,
    /// Short display label for the tab.
    pub label: String,
    /// Whether this tab is active.
    pub active: bool,
}

/// Small engine-facing decoration primitive that carries only semantic facts.
/// Intentionally monospace/minimal: no colors, fonts, or layout.
///
/// Phase 38: Renamed from ChromePrimitive. Removed IDE-specific fields
/// (ai_indicator, content_preview).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromePrimitive {
    pub chrome_label: Option<String>,
    pub tabs: Vec<PanelTab>,
    pub active_tab_index: Option<usize>,
    pub active_panel_id: Option<String>,
    pub status_text: Option<String>,
}

impl Default for ChromePrimitive {
    fn default() -> Self {
        Self {
            chrome_label: None,
            tabs: Vec::new(),
            active_tab_index: None,
            active_panel_id: None,
            status_text: None,
        }
    }
}

impl From<ShellChrome> for ChromePrimitive {
    fn from(src: ShellChrome) -> Self {
        let tabs = src
            .tabs
            .into_iter()
            .map(|t| PanelTab { index: t.index, id: t.id, label: t.label, active: t.active })
            .collect();

        ChromePrimitive {
            chrome_label: src.chrome_label,
            tabs,
            active_tab_index: src.active_tab_index,
            active_panel_id: src.active_panel_id,
            status_text: src.status_text,
        }
    }
}

impl From<ShellChrome> for RenderSection {
    fn from(chrome: ShellChrome) -> Self {
        RenderSection::Chrome { chrome: ChromePrimitive::from(chrome) }
    }
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
                    sections.push(RenderSection::Selection { line: sb.line, column: sb.column });
                }
                LayoutBlock::Status(st) => {
                    sections.push(RenderSection::Status { summary: st.summary });
                }
                LayoutBlock::Decoration => {
                    // Layout-level decoration blocks do not carry presenter chrome data.
                    // Produce an empty engine chrome primitive as a stable, deterministic
                    // placeholder. Presenters that have richer chrome (ShellChrome)
                    // can be converted into RenderSection::from(ShellChrome) elsewhere.
                    sections.push(RenderSection::Chrome { chrome: ChromePrimitive::default() });
                }
            }
        }

        let selection_present =
            sections.iter().any(|s| matches!(s, RenderSection::Selection { .. }));
        let status_present = sections.iter().any(|s| matches!(s, RenderSection::Status { .. }));

        ShellRenderIntent { sections, selection_present, status_present }
    }
}
