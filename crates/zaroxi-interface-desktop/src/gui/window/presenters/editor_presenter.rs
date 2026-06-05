use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_interface_theme::theme::SemanticColors;

use super::super::editor::EditorContentData;

/// Shape live editor content from work_content into `EditorContentData`.
/// Uses the syntax_color module for syntax highlighting, and includes
/// ContentView title for the editor body header.
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
            cv.lines
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4} │ {}", i + 1, line))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "No file open".to_string());

    let cursor_line = editor_body.map(|cv| cv.cursor_line).unwrap_or(0);
    let cursor_col = editor_body.map(|cv| cv.cursor_col).unwrap_or(0);

    let body_title = editor_body
        .map(|cv| if cv.title.is_empty() { cv.subtitle.clone() } else { cv.title.clone() })
        .unwrap_or_default();

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
        body_title,
    }
}
