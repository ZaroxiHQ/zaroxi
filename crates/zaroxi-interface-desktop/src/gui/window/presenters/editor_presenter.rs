use zaroxi_core_editor_rope::Rope;
use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::chrome::TabEntry;
use zaroxi_core_platform_syntax::highlight::HighlightSpan;
use zaroxi_interface_theme::theme::SemanticColors;

use super::super::editor::EditorContentData;
use super::super::syntax_color;

const OVERSCAN_LINES: usize = 20;

/// Tab width used for soft-wrap column math. Matches
/// `EditorBufferState::TAB_WIDTH` and the renderer/gutter so wrap columns line
/// up 1:1 with the caret's visual column (`caret_vis_col`).
const WRAP_TAB_WIDTH: usize = 4;

#[inline]
fn wrap_char_col_width(ch: char, col: usize) -> usize {
    if ch == '\t' { WRAP_TAB_WIDTH - (col % WRAP_TAB_WIDTH) } else { 1 }
}

/// A break may occur AFTER a whitespace char — the strongly preferred, prose
/// boundary. The trailing space stays on the previous row (monospace-friendly).
#[inline]
fn is_ws_break(ch: char) -> bool {
    ch == ' ' || ch == '\t'
}

/// … or after a "soft" structural separator common in code (paths, URLs,
/// snake_case, kebab-case). Breaking AFTER keeps the separator with the left
/// token, e.g. `a/b/c` → `a/` · `b/` · `c`. Deliberately excludes `.`/`,`/`;`
/// so decimals and abbreviations are not split at ugly spots.
#[inline]
fn is_soft_break(ch: char) -> bool {
    matches!(ch, '/' | '\\' | '-' | '_')
}

/// Compute soft-wrap row starts for one logical line.
///
/// Returns one `(char_start, col_start)` per visual row (row 0 is always
/// `(0, 0)`), where `char_start` is the char index in `line` where the row
/// begins and `col_start` is the tab-expanded, line-relative column where it
/// begins. `col_start` shares the exact coordinate space as
/// `EditorBufferState::caret_vis_col`, so caret projection can reuse this plan.
///
/// Policy (readable, not terminal-brutal):
///   1. break at the last whitespace that fits (whole word moves down),
///   2. else the last soft separator that fits,
///   3. else — only for a single token wider than the row — a character break
///      so text can never overflow the wrap width.
pub(crate) fn plan_line_wrap(line: &str, chars_per_row: usize) -> Vec<(usize, usize)> {
    let mut rows = vec![(0usize, 0usize)];
    if chars_per_row == 0 || line.is_empty() {
        return rows;
    }
    let chars: Vec<char> = line.chars().collect();
    let mut line_col = 0usize; // cumulative tab-expanded column within the line
    let mut row_start_col = 0usize; // line_col at the current row's start
    // Best break opportunities seen so far in the CURRENT row. Each stores the
    // char index at which the NEXT row would begin and the col at that point.
    let mut last_ws: Option<(usize, usize)> = None;
    let mut last_soft: Option<(usize, usize)> = None;

    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        let w = wrap_char_col_width(ch, line_col);
        let row_start_char = rows.last().map(|&(c, _)| c).unwrap_or(0);
        // Placing this char would overflow a non-empty row → break.
        if line_col + w - row_start_col > chars_per_row && line_col > row_start_col {
            let (brk_char, brk_col) = last_ws
                .filter(|&(c, _)| c > row_start_char)
                .or_else(|| last_soft.filter(|&(c, _)| c > row_start_char))
                .unwrap_or((i, line_col)); // char-level fallback
            rows.push((brk_char, brk_col));
            row_start_col = brk_col;
            last_ws = None;
            last_soft = None;
            // Re-scan the carried segment [brk_char, i) for break opportunities
            // that belong to the new row (e.g. a slash inside a carried token).
            let mut c = brk_char;
            let mut probe = brk_col;
            while c < i {
                probe += wrap_char_col_width(chars[c], probe);
                if is_ws_break(chars[c]) {
                    last_ws = Some((c + 1, probe));
                    last_soft = None;
                } else if is_soft_break(chars[c]) {
                    last_soft = Some((c + 1, probe));
                }
                c += 1;
            }
            // Re-test the current char against the freshly started row.
            continue;
        }
        line_col += w;
        if is_ws_break(ch) {
            last_ws = Some((i + 1, line_col));
            last_soft = None;
        } else if is_soft_break(ch) {
            last_soft = Some((i + 1, line_col));
        }
        i += 1;
    }
    rows
}

