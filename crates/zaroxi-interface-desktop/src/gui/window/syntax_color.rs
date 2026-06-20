//! Apply syntax-highlight spans to editor source lines.
//!
//! Phase 1 syntax-highlighting source of truth:
//! - Tree-sitter parsing happens off the main thread in
//!   `app::background_parse::BackgroundParseWorker` using the language
//!   detected from the file path (`LanguageId::from_path`).
//! - The accepted `ParseResult` is stored on `GuiApp` as
//!   `latest_spans` (full-document byte-offset `HighlightSpan`s).
//! - This module is pure presentation: it maps those stored spans onto the
//!   editor's source lines.  It performs NO parsing and is language-agnostic
//!   (it never references a concrete `LanguageId`).
//!
//! Byte-offset contract: `lines.join("\n")` here is byte-identical to the
//! text the worker parsed (`EditorBufferState::to_string()`), because the
//! buffer is populated from the same lines joined by `"\n"`.  Therefore the
//! absolute byte offsets carried by `HighlightSpan` line up directly with the
//! per-line byte offsets computed below.

use std::collections::HashMap;

use zaroxi_core_platform_syntax::highlight::{Highlight, HighlightSpan};
use zaroxi_interface_theme::theme::SemanticColors;

fn highlight_color(h: Highlight, sem: &SemanticColors, default: [f32; 4]) -> [f32; 4] {
    let to_f32 = |c: &zaroxi_interface_theme::Color| -> [f32; 4] { [c.r, c.g, c.b, c.a] };
    match h {
        Highlight::Comment => to_f32(&sem.syntax_comment),
        Highlight::String => to_f32(&sem.syntax_string),
        Highlight::Keyword => to_f32(&sem.syntax_keyword),
        Highlight::Function => to_f32(&sem.syntax_function),
        Highlight::Type => to_f32(&sem.syntax_type),
        Highlight::Number => to_f32(&sem.syntax_number),
        Highlight::Constant => to_f32(&sem.syntax_constant),
        Highlight::Variable => to_f32(&sem.syntax_variable),
        Highlight::Operator => to_f32(&sem.syntax_operator),
        Highlight::Attribute => to_f32(&sem.syntax_attribute),
        Highlight::Property => to_f32(&sem.syntax_property),
        Highlight::Namespace => to_f32(&sem.syntax_namespace),
        Highlight::Plain => default,
    }
}

/// Colorize editor source lines by applying the supplied (full-document)
/// highlight spans.  Returns per-line colored spans as `(text, [r, g, b, a])`,
/// including `"\n"` separators between lines (matching the renderer's
/// span-emission contract).
pub fn colorize_source(
    lines: &[String],
    sem: &SemanticColors,
    spans: &[HighlightSpan],
) -> Vec<(String, [f32; 4])> {
    let source = lines.join("\n");
    let default_color: [f32; 4] =
        [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a];

    let mut result: Vec<(String, [f32; 4])> = Vec::new();
    let mut byte_offset = 0usize;

    for line in lines {
        extract_line_spans(line, byte_offset, &source, spans, &default_color, sem, &mut result);
        result.push(("\n".to_string(), default_color));
        byte_offset += line.len() + 1;
    }

    result
}

/// Colorize only the lines in a viewport window, rebasing the full-document
/// highlight spans into window-local byte coordinates.
///
/// `window_lines` are the visible (overscanned) lines; `window_base_byte` is the
/// absolute byte offset of the first window line within the full document. The
/// returned runs cover exactly `window_lines` (with `"\n"` separators), so the
/// renderer emits only viewport rows rather than the whole document — this is
/// the key to bounding per-frame text/clone cost on large files.
pub fn colorize_window(
    window_lines: &[String],
    window_base_byte: usize,
    spans: &[HighlightSpan],
    sem: &SemanticColors,
) -> Vec<(String, [f32; 4])> {
    // Window byte length (lines joined by '\n').
    let window_len: usize = if window_lines.is_empty() {
        0
    } else {
        window_lines.iter().map(|l| l.len()).sum::<usize>() + (window_lines.len() - 1)
    };
    let window_end_byte = window_base_byte + window_len;

    // Rebase + clip spans that intersect the window into window-local coords.
    let mut local_spans: Vec<HighlightSpan> = Vec::new();
    for s in spans {
        if s.end <= window_base_byte || s.start >= window_end_byte {
            continue;
        }
        let start = s.start.saturating_sub(window_base_byte);
        let end = (s.end - window_base_byte).min(window_len);
        if start < end {
            local_spans.push(HighlightSpan { start, end, highlight: s.highlight });
        }
    }

    colorize_source(window_lines, sem, &local_spans)
}

