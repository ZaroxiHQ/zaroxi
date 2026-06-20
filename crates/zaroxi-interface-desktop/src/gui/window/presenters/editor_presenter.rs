use std::collections::HashMap;

use zaroxi_core_editor_rope::Rope;
use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::chrome::TabEntry;
use zaroxi_core_platform_syntax::highlight::HighlightSpan;
use zaroxi_interface_theme::theme::SemanticColors;

use super::super::editor::EditorContentData;
use super::super::syntax_color;

const OVERSCAN_LINES: usize = 20;

pub fn shape_editor_content_plain(
    work_content: &Option<ShellWorkContent>,
    _sem: &SemanticColors,
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
) -> EditorContentData {
    let wc = match work_content {
        Some(w) => w,
        None => return EditorContentData::default(),
    };

    let editor_body = wc.editor_body.as_ref();
    let total_lines = rope
        .map(|r| r.line_count())
        .unwrap_or_else(|| editor_body.map(|cv| cv.lines.len()).unwrap_or(0));

    let (editor_body_text, used_visible_range) = if let Some(r) = rope {
        if let Some((vis_start, vis_end)) = visible_line_range {
            let start = vis_start.saturating_sub(OVERSCAN_LINES);
            let end = (vis_end + OVERSCAN_LINES).min(total_lines);
            let slice = r.visible_lines(start, end);
            let range = if slice.is_empty() && start >= total_lines {
                None
            } else {
                Some((start, end.min(total_lines)))
            };
            (slice, range)
        } else {
            // No visible range: use full text only for small files.
            // For large files (total_lines > LARGE_FILE_LINE_THRESHOLD),
            // construct a default window so we never materialize the
            // full 48+ MB document into a single String.
            const LARGE: usize = 1000;
            if total_lines > LARGE {
                let vis_lines = 50usize; // reasonable default viewport height
                let start = 0usize;
                let end = (vis_lines + OVERSCAN_LINES).min(total_lines);
                let slice = r.visible_lines(start, end);
                (slice, Some((start, end)))
            } else {
                (r.to_string(), None)
            }
        }
    } else {
        // No rope: fallback to ContentView.lines (backward compat)
        visible_line_range
            .map(|(vis_start, vis_end)| {
                let start = vis_start.saturating_sub(OVERSCAN_LINES);
                let end = (vis_end + OVERSCAN_LINES).min(total_lines);
                let slice = editor_body
                    .map(|cv| {
                        if start < cv.lines.len() {
                            cv.lines[start..end.min(cv.lines.len())].join("\n")
                        } else {
                            String::new()
                        }
                    })
                    .unwrap_or_default();
                let range = if slice.is_empty() { None } else { Some((start, end)) };
                (slice, range)
            })
            .unwrap_or_else(|| {
                let text = editor_body
                    .map(|cv| cv.lines.join("\n"))
                    .unwrap_or_else(|| "No file open".to_string());
                (text, None)
            })
    };

    let cursor_line = editor_body.map(|cv| cv.cursor_line).unwrap_or(0);
    let cursor_col = editor_body.map(|cv| cv.cursor_col).unwrap_or(0);

    let body_title = editor_body
        .map(|cv| if cv.title.is_empty() { cv.subtitle.clone() } else { cv.title.clone() })
        .unwrap_or_default();

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

    if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
        eprintln!(
            "ZAROXI_DEBUG_LARGE_FILE: shape_plain visible_range={:?} total={} text_bytes={} has_rope={}",
            used_visible_range,
            total_lines,
            editor_body_text.len(),
            rope.is_some(),
        );
    }

    EditorContentData {
        tab_entries,
        breadcrumb_label,
        editor_body_text,
        editor_spans: None,
        cursor_line,
        cursor_col,
        body_title,
        total_lines,
        visible_line_range: used_visible_range,
    }
}

pub fn shape_editor_content_incremental(
    work_content: &Option<ShellWorkContent>,
    sem: &SemanticColors,
    spans: &[HighlightSpan],
    line_syntax_cache: &mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
) -> EditorContentData {
    shape_editor_content_impl(
        work_content,
        sem,
        spans,
        true,
        Some(line_syntax_cache),
        per_line_hashes,
        cached_line_hashes,
        visible_line_range,
        rope,
    )
}

