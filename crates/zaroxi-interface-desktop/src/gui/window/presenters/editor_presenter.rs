use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::chrome::TabEntry;
use zaroxi_core_platform_syntax::parser::ParserPool;
use zaroxi_interface_theme::theme::SemanticColors;

use super::super::editor::EditorContentData;

pub fn shape_editor_content(
    work_content: &Option<ShellWorkContent>,
    sem: &SemanticColors,
    parser_pool: &ParserPool,
) -> EditorContentData {
    let wc = match work_content {
        Some(w) => w,
        None => return EditorContentData::default(),
    };

    let editor_body = wc.editor_body.as_ref();

    let editor_body_text =
        editor_body.map(|cv| cv.lines.join("\n")).unwrap_or_else(|| "No file open".to_string());

    let cursor_line = editor_body.map(|cv| cv.cursor_line).unwrap_or(0);
    let cursor_col = editor_body.map(|cv| cv.cursor_col).unwrap_or(0);

    let body_title = editor_body
        .map(|cv| if cv.title.is_empty() { cv.subtitle.clone() } else { cv.title.clone() })
        .unwrap_or_default();

    let editor_spans: Option<Vec<(String, [f32; 4])>> = editor_body.map(|cv| {
        let mut spans: Vec<(String, [f32; 4])> = Vec::new();

        if cv.lines.is_empty() {
            return spans;
        }

        let syntax_spans = super::super::syntax_color::colorize_source(&cv.lines, sem, parser_pool);

        let mut syntax_by_line: std::collections::BTreeMap<usize, Vec<(String, [f32; 4])>> =
            std::collections::BTreeMap::new();
        for (text, color) in syntax_spans {
            let lines: Vec<&str> = text.split('\n').collect();
            if lines.len() > 1 {
                let mut line_idx = syntax_by_line.len();
                for part in &lines {
                    if !part.is_empty() {
                        syntax_by_line.entry(line_idx).or_default().push((part.to_string(), color));
                    }
                    line_idx += 1;
                }
            }
        }

        if syntax_by_line.is_empty() {
            for (i, _line) in cv.lines.iter().enumerate() {
                spans.push((_line.clone(), [1.0, 1.0, 1.0, 1.0]));
                if i + 1 < cv.lines.len() {
                    spans.push(("\n".to_string(), [1.0, 1.0, 1.0, 1.0]));
                }
            }
            return spans;
        }

        for (i, line) in cv.lines.iter().enumerate() {
            if let Some(line_spans) = syntax_by_line.get(&i) {
                for (text, color) in line_spans {
                    spans.push((text.clone(), *color));
                }
            } else {
                spans.push((line.clone(), [1.0, 1.0, 1.0, 1.0]));
            }

            if i + 1 < cv.lines.len() {
                spans.push(("\n".to_string(), [1.0, 1.0, 1.0, 1.0]));
            }
        }
        spans
    });

    let tab_labels = wc.editor_tabs.clone().unwrap_or_else(Vec::new);
    let tab_entries: Vec<TabEntry> = if tab_labels.is_empty() {
        vec![TabEntry { label: "No file open".to_string(), active: false }]
    } else {
        tab_labels
            .into_iter()
            .enumerate()
            .map(|(i, label)| TabEntry { label, active: i == 0 })
            .collect()
    };

    let breadcrumb_label = wc.editor_breadcrumb.clone().unwrap_or_else(String::new);

    EditorContentData {
        tab_entries,
        breadcrumb_label,
        editor_body_text,
        editor_spans,
        cursor_line,
        cursor_col,
        body_title,
    }
}