/// Project a caret's visual column onto its wrapped sub-row.
///
/// Given the caret's logical line text and tab-expanded column, returns
/// `(sub_row, col_within_row)` under the same policy as [`plan_line_wrap`], so
/// the caret lands on the exact row/column the renderer drew — even when rows
/// have unequal widths (word wrap) rather than a fixed `chars_per_row` stride.
pub(crate) fn wrapped_caret_subrow_col(
    line: &str,
    chars_per_row: usize,
    caret_vis_col: usize,
) -> (usize, usize) {
    if chars_per_row == 0 {
        return (0, caret_vis_col);
    }
    let rows = plan_line_wrap(line, chars_per_row);
    // Last row whose start column is <= the caret column.
    let mut sub_row = 0usize;
    for (idx, &(_, col_start)) in rows.iter().enumerate() {
        if col_start <= caret_vis_col {
            sub_row = idx;
        } else {
            break;
        }
    }
    let (_, col_start) = rows[sub_row];
    (sub_row, caret_vis_col.saturating_sub(col_start))
}

fn apply_wrap(
    raw_text: &str,
    raw_spans: Option<&syntax_color::ColoredSpans>,
    chars_per_row: usize,
    first_logical_line: usize,
    scroll_top: usize,
) -> (String, Option<syntax_color::ColoredSpans>, Vec<usize>, usize, usize) {
    if chars_per_row == 0 || raw_text.is_empty() {
        // Count via split('\n') (not `str::lines()`) so a trailing empty line is
        // preserved as its own logical line.
        let logical_count = raw_text.split('\n').count().max(1);
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
    // Split on '\n' (NOT `str::lines()`) so a trailing empty line — e.g. the
    // line created by Enter at EOF — is preserved as its own logical line and
    // gets a visual row. A trailing '\r' (CRLF) is trimmed per line.
    let logical_lines: Vec<&str> =
        raw_text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
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
        // Word-boundary row plan (shared with the caret projection so rows line
        // up exactly). Insert a soft '\n' at each row start after the first.
        let rows = plan_line_wrap(logical_line, chars_per_row);
        let mut next_row = 1usize;
        for (ci, ch) in logical_line.chars().enumerate() {
            if next_row < rows.len() && ci == rows[next_row].0 {
                out.push('\n');
                visual_to_logical.push(logical_idx);
                *total_visual += 1;
                next_row += 1;
            }
            out.push(ch);
        }
        // The final visual row of this logical line.
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
    // Emit one logical line's colored chars, inserting a soft '\n' at each row
    // start using the SAME plan as `wrap_text`, so the styled stream and the
    // plain fallback break at byte-identical positions.
    fn flush_line(
        line_chars: &[(char, [f32; 4])],
        chars_per_row: usize,
        logical_idx: usize,
        out: &mut Vec<(String, [f32; 4])>,
        visual_to_logical: &mut Vec<usize>,
        total_visual: &mut usize,
    ) {
        if line_chars.is_empty() {
            visual_to_logical.push(logical_idx);
            *total_visual += 1;
            return;
        }
        let line: String = line_chars.iter().map(|(c, _)| *c).collect();
        let rows = plan_line_wrap(&line, chars_per_row);
        let mut next_row = 1usize;
        for (ci, (ch, color)) in line_chars.iter().enumerate() {
            if next_row < rows.len() && ci == rows[next_row].0 {
                out.push(("\n".to_string(), *color));
                visual_to_logical.push(logical_idx);
                *total_visual += 1;
                next_row += 1;
            }
            out.push((ch.to_string(), *color));
        }
        visual_to_logical.push(logical_idx);
        *total_visual += 1;
    }

    let mut out: Vec<(String, [f32; 4])> = Vec::new();
    let mut logical_idx = first_logical_line;
    let mut line_chars: Vec<(char, [f32; 4])> = Vec::new();
    for (text, color) in spans {
        if text == "\n" {
            flush_line(
                &line_chars,
                chars_per_row,
                logical_idx,
                &mut out,
                visual_to_logical,
                total_visual,
            );
            out.push(("\n".to_string(), *color));
            line_chars.clear();
            logical_idx += 1;
            continue;
        }
        for ch in text.chars() {
            line_chars.push((ch, *color));
        }
    }
    // Flush the trailing logical line (no '\n' span follows it).
    flush_line(&line_chars, chars_per_row, logical_idx, &mut out, visual_to_logical, total_visual);
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
    line_syntax_cache: &mut syntax_color::LineSyntaxCache,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
    wrap_chars_per_row: usize,
    total_lines_override: Option<usize>,
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
        total_lines_override,
    )
}

