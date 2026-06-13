use std::collections::HashMap;

use zaroxi_core_platform_syntax::highlight::{Highlight, HighlightEngine, HighlightSpan};
use zaroxi_core_platform_syntax::language::LanguageId;
use zaroxi_core_platform_syntax::parser::ParserPool;
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

/// Colorize editor source lines using tree-sitter syntax highlighting.
/// Returns per-line colored spans as `(text, [r, g, b, a])`.
pub fn colorize_source(
    lines: &[String],
    sem: &SemanticColors,
    pool: &ParserPool,
) -> Vec<(String, [f32; 4])> {
    let source = lines.join("\n");
    let default_color: [f32; 4] =
        [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a];

    let mut parser = match pool.acquire(&LanguageId::Rust) {
        Some(p) => p,
        None => return lines.iter().map(|l| (l.clone(), default_color)).collect(),
    };

    let tree = match parser.parse(&source, None) {
        Some(t) => t,
        None => return lines.iter().map(|l| (l.clone(), default_color)).collect(),
    };

    let engine = HighlightEngine::new();
    let spans = engine.highlight(LanguageId::Rust, &source, &tree).unwrap_or_default();

    let mut result: Vec<(String, [f32; 4])> = Vec::new();
    let mut byte_offset = 0usize;

    for line in lines {
        extract_line_spans(line, byte_offset, &source, &spans, &default_color, sem, &mut result);
        result.push(("\n".to_string(), default_color));
        byte_offset += line.len() + 1;
    }

    drop(tree);
    pool.release(&LanguageId::Rust, parser);

    result
}

/// Parse and highlight a single line, returning its colored spans (without trailing newline).
/// Parses the line in isolation; for best results the full-file path is preferred when
/// available.  Available for external incremental use.
#[allow(dead_code)]
pub fn colorize_single_line(
    line: &str,
    sem: &SemanticColors,
    pool: &ParserPool,
) -> Vec<(String, [f32; 4])> {
    let default_color: [f32; 4] =
        [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a];

    if line.is_empty() {
        return vec![(String::new(), default_color)];
    }

    let mut parser = match pool.acquire(&LanguageId::Rust) {
        Some(p) => p,
        None => return vec![(line.to_string(), default_color)],
    };

    let tree = match parser.parse(line, None) {
        Some(t) => t,
        None => return vec![(line.to_string(), default_color)],
    };

    let engine = HighlightEngine::new();
    let spans = engine.highlight(LanguageId::Rust, line, &tree).unwrap_or_default();

    let mut result: Vec<(String, [f32; 4])> = Vec::new();
    extract_line_spans(line, 0, line, &spans, &default_color, sem, &mut result);

    drop(tree);
    pool.release(&LanguageId::Rust, parser);

    result
}

/// Extract colored spans for a single line given the full-source highlight spans
/// and the line's byte offset into the source.
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

/// Colorize the full source and update a per-line span cache incrementally.
/// Only lines whose `per_line_hashes[i] != cached_line_hashes[i]` are
/// re-extracted; unchanged lines are left alone in the supplied cache.
/// `line_syntax_cache` is updated in-place with new spans for changed lines.
pub fn colorize_source_incremental(
    lines: &[String],
    sem: &SemanticColors,
    pool: &ParserPool,
    line_syntax_cache: &mut HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    per_line_hashes: &[u64],
    cached_line_hashes: &[u64],
) -> Vec<(String, [f32; 4])> {
    let default_color: [f32; 4] =
        [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a];

    let n = lines.len();
    let mut result: Vec<(String, [f32; 4])> = Vec::with_capacity(n * 6);

    // Parse and highlight the full source once.
    let source = lines.join("\n");
    let spans: Vec<HighlightSpan> = pool.acquire(&LanguageId::Rust).map_or(vec![], |mut parser| {
        let tree = parser.parse(&source, None);
        let spans = tree.as_ref().map_or_else(Vec::new, |t| {
            let engine = HighlightEngine::new();
            engine.highlight(LanguageId::Rust, &source, t).unwrap_or_default()
        });
        pool.release(&LanguageId::Rust, parser);
        spans
    });

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
                // Cache miss — extract from highlight
                let mut line_out = Vec::new();
                extract_line_spans(
                    line,
                    byte_offset,
                    &source,
                    &spans,
                    &default_color,
                    sem,
                    &mut line_out,
                );
                line_syntax_cache.insert(cache_key, line_out.clone());
                result.extend(line_out);
            }
        } else {
            // Changed line — extract from highlight and update cache
            let mut line_out: Vec<(String, [f32; 4])> = Vec::new();
            extract_line_spans(
                line,
                byte_offset,
                &source,
                &spans,
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