/// Extract colored runs for a single line given byte-offset highlight spans.
fn extract_line_spans(
    line: &str,
    byte_offset: usize,
    source: &str,
    spans: &[HighlightSpan],
    default_color: &[f32; 4],
    sem: &SemanticColors,
    out: &mut Vec<(String, [f32; 4])>,
) {
    let line_end = byte_offset + line.len();
    let line_spans: Vec<HighlightSpan> =
        spans.iter().filter(|s| s.start < line_end && s.end > byte_offset).cloned().collect();

    if line_spans.is_empty() {
        out.push((line.to_string(), *default_color));
    } else {
        let mut pos = byte_offset;
        for span in &line_spans {
            let seg_start = span.start.max(pos);
            let seg_end = span.end.min(line_end);
            if seg_start > pos {
                let before = &source[pos..seg_start];
                if !before.is_empty() {
                    out.push((before.to_string(), *default_color));
                }
            }
            if seg_start < seg_end && seg_start >= pos {
                let text = &source[seg_start..seg_end];
                if !text.is_empty() {
                    out.push((
                        text.to_string(),
                        highlight_color(span.highlight, sem, *default_color),
                    ));
                }
            }
            pos = seg_end.max(pos);
        }
        if pos < line_end {
            let after = &source[pos..line_end];
            if !after.is_empty() {
                out.push((after.to_string(), *default_color));
            }
        }
    }
}

/// Colorize the full source from the supplied spans, reusing a per-line span
/// cache.  Only lines whose `per_line_hashes[i] != cached_line_hashes[i]` are
/// re-extracted; unchanged lines reuse their cached colored spans.
///
/// Note: when new spans arrive from the background worker the caller clears
/// `line_syntax_cache`, so a stale cache never masks fresh highlight colors.
pub fn colorize_source_incremental(
    lines: &[String],
    sem: &SemanticColors,
    spans: &[HighlightSpan],
    line_syntax_cache: &mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
) -> Vec<(String, [f32; 4])> {
    let default_color: [f32; 4] =
        [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a];

    let n = lines.len();
    let mut result: Vec<(String, [f32; 4])> = Vec::with_capacity(n * 6);

    let source = lines.join("\n");

    let mut byte_offset = 0usize;
    for i in 0..n {
        let line = &lines[i];
        let cur_hash = per_line_hashes.get(i).copied().unwrap_or(0);
        let prev_hash = cached_line_hashes.get(i).copied().unwrap_or(0);

        if cur_hash == prev_hash && prev_hash != 0 {
            // Reuse cached spans
            let cache_key = (i, cur_hash);
            if let Some(cached_spans) = line_syntax_cache.get(&cache_key) {
                result.extend(cached_spans.clone());
            } else {
                let mut line_out = Vec::new();
                extract_line_spans(
                    line,
                    byte_offset,
                    &source,
                    spans,
                    &default_color,
                    sem,
                    &mut line_out,
                );
                line_syntax_cache.insert(cache_key, line_out.clone());
                result.extend(line_out);
            }
        } else {
            let mut line_out: Vec<(String, [f32; 4])> = Vec::new();
            extract_line_spans(
                line,
                byte_offset,
                &source,
                spans,
                &default_color,
                sem,
                &mut line_out,
            );
            let cache_key = (i, cur_hash);
            line_syntax_cache.insert(cache_key, line_out.clone());
            result.extend(line_out);
        }

        if i + 1 < n {
            result.push(("\n".to_string(), default_color));
        }
        byte_offset += line.len() + 1;
    }

    result
}
