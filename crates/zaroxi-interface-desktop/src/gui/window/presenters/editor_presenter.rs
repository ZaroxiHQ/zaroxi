use std::collections::HashMap;

use zaroxi_core_editor_rope::Rope;
use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::chrome::TabEntry;
use zaroxi_core_platform_syntax::highlight::HighlightSpan;
use zaroxi_interface_theme::theme::SemanticColors;

use super::super::editor::EditorContentData;
use super::super::syntax_color;

const OVERSCAN_LINES: usize = 20;

fn apply_wrap(
    raw_text: &str,
    raw_spans: Option<&Vec<(String, [f32; 4])>>,
    chars_per_row: usize,
    first_logical_line: usize,
    scroll_top: usize,
) -> (String, Option<Vec<(String, [f32; 4])>>, Vec<usize>, usize, usize) {
    if chars_per_row == 0 || raw_text.is_empty() {
        let logical_count = raw_text.lines().count().max(1);
        let v2l: Vec<usize> = (first_logical_line..first_logical_line + logical_count).collect();
        return (
            raw_text.to_string(),
            raw_spans.cloned(),
            v2l,
            logical_count,
            scroll_top.saturating_sub(first_logical_line),
        );
    }
    let mut visual_to_logical: Vec<usize> = Vec::new();
    let mut total_visual: usize = 0;

    let wrapped_text = wrap_text(
        raw_text,
        chars_per_row,
        &mut visual_to_logical,
        first_logical_line,
        &mut total_visual,
    );

    let wrapped_spans = raw_spans.map(|sp| {
        let mut v2l2: Vec<usize> = Vec::new();
        let mut tv2: usize = 0;
        wrap_spans(sp, chars_per_row, &mut v2l2, first_logical_line, &mut tv2)
    });

    let visual_offset = visual_to_logical.iter().take_while(|&&ll| ll < scroll_top).count();

    (wrapped_text, wrapped_spans, visual_to_logical, total_visual, visual_offset)
}

fn wrap_text(
    raw_text: &str,
    chars_per_row: usize,
    visual_to_logical: &mut Vec<usize>,
    first_logical_line: usize,
    total_visual: &mut usize,
) -> String {
    if chars_per_row == 0 {
        for (i, _) in raw_text.lines().enumerate() {
            visual_to_logical.push(first_logical_line + i);
        }
        *total_visual += raw_text.lines().count().max(1);
        return raw_text.to_string();
    }
    let logical_lines: Vec<&str> = raw_text.lines().collect();
    let mut out = String::with_capacity(raw_text.len() + raw_text.len() / chars_per_row);
    for (li, logical_line) in logical_lines.iter().enumerate() {
        if li > 0 {
            out.push('\n');
        }
        let logical_idx = first_logical_line + li;
        if logical_line.is_empty() {
            visual_to_logical.push(logical_idx);
            *total_visual += 1;
            continue;
        }
        let mut col: usize = 0;
        for ch in logical_line.chars() {
            let ch_w = if ch == '\t' { 4 - (col % 4) } else { 1 };
            if col + ch_w > chars_per_row && col > 0 {
                out.push('\n');
                visual_to_logical.push(logical_idx);
                *total_visual += 1;
                col = 0;
            }
            out.push(ch);
            if ch == '\t' {
                col += 4 - (col % 4);
            } else {
                col += 1;
            }
        }
        visual_to_logical.push(logical_idx);
        *total_visual += 1;
    }
    if logical_lines.is_empty() {
        visual_to_logical.push(first_logical_line);
        *total_visual += 1;
    }
    out
}

fn wrap_spans(
    spans: &[(String, [f32; 4])],
    chars_per_row: usize,
    visual_to_logical: &mut Vec<usize>,
    first_logical_line: usize,
    total_visual: &mut usize,
) -> Vec<(String, [f32; 4])> {
    if chars_per_row == 0 || spans.is_empty() {
        let logical_count = spans.iter().filter(|(t, _)| t == "\n").count() + 1;
        for i in 0..logical_count {
            visual_to_logical.push(first_logical_line + i);
        }
        *total_visual += logical_count;
        return spans.to_vec();
    }
    let mut out: Vec<(String, [f32; 4])> = Vec::new();
    let mut col: usize = 0;
    let mut logical_idx = first_logical_line;
    for (text, color) in spans {
        if text == "\n" {
            visual_to_logical.push(logical_idx);
            *total_visual += 1;
            out.push(("\n".to_string(), *color));
            logical_idx += 1;
            col = 0;
            continue;
        }
        for ch in text.chars() {
            let ch_w = if ch == '\t' { 4 - (col % 4) } else { 1 };
            if col + ch_w > chars_per_row && col > 0 {
                out.push(("\n".to_string(), *color));
                visual_to_logical.push(logical_idx);
                *total_visual += 1;
                col = 0;
            }
            out.push((ch.to_string(), *color));
            if ch == '\t' {
                col += 4 - (col % 4);
            } else {
                col += 1;
            }
        }
    }
    if out.is_empty() {
        visual_to_logical.push(first_logical_line);
        *total_visual += 1;
    }
    out
}