fn shape_editor_content_impl(
    work_content: &Option<ShellWorkContent>,
    sem: &SemanticColors,
    spans: &[HighlightSpan],
    incremental: bool,
    mut line_syntax_cache: Option<&mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>>,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
) -> EditorContentData {
    let wc = match work_content {
        Some(w) => w,
        None => return EditorContentData::default(),
    };

    let editor_body = wc.editor_body.as_ref();
    let total_lines = rope
        .map(|r| r.line_count())
        .unwrap_or_else(|| editor_body.map(|cv| cv.lines.len()).unwrap_or(0));

    let (editor_body_text, used_visible_range) = if let Some(r) = rope {
        if let Some((vis_start, vis_end)) = visible_line_range {
            let start = vis_start.saturating_sub(OVERSCAN_LINES);
            let end = (vis_end + OVERSCAN_LINES).min(total_lines);
            let slice = r.visible_lines(start, end);
            let range = if slice.is_empty() && start >= total_lines {
                None
            } else {
                Some((start, end.min(total_lines)))
            };
            (slice, range)
        } else {
            // No visible range: use full text only for small files.
            // For large files, construct a default window so we never
            // materialize the full document into a single String.
            const LARGE: usize = 1000;
            if total_lines > LARGE {
                let vis_lines = 50usize;
                let start = 0usize;
                let end = (vis_lines + OVERSCAN_LINES).min(total_lines);
                let slice = r.visible_lines(start, end);
                (slice, Some((start, end)))
            } else {
                (r.to_string(), None)
            }
        }
    } else {
        visible_line_range
            .map(|(vis_start, vis_end)| {
                let start = vis_start.saturating_sub(OVERSCAN_LINES);
                let end = (vis_end + OVERSCAN_LINES).min(total_lines);
                let slice = editor_body
                    .map(|cv| {
                        if start < cv.lines.len() {
                            cv.lines[start..end.min(cv.lines.len())].join("\n")
                        } else {
                            String::new()
                        }
                    })
                    .unwrap_or_default();
                let range = if slice.is_empty() { None } else { Some((start, end)) };
                (slice, range)
            })
            .unwrap_or_else(|| {
                let text = editor_body
                    .map(|cv| cv.lines.join("\n"))
                    .unwrap_or_else(|| "No file open".to_string());
                (text, None)
            })
    };

    let cursor_line = editor_body.map(|cv| cv.cursor_line).unwrap_or(0);
    let cursor_col = editor_body.map(|cv| cv.cursor_col).unwrap_or(0);

    let body_title = editor_body
        .map(|cv| if cv.title.is_empty() { cv.subtitle.clone() } else { cv.title.clone() })
        .unwrap_or_default();

    // Apply the latest stored highlight spans (full-document, byte offsets).
    // When a viewport window is active we colorize ONLY the visible (overscanned)
    // rows, rebasing the document-global spans into the window. This bounds the
    // per-frame styled-run vector (and its clone cost) to the viewport rather
    // than the whole document. When no spans are available (parse pending,
    // unsupported language, or large-file mode) we leave `editor_spans = None`
    // so the renderer falls back to plain text.
    let editor_spans: Option<Vec<(String, [f32; 4])>> = editor_body.and_then(|cv| {
        if cv.lines.is_empty() || spans.is_empty() {
            return None;
        }

        match used_visible_range {
            Some((start, end)) => {
                let n = cv.lines.len();
                let start = start.min(n);
                let end = end.min(n);
                if start >= end {
                    return None;
                }
                let window = &cv.lines[start..end];
                // Absolute byte offset of the first window line (lines joined by
                // '\n', matching the parsed source).
                let window_base_byte: usize = cv.lines[..start].iter().map(|l| l.len() + 1).sum();
                Some(syntax_color::colorize_window(window, window_base_byte, spans, sem))
            }
            None => {
                if incremental {
                    if let Some(ref mut cache) = line_syntax_cache {
                        return Some(syntax_color::colorize_source_incremental(
                            &cv.lines,
                            sem,
                            spans,
                            cache,
                            per_line_hashes,
                            cached_line_hashes,
                        ));
                    }
                }
                Some(syntax_color::colorize_source(&cv.lines, sem, spans))
            }
        }
    });

    if std::env::var("ZAROXI_DEBUG_EDITOR_SPANS").as_deref() == Ok("1") {
        eprintln!(
            "ZAROXI_DEBUG_EDITOR_SPANS: presenter spans_in={} editor_spans_segments={:?} total_lines={} used_visible_range={:?} body_bytes={}",
            spans.len(),
            editor_spans.as_ref().map(|s| s.len()),
            total_lines,
            used_visible_range,
            editor_body_text.len(),
        );
    }

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
        total_lines,
        visible_line_range: used_visible_range,
    }
}