// Editor shaping pipeline inputs (content, spans, caches, viewport, rope). This
// is the internal shared impl behind the two public wrappers; bundling would
// duplicate the same fields into a throwaway struct.
#[allow(clippy::too_many_arguments)]
fn shape_editor_content_impl(
    work_content: &Option<ShellWorkContent>,
    sem: &SemanticColors,
    spans: &[HighlightSpan],
    incremental: bool,
    mut line_syntax_cache: Option<&mut syntax_color::LineSyntaxCache>,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
    visible_line_range: Option<(usize, usize)>,
    rope: Option<&Rope>,
    wrap_chars_per_row: usize,
    total_lines_override: Option<usize>,
) -> EditorContentData {
    let wc = match work_content {
        Some(w) => w,
        None => return EditorContentData::default(),
    };

    let editor_body = wc.editor_body.as_ref();
    let total_lines = total_lines_override.unwrap_or_else(|| {
        rope.map(|r| r.line_count())
            .unwrap_or_else(|| editor_body.map(|cv| cv.lines.len()).unwrap_or(0))
    });

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

    // `Rope::visible_lines` strips the window's trailing newline, which drops the
    // trailing EMPTY line (e.g. the line created by pressing Enter at EOF). When
    // the window covers fewer text lines than its logical line span, restore the
    // missing trailing empty line(s) so the rendered line count matches the rope's
    // logical line range — the new line then gets a real visual row (caret +
    // gutter), instead of the caret having to be projected onto a phantom row.
    let editor_body_text = {
        let mut text = editor_body_text;
        if let Some((wstart, wend)) = used_visible_range {
            let expected = wend.saturating_sub(wstart);
            let have = text.split('\n').count();
            for _ in have..expected {
                text.push('\n');
            }
        }
        text
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

#[cfg(test)]
mod wrap_tests {
    use super::*;

    /// Split `wrap_text` output into visual rows for the single logical line.
    fn wrapped_rows(line: &str, cpr: usize) -> Vec<String> {
        let mut v2l = Vec::new();
        let mut tv = 0usize;
        let out = wrap_text(line, cpr, &mut v2l, 0, &mut tv);
        assert_eq!(v2l.len(), tv);
        out.split('\n').map(|s| s.to_string()).collect()
    }

    #[test]
    fn wraps_at_space_before_breaking_a_word() {
        let rows = wrapped_rows("hello world foo", 8);
        assert_eq!(rows, vec!["hello ", "world ", "foo"]);
        for r in &rows {
            assert!(r.chars().count() <= 8, "row {r:?} exceeds width");
        }
    }

    #[test]
    fn falls_back_to_char_break_only_for_oversized_token() {
        let rows = wrapped_rows("supercalifragilistic", 8);
        assert_eq!(rows, vec!["supercal", "ifragili", "stic"]);
        for r in &rows {
            assert!(r.chars().count() <= 8);
        }
    }

    #[test]
    fn breaks_after_soft_separators_in_paths() {
        let rows = wrapped_rows("aaa/bbb/ccc", 5);
        assert_eq!(rows, vec!["aaa/", "bbb/", "ccc"]);
    }

    #[test]
    fn snake_and_kebab_break_at_underscore_and_hyphen() {
        assert_eq!(wrapped_rows("alpha_beta_gamma", 7), vec!["alpha_", "beta_", "gamma"]);
        assert_eq!(wrapped_rows("one-two-three", 5), vec!["one-", "two-", "three"]);
    }

    #[test]
    fn short_line_is_not_wrapped() {
        assert_eq!(wrapped_rows("fn main() {}", 80), vec!["fn main() {}"]);
    }

    #[test]
    fn continuation_rows_stay_within_wrap_width() {
        let src = "The quick brown fox jumps over the lazy dog near the riverbank.";
        let cpr = 20;
        let rows = wrapped_rows(src, cpr);
        for r in &rows {
            assert!(r.chars().count() <= cpr, "row {r:?} exceeds {cpr}");
        }
        assert_eq!(rows.join(""), src);
    }

    #[test]
    fn caret_subrow_tracks_word_wrapped_rows() {
        let line = "hello world foo";
        assert_eq!(wrapped_caret_subrow_col(line, 8, 0), (0, 0));
        assert_eq!(wrapped_caret_subrow_col(line, 8, 6), (1, 0));
        assert_eq!(wrapped_caret_subrow_col(line, 8, 8), (1, 2));
        assert_eq!(wrapped_caret_subrow_col(line, 8, 12), (2, 0));
        assert_eq!(wrapped_caret_subrow_col(line, 8, 15), (2, 3));
    }

    #[test]
    fn caret_subrow_zero_when_not_wrapping() {
        assert_eq!(wrapped_caret_subrow_col("anything", 0, 5), (0, 5));
    }

    #[test]
    fn plain_and_styled_paths_break_identically() {
        let line = "let path = crate/module/name_value;";
        let cpr = 12;
        let mut v2l_plain = Vec::new();
        let mut tv_plain = 0usize;
        let _ = wrap_text(line, cpr, &mut v2l_plain, 0, &mut tv_plain);

        let spans = vec![(line.to_string(), [1.0, 1.0, 1.0, 1.0])];
        let mut v2l_styled = Vec::new();
        let mut tv_styled = 0usize;
        let _ = wrap_spans(&spans, cpr, &mut v2l_styled, 0, &mut tv_styled);

        assert_eq!(v2l_plain, v2l_styled);
        assert_eq!(tv_plain, tv_styled);
    }
}
