use zaroxi_core_editor_view::{EditorRenderContract, EditorRenderMetrics};
use zaroxi_core_engine_scene::{CaretItem, EditorPrimitiveSet, SelectionRect, TextPrimitive};

/// Editor layout spec: kept for backward-compatibility with existing callers.
/// This is a thin wrapper around the fields shared with `EditorRenderContract`.
///
/// Phase 39: `EditorRenderContract` (from `zaroxi-core-editor-view`) is the
/// canonical input type. New code should use the contract directly.
/// `EditorLayoutSpec` remains for existing callers.
#[derive(Clone, Debug)]
pub struct EditorLayoutSpec {
    pub top_line: Option<u32>,
    pub cursor_line: Option<u32>,
    pub cursor_column: Option<u32>,
    pub selection: Option<(u32, u32, u32, u32)>,
}

/// Deterministic local monospace metrics used by the presenter (kept small).
/// Phase 39: `EditorRenderMetrics` (from `zaroxi-core-editor-view`) is the
/// canonical metrics type. These constants remain for backward compatibility.
pub const DEFAULT_CHAR_WIDTH: u32 = 8;
pub const DEFAULT_LINE_HEIGHT: u32 = 16;

/// Build an `EditorPrimitiveSet` from the formal editor render contract.
///
/// This is the canonical projection function consuming the app-neutral contract
/// types from `zaroxi-core-editor-view`. It uses `EditorRenderMetrics` for
/// deterministic layout rather than hardcoded constants.
pub fn build_primitives_from_contract(
    content_x: u32,
    base_y: u32,
    contract: &EditorRenderContract,
    metrics: &EditorRenderMetrics,
) -> EditorPrimitiveSet {
    let mut set = EditorPrimitiveSet::new();

    let gutter_width = metrics.gutter_width;
    let line_height = metrics.line_height;
    let char_w = metrics.char_width;
    let content_inset = metrics.content_inset;

    let gutter_x = content_x.saturating_sub(gutter_width);
    let top_line_val = contract.top_line;

    // Emit gutter/text primitives per visible row
    for (i, text) in contract.visible_lines.iter().enumerate() {
        let doc_row = top_line_val.saturating_add(i as u32);
        let y = base_y.saturating_add((i as u32).saturating_mul(line_height));

        // Gutter label
        let label = format!("{:>4}", doc_row);
        set.gutter_labels.push(TextPrimitive {
            x: gutter_x,
            y,
            text: label,
            font_name: "ZaroxiMono".to_string(),
            max_width: None,
        });

        // Content text entry
        let content_text_x = content_x.saturating_add(content_inset);
        set.texts.push(TextPrimitive {
            x: content_text_x,
            y,
            text: text.clone(),
            font_name: "ZaroxiMono".to_string(),
            max_width: None,
        });
    }

    // Caret projection
    let content_text_x = content_x.saturating_add(content_inset);

    if let Some(cl) = contract.cursor_line {
        let col = contract.cursor_column.unwrap_or(0);
        let offset_rows = cl.saturating_sub(top_line_val);
        let caret_x = content_text_x.saturating_add(col.saturating_mul(char_w));
        let caret_y = base_y.saturating_add(offset_rows.saturating_mul(line_height));
        set.carets.push(CaretItem { x: caret_x, y: caret_y, height: line_height });
    }

    // Selection projection: one rect per visible row intersection
    if let Some((sline, scol, eline, ecol)) = contract.selection {
        for (i, line_text) in contract.visible_lines.iter().enumerate() {
            let row = top_line_val.saturating_add(i as u32);
            if row < sline || row > eline {
                continue;
            }
            let sel_start_col = if row == sline { scol } else { 0 };
            let sel_end_col = if row == eline { ecol } else { line_text.chars().count() as u32 };
            if sel_end_col <= sel_start_col {
                continue;
            }
            let sx = content_text_x.saturating_add(sel_start_col.saturating_mul(char_w));
            let w = sel_end_col.saturating_sub(sel_start_col).saturating_mul(char_w);
            let sy = base_y.saturating_add((i as u32).saturating_mul(line_height));
            set.selections.push(SelectionRect { x: sx, y: sy, width: w, height: line_height });
        }
    }

    set
}

/// Build a minimal engine-facing EditorPrimitiveSet directly from the
/// presenter's deterministic visible-line inputs and an optional
/// EditorLayoutSpec. This is kept for backward compatibility.
///
/// New code should use `build_primitives_from_contract` with the formal
/// `EditorRenderContract` + `EditorRenderMetrics` types.
pub fn build_editor_primitives_from_lines(
    content_x: u32,
    base_y: u32,
    editor_lines: &[String],
    editor_layout: Option<&EditorLayoutSpec>,
) -> EditorPrimitiveSet {
    let top_line = editor_layout.and_then(|l| l.top_line).unwrap_or(1);
    let cursor_line = editor_layout.and_then(|l| l.cursor_line);
    let cursor_column = editor_layout.and_then(|l| l.cursor_column);
    let selection = editor_layout.and_then(|l| l.selection);

    let contract = EditorRenderContract::new(
        editor_lines.to_vec(),
        top_line,
        cursor_line,
        cursor_column,
        selection,
    );

    build_primitives_from_contract(content_x, base_y, &contract, &EditorRenderMetrics::DEFAULT)
}
