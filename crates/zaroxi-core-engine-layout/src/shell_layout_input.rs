/*!
Tiny structural layout input model.

Design notes (short):
- Purpose: provide a read-only, structural-only input for a future layout solver.
- It is created from `zaroxi_core_engine_scene::ShellSceneModel`.
- It contains semantic blocks (Text, optional Selection, optional Status) and viewport facts.
- It intentionally omits any geometric information (no coordinates, sizes, margins, fonts, colors, etc).
*/

use std::vec::Vec;

/// Top-level layout input produced from a scene description.
///
/// Keeps only structural/semantic information: ordered blocks and viewport facts.
/// No geometry or rendering details are present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellLayoutInput {
    pub blocks: Vec<LayoutBlock>,
    pub viewport: ViewportFacts,
}

/// Semantic block kinds preserved from the scene in a small, ordered list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutBlock {
    /// Main text content block (always present).
    Text(TextBlock),

    /// Optional selection/cursor block (present when the scene reports a cursor).
    Selection(SelectionBlock),

    /// Optional status summary block (present when the scene provides a viewport summary).
    Status(StatusBlock),

    /// Placeholder for potential chrome UI (kept for future structural use).
    Chrome,
}

/// Main textual block: an ordered list of visible lines (cloned from the scene).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBlock {
    pub lines: Vec<String>,
}

/// Selection block expresses the logical cursor/selection location (line/column).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionBlock {
    /// 1-based line index (keeps same convention as the scene model).
    pub line: u32,
    /// 0-based column index (keeps same convention as the scene model).
    pub column: u32,
}

/// Small status block carrying an optional summary string from the scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusBlock {
    pub summary: String,
}

/// Lightweight viewport facts preserved for layout decisions at higher phases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportFacts {
    pub top_line: u32,
    pub total_lines: u32,
    pub summary: Option<String>,

    /// Mirror scene cursor facts (if any) to allow layout layers to reason about
    /// selection presence without peeking into the scene model.
    pub cursor_line: Option<u32>,
    pub cursor_column: Option<u32>,
}

/// Convert from the scene model into the tiny layout input.
///
/// Rules:
/// - Always produce a Text block containing `scene.text_lines`.
/// - If `scene.cursor_line` is Some -> produce a Selection block (uses cursor_column if present, otherwise 0).
/// - If `scene.viewport_summary` is Some(non-empty) -> produce a Status block (preserve the summary).
impl From<zaroxi_core_engine_scene::ShellSceneModel> for ShellLayoutInput {
    fn from(scene: zaroxi_core_engine_scene::ShellSceneModel) -> Self {
        let mut blocks: Vec<LayoutBlock> = Vec::new();

        // Text block (structural, always present)
        blocks.push(LayoutBlock::Text(TextBlock { lines: scene.text_lines.clone() }));

        // Selection block if the scene reports a cursor line
        if let Some(line) = scene.cursor_line {
            let column = scene.cursor_column.unwrap_or(0);
            blocks.push(LayoutBlock::Selection(SelectionBlock { line, column }));
        }

        // Status block when a viewport summary exists and is non-empty
        if let Some(summary) = scene.viewport_summary.clone() {
            if !summary.is_empty() {
                blocks.push(LayoutBlock::Status(StatusBlock { summary }));
            }
        }

        ShellLayoutInput {
            blocks,
            viewport: ViewportFacts {
                top_line: scene.viewport_top_line,
                total_lines: scene.viewport_total_lines,
                summary: scene.viewport_summary,
                cursor_line: scene.cursor_line,
                cursor_column: scene.cursor_column,
            },
        }
    }
}