#[allow(dead_code)]
pub fn shape_editor_content_plain(
    work_content: &Option<ShellWorkContent>,
    _sem: &SemanticColors,
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
    doc_buffer: Option<&zaroxi_core_editor_largefile::DocumentBuffer>,
    wrap_chars_per_row: usize,
) -> EditorContentData {
    let wc = match work_content {
        Some(w) => w,
        None => return EditorContentData::default(),
    };

    let editor_body = wc.editor_body.as_ref();

    let total_lines: usize;
    let editor_body_text: String;
    let used_visible_range: Option<(usize, usize)>;

    // Priority 1: doc_buffers DocumentBuffer (modern path)
    if let Some(db) = doc_buffer {
        if std::env::var("ZAROXI_DEBUG_RENDER_SOURCE").as_deref() == Ok("1") {
            let total = db.total_lines();
            eprintln!(
                "RENDER_SOURCE: doc_buffers total_lines={total} vis_range={visible_line_range:?}"
            );
        }
        let total = db.total_lines();
        total_lines = total;
        if let Some((vis_start, vis_end)) = visible_line_range {
            let start = vis_start.saturating_sub(OVERSCAN_LINES);
            let end = (vis_end + OVERSCAN_LINES).min(total);
            let lines: Vec<String> =
                db.lines_in_range(start, end).into_iter().map(|(_, s)| s).collect();
            editor_body_text = lines.join("\n");
            used_visible_range = if editor_body_text.is_empty() && start >= total {
                None
            } else {
                Some((start, end.min(total)))
            };
        } else {
            let vis_lines = 50usize;
            let start = 0usize;
            let end = (vis_lines + OVERSCAN_LINES).min(total);
            let lines: Vec<String> =
                db.lines_in_range(start, end).into_iter().map(|(_, s)| s).collect();
            editor_body_text = lines.join("\n");
            used_visible_range = Some((start, end));
        }
    } else {
        if std::env::var("ZAROXI_DEBUG_RENDER_SOURCE").as_deref() == Ok("1") {
            let source = if rope.is_some() { "rope" } else { "editor_body" };
            eprintln!(
                "RENDER_SOURCE: {source} (doc_buffers miss) vis_range={visible_line_range:?}"
            );
        }
        total_lines = rope
            .map(|r| r.line_count())
            .unwrap_or_else(|| editor_body.map(|cv| cv.lines.len()).unwrap_or(0));

        let (text, range) = if let Some(r) = rope {
            if let Some((vis_start, vis_end)) = visible_line_range {
                let start = vis_start.saturating_sub(OVERSCAN_LINES);
                let end = (vis_end + OVERSCAN_LINES).min(total_lines);
                let slice = r.visible_lines(start, end);
                let rng = if slice.is_empty() && start >= total_lines {
                    None
                } else {
                    Some((start, end.min(total_lines)))
                };
                (slice, rng)
            } else {
                const LARGE: usize = 1000;
                if total_lines > LARGE {
                    let vis_lines = 50usize;
                    let start = 0usize;
                    let end = (vis_lines + OVERSCAN_LINES).min(total_lines);
                    (r.visible_lines(start, end), Some((start, end)))
                } else {
                    (r.to_string(), None)
                }
            }
        } else {
            if let Some((vis_start, vis_end)) = visible_line_range {
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
                let rng = if slice.is_empty() { None } else { Some((start, end)) };
                (slice, rng)
            } else {
                let t = editor_body
                    .map(|cv| cv.lines.join("\n"))
                    .unwrap_or_else(|| "No file open".to_string());
                (t, None)
            }
        };
        editor_body_text = text;
        used_visible_range = range;
    }

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

    let scroll_top = visible_line_range.map(|(vis_start, _)| vis_start).unwrap_or(0);
    let (wrapped_text, wrapped_spans, visual_to_logical, total_visual_lines, wrap_visual_offset) =
        apply_wrap(
            &editor_body_text,
            None,
            wrap_chars_per_row,
            used_visible_range.map(|(s, _)| s).unwrap_or(0),
            scroll_top,
        );

    EditorContentData {
        tab_entries,
        breadcrumb_label,
        editor_body_text: wrapped_text,
        editor_spans: wrapped_spans,
        cursor_line,
        cursor_col,
        body_title,
        total_lines,
        visible_line_range: used_visible_range,
        visual_to_logical,
        total_visual_lines,
        chars_per_row: wrap_chars_per_row,
        wrap_visual_offset,
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
    wrap_chars_per_row: usize,
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
        wrap_chars_per_row,
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
    wrap_chars_per_row: usize,
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
    //
    // Spans MUST be derived from the SAME text snapshot that is rendered
    // (`editor_body_text`) AND that the background worker parsed
    // (`EditorBufferState::to_string()`). Otherwise the document-global byte
    // offsets carried by `spans` land on the wrong characters and paint broad
    // wrong-color blocks. When a Rope is present it is that single source of
    // truth, so the visible window text and its base byte offset are taken from
    // the Rope here — NOT from `cv.lines`, which is tab-expanded after edits and
    // would drift from the parsed byte offsets. `cv.lines` is used only as a
    // fallback when no Rope is available (e.g. unit tests).
    //
    // When a viewport window is active we colorize ONLY the visible (overscanned)
    // rows, rebasing the document-global spans into the window so the per-frame
    // styled-run vector is bounded to the viewport. When no spans are available
    // (parse pending, unsupported language, or large-file mode) we leave
    // `editor_spans = None` so the renderer falls back to plain text rather than
    // corrupted color.
    let editor_spans: Option<Vec<(String, [f32; 4])>> = if spans.is_empty() {
        None
    } else if let Some(r) = rope {
        // Reuse the exact text that will be rendered (`editor_body_text`) as the
        // span source so the colored runs and the plain fallback are byte-for-byte
        // the same window the parser saw.
        match used_visible_range {
            Some((start, end)) => {
                if start >= end || editor_body_text.is_empty() {
                    None
                } else {
                    let window_lines: Vec<String> =
                        editor_body_text.split('\n').map(|s| s.to_string()).collect();
                    // Absolute byte offset of the first window line within the
                    // full document (== `rope.to_string()`): the bytes of lines
                    // [0..start] joined by '\n'. `visible_lines(0, start)` drops
                    // the trailing newline, so add 1 for the '\n' preceding line
                    // `start` (only when start > 0).
                    let window_base_byte =
                        if start == 0 { 0 } else { r.visible_lines(0, start).len() + 1 };
                    Some(syntax_color::colorize_window(&window_lines, window_base_byte, spans, sem))
                }
            }
            None => {
                // Full (small-file) colorize from the same rendered snapshot. The
                // prepare_editor_data cache bounds this to content/spans changes,
                // so a non-incremental pass over a <LARGE document is cheap and
                // avoids per-line-hash drift between cv.lines and the rope.
                if editor_body_text.is_empty() {
                    None
                } else {
                    let all_lines: Vec<String> =
                        editor_body_text.split('\n').map(|s| s.to_string()).collect();
                    Some(syntax_color::colorize_source(&all_lines, sem, spans))
                }
            }
        }
    } else {
        // No Rope (tests / detached presenter): fall back to the editor_body
        // lines snapshot. This preserves the incremental per-line cache path.
        editor_body.and_then(|cv| {
            if cv.lines.is_empty() {
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
                    // Absolute byte offset of the first window line (lines joined
                    // by '\n', matching the parsed source).
                    let window_base_byte: usize =
                        cv.lines[..start].iter().map(|l| l.len() + 1).sum();
                    Some(syntax_color::colorize_window(window, window_base_byte, spans, sem))
                }
                None => {
                    if incremental && let Some(ref mut cache) = line_syntax_cache {
                        return Some(syntax_color::colorize_source_incremental(
                            &cv.lines,
                            sem,
                            spans,
                            cache,
                            per_line_hashes,
                            cached_line_hashes,
                        ));
                    }
                    Some(syntax_color::colorize_source(&cv.lines, sem, spans))
                }
            }
        })
    };

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

    let scroll_top = visible_line_range.map(|(vis_start, _)| vis_start).unwrap_or(0);
    let (wrapped_text, wrapped_spans, visual_to_logical, total_visual_lines, wrap_visual_offset) =
        apply_wrap(
            &editor_body_text,
            editor_spans.as_ref(),
            wrap_chars_per_row,
            used_visible_range.map(|(s, _)| s).unwrap_or(0),
            scroll_top,
        );

    EditorContentData {
        tab_entries,
        breadcrumb_label,
        editor_body_text: wrapped_text,
        editor_spans: wrapped_spans,
        cursor_line,
        cursor_col,
        body_title,
        total_lines,
        visible_line_range: used_visible_range,
        visual_to_logical,
        total_visual_lines,
        chars_per_row: wrap_chars_per_row,
        wrap_visual_offset,
    }
}
