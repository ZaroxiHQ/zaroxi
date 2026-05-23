use zaroxi_core_engine_scene::{TextPrimitive, CaretItem, SelectionRect, EditorPrimitiveSet};

/// editor layout / font helpers for caret/selection projection into transcript.
/// Provide a tiny, local layout/font shim so this presenter does not
/// require adding new workspace crate dependencies in Cargo.toml.
///
/// This keeps the presenter lightweight and deterministic: we only need
/// cursor/selection fields and stable monospace metrics for transcript math.
#[derive(Clone, Debug)]
pub struct EditorLayoutSpec {
    /// 1-based top-most visible line index (when known). This is used to
    /// convert absolute document line numbers (cursor/selection) into visible
    /// row offsets inside the provided `editor_lines` slice.
    pub top_line: Option<u32>,
    pub cursor_line: Option<u32>,
    pub cursor_column: Option<u32>,
    pub selection: Option<(u32, u32, u32, u32)>,
}

/// Deterministic local monospace metrics used by the presenter (kept small).
pub const DEFAULT_CHAR_WIDTH: u32 = 8;
pub const DEFAULT_LINE_HEIGHT: u32 = 16;

/// Build a minimal engine-facing EditorPrimitiveSet directly from the
/// presenter's deterministic visible-line inputs (visible rows and an optional
/// EditorLayoutSpec). This re-implements the same projection math the
/// presenter uses to emit "Gutter"/"Text"/"Caret"/"Selection" plan lines so
/// tests and harnesses can validate the exact primitives without pulling
/// presenter internals into engine backends.
///
/// This function is intentionally stable and deterministic: it mirrors
/// the presenter's metrics constants (DEFAULT_CHAR_WIDTH / DEFAULT_LINE_HEIGHT)
/// and uses the same content inset heuristics.
pub fn build_editor_primitives_from_lines(
    content_x: u32,
    base_y: u32,
    editor_lines: &[String],
    editor_layout: Option<&EditorLayoutSpec>,
) -> EditorPrimitiveSet {
    let mut set = EditorPrimitiveSet::new();

    // Stable presenter heuristics (must match presenter emission)
    let gutter_width: u32 = 48;
    let line_height: u32 = DEFAULT_LINE_HEIGHT;
    let char_w: u32 = DEFAULT_CHAR_WIDTH;
    let content_inset: u32 = 6;

    // gutter_x placed to the left of the content rect
    let gutter_x = if content_x > gutter_width { content_x - gutter_width } else { 0 };

    // Determine the absolute top-most visible document line (1-based)
    let top_line_val = editor_layout.and_then(|l| l.top_line).unwrap_or(1);

    // Emit gutter/text primitives per visible row
    for (i, text) in editor_lines.iter().enumerate() {
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

        // Content text entry (slight inset)
        let content_text_x = content_x.saturating_add(content_inset);
        set.texts.push(TextPrimitive {
            x: content_text_x,
            y,
            text: text.clone(),
            font_name: "ZaroxiMono".to_string(),
            max_width: None,
        });
    }

    // If layout provided, project caret & selections into primitives.
    if let Some(layout) = editor_layout {
        let content_text_x = content_x.saturating_add(content_inset);

        // Caret projection
        if let Some(cl) = layout.cursor_line {
            let col = layout.cursor_column.unwrap_or(0);
            let top_line_val = layout.top_line.unwrap_or(1);
            let offset_rows = cl.saturating_sub(top_line_val);
            let caret_x = content_text_x.saturating_add(col.saturating_mul(char_w));
            let caret_y = base_y.saturating_add(offset_rows.saturating_mul(line_height));
            set.carets.push(CaretItem { x: caret_x, y: caret_y, height: line_height });
        }

        // Selection projection: one rect per visible row intersection.
        if let Some((sline, scol, eline, ecol)) = layout.selection {
            let top_line_val = layout.top_line.unwrap_or(1);
            for (i, _) in editor_lines.iter().enumerate() {
                let row = top_line_val.saturating_add(i as u32);
                if row < sline || row > eline {
                    continue;
                }
                let sel_start_col = if row == sline { scol } else { 0 };
                let sel_end_col = if row == eline {
                    ecol
                } else {
                    editor_lines.get(i).map(|s| s.chars().count() as u32).unwrap_or(0)
                };
                if sel_end_col <= sel_start_col {
                    continue;
                }
                let sx = content_text_x.saturating_add(sel_start_col.saturating_mul(char_w));
                let w = sel_end_col.saturating_sub(sel_start_col).saturating_mul(char_w);
                let sy = base_y.saturating_add((i as u32).saturating_mul(line_height));
                set.selections.push(SelectionRect { x: sx, y: sy, width: w, height: line_height });
            }
        }
    }

    set
}
