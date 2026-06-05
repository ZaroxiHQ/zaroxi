use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_interface_theme::theme::SemanticColors;

use super::super::editor::EditorContentData;

/// Shape live editor content from work_content into `EditorContentData`.
/// Includes line numbering, tab/breadcrumb extraction, syntax coloring,
/// and cursor position.
pub fn shape_editor_content(
    work_content: &Option<ShellWorkContent>,
    sem: &SemanticColors,
) -> EditorContentData {
    let wc = match work_content {
        Some(w) => w,
        None => return EditorContentData::default(),
    };

    let editor_body = wc.editor_body.as_ref();

    let editor_body_text = editor_body
        .map(|cv| {
            let numbered: String = cv
                .lines
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4} │ {}", i + 1, line))
                .collect::<Vec<_>>()
                .join("\n");
            numbered
        })
        .unwrap_or_else(|| "No file open".to_string());

    let cursor_line = editor_body.map(|cv| cv.cursor_line).unwrap_or(0);
    let cursor_col = editor_body.map(|cv| cv.cursor_col).unwrap_or(0);

    let editor_spans: Option<Vec<(String, [f32; 4])>> = editor_body.and_then(|cv| {
        if cv.lines.is_empty() {
            return None;
        }
        super::super::syntax_color::colorize_source(&cv.lines, sem).into()
    });

    let tab_labels = wc.editor_tabs.clone().unwrap_or_else(Vec::new);
    let tab_title = tab_labels.first().cloned().unwrap_or_else(|| "No file open".into());
    let tab_content: String = tab_labels.iter().skip(1).cloned().collect::<Vec<_>>().join("  ");

    let breadcrumb_label = wc.editor_breadcrumb.clone().unwrap_or_else(String::new);

    EditorContentData {
        tab_title,
        tab_content,
        breadcrumb_label,
        editor_body_text,
        editor_spans,
        cursor_line,
        cursor_col,
    }
}
