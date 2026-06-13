use std::collections::HashMap;

use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::chrome::TabEntry;
use zaroxi_core_platform_syntax::parser::ParserPool;
use zaroxi_interface_theme::theme::SemanticColors;

use super::super::editor::EditorContentData;
use super::super::syntax_color;

pub fn shape_editor_content(
    work_content: &Option<ShellWorkContent>,
    sem: &SemanticColors,
    parser_pool: &ParserPool,
) -> EditorContentData {
    shape_editor_content_impl(work_content, sem, parser_pool, false, None, &[], &[])
}

/// Build EditorContentData with incremental per-line syntax caching.
/// Only lines whose content hash differs from `cached_line_hashes` are
/// re-colored; other lines reuse spans from `line_syntax_cache`.
pub fn shape_editor_content_incremental(
    work_content: &Option<ShellWorkContent>,
    sem: &SemanticColors,
    parser_pool: &ParserPool,
    line_syntax_cache: &mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
) -> EditorContentData {
    shape_editor_content_impl(
        work_content,
        sem,
        parser_pool,
        true,
        Some(line_syntax_cache),
        per_line_hashes,
        cached_line_hashes,
    )
}

fn shape_editor_content_impl(
    work_content: &Option<ShellWorkContent>,
    sem: &SemanticColors,
    parser_pool: &ParserPool,
    incremental: bool,
    mut line_syntax_cache: Option<&mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>>,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
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
        if cv.lines.is_empty() {
            return Vec::new();
        }

        if incremental {
            if let Some(ref mut cache) = line_syntax_cache {
                return syntax_color::colorize_source_incremental(
                    &cv.lines,
                    sem,
                    parser_pool,
                    cache,
                    per_line_hashes,
                    cached_line_hashes,
                );
            }
        }

        syntax_color::colorize_source(&cv.lines, sem, parser_pool)
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
