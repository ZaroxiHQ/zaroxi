/*!
Tiny semantic render intent.

Purpose:
- Provide a read-only, semantic view of "what to draw" produced from
  `zaroxi_core_engine_layout::ShellLayoutInput`.
- Keeps ordering and presence semantics only. No geometry, metrics, colors,
  or any drawing-facing concepts are contained here.
*/

use zaroxi_core_engine_layout::{ShellLayoutInput, LayoutBlock};
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

/// Minimal representation of a single tab suitable for render-stage semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromeTab {
    /// 1-based tab index (keeps presenter's convention).
    pub index: u32,
    /// Stable identifier for the tab (presenter-provided).
    pub id: String,
    /// Short display label for the tab.
    pub label: String,
    /// Whether this tab is active.
    pub active: bool,
}

/// Small engine-facing chrome primitive that carries only semantic facts.
/// Intentionally monospace/minimal: no colors, fonts, or layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromePrimitive {
    pub chrome_label: Option<String>,
    pub tabs: Vec<ChromeTab>,
    pub active_tab_index: Option<usize>,
    pub focus_slot: Option<String>,
    pub status_text: Option<String>,
    pub ai_indicator: Option<String>,
    pub content_preview: Option<String>,
}

impl Default for ChromePrimitive {
    fn default() -> Self {
        Self {
            chrome_label: None,
            tabs: Vec::new(),
            active_tab_index: None,
            focus_slot: None,
            status_text: None,
            ai_indicator: None,
            content_preview: None,
        }
    }
}

impl From<ShellChrome> for ChromePrimitive {
    fn from(src: ShellChrome) -> Self {
        let tabs = src
            .tabs
            .into_iter()
            .map(|t| ChromeTab {
                index: t.index,
                id: t.id,
                label: t.label,
                active: t.active,
            })
            .collect();

        ChromePrimitive {
            chrome_label: src.chrome_label,
            tabs,
            active_tab_index: src.active_tab_index,
            focus_slot: src.focus_slot,
            status_text: src.status_text,
            ai_indicator: src.ai_indicator,
            content_preview: src.content_preview,
        }
    }
}

impl From<ShellChrome> for RenderSection {
    fn from(chrome: ShellChrome) -> Self {
        RenderSection::Chrome {
            chrome: ChromePrimitive::from(chrome),
        }
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
                    // Layout-level chrome blocks do not carry presenter chrome data.
                    // Produce an empty engine chrome primitive as a stable, deterministic
                    // placeholder. Presenters that have richer chrome (ShellChrome)
                    // can be converted into RenderSection::from(ShellChrome) elsewhere.
                    sections.push(RenderSection::Chrome { chrome: ChromePrimitive::default() });
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
