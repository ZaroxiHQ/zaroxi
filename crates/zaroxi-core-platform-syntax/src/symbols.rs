//! Structural symbol extraction from syntax highlight spans.
//!
//! The highlighter ([`crate::highlight`]) produces byte-offset [`HighlightSpan`]s
//! classifying every token in a document. A useful subset of those tokens are
//! *structural* — functions, types, and namespaces — which together form a
//! lightweight, line-based map of a file's shape. [`extract_symbols`] turns the
//! raw spans into [`DocumentSymbol`]s placed on source **lines**, ready for the
//! semantic minimap and future navigation widgets.
//!
//! This path intentionally reuses the existing highlight query rather than
//! introducing a separate tree-sitter `tags` query: it needs no new grammar
//! resources, updates incrementally with every reparse (spans are the input),
//! and is cheap (`O(spans)`). The trade-off is that it reflects *occurrences*
//! of structural tokens (definitions and references) rather than definitions
//! only; symbols are deduplicated per `(line, kind)` so the output stays bounded
//! and reads cleanly as a structural density map.

use crate::highlight::{Highlight, HighlightSpan};
use std::collections::HashSet;

/// Whether structural-symbol diagnostics are enabled (`ZAROXI_SYMBOLS_TRACE=1`).
///
/// When set, [`extract_symbols`] prints a one-line summary of the input span
/// count, the extracted symbol count, and the per-kind breakdown — the trace
/// used to observe extraction volume and update behavior.
pub fn symbols_trace_enabled() -> bool {
    std::env::var("ZAROXI_SYMBOLS_TRACE").as_deref() == Ok("1")
}

/// The structural classification of a [`DocumentSymbol`].
///
/// Deliberately small and UI-agnostic: it maps the structural subset of
/// [`Highlight`] categories and leaves any presentation mapping (glyph, color)
/// to the consuming widget layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// A function or method (definition or call site).
    Function,
    /// A type, trait, or type-like name.
    Type,
    /// A namespace, module, or import.
    Namespace,
}

/// A structural symbol resolved to a source line.
///
/// `line` is 0-based. `byte_start`/`byte_end` are the symbol's byte range in the
/// document (the same coordinate space as [`HighlightSpan`]), and `name` is the
/// source text of that range (empty when the range is not on a UTF-8 boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    /// 0-based source line the symbol begins on.
    pub line: usize,
    /// Structural classification.
    pub kind: SymbolKind,
    /// The symbol's source text (its identifier), if extractable.
    pub name: String,
    /// Inclusive start byte offset in the document.
    pub byte_start: usize,
    /// Exclusive end byte offset in the document.
    pub byte_end: usize,
}

/// Classify a [`Highlight`] into a structural [`SymbolKind`], or `None` if the
/// highlight is not structural (keywords, strings, comments, operators, …).
fn symbol_kind_of(highlight: Highlight) -> Option<SymbolKind> {
    match highlight {
        Highlight::Function => Some(SymbolKind::Function),
        Highlight::Type => Some(SymbolKind::Type),
        Highlight::Namespace => Some(SymbolKind::Namespace),
        _ => None,
    }
}

/// Map a byte offset to its 0-based line via binary search over `line_starts`
/// (the byte offsets of each line start, e.g. from `LineIndex::line_starts`).
fn line_of_byte(line_starts: &[usize], byte: usize) -> usize {
    match line_starts.binary_search(&byte) {
        Ok(line) => line,
        Err(insert) => insert.saturating_sub(1),
    }
}

/// Extract the byte range `[start, end)` from `source` as an owned `String`,
/// returning an empty string if the range is invalid or not on char boundaries.
fn safe_slice(source: &str, start: usize, end: usize) -> String {
    if start >= end || end > source.len() {
        return String::new();
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return String::new();
    }
    source[start..end].to_string()
}

/// Extract structural [`DocumentSymbol`]s from highlight `spans`.
///
/// `source` is the full document text the spans were produced against, and
/// `line_starts` are the byte offsets of each line start (see
/// `zaroxi_core_editor_rope::LineIndex`). The byte-offset contract between the
/// three must hold (they all describe the same document).
///
/// The result is sorted by line and deduplicated so each `(line, kind)` pair
/// appears at most once, keeping the output bounded (`<= lines * kinds`) and
/// suitable for direct minimap rendering. Cost is `O(spans)` plus the final
/// sort — no full document rescan.
pub fn extract_symbols(
    spans: &[HighlightSpan],
    source: &str,
    line_starts: &[usize],
) -> Vec<DocumentSymbol> {
    let mut out: Vec<DocumentSymbol> = Vec::new();
    let mut seen: HashSet<(usize, SymbolKind)> = HashSet::new();

    for span in spans {
        let Some(kind) = symbol_kind_of(span.highlight) else {
            continue;
        };
        let line = line_of_byte(line_starts, span.start);
        // Keep the first (leftmost — spans arrive start-sorted) occurrence of
        // each structural kind on a given line.
        if !seen.insert((line, kind)) {
            continue;
        }
        out.push(DocumentSymbol {
            line,
            kind,
            name: safe_slice(source, span.start, span.end),
            byte_start: span.start,
            byte_end: span.end,
        });
    }

    out.sort_by_key(|s| s.line);

    if symbols_trace_enabled() {
        let (mut functions, mut types, mut namespaces) = (0usize, 0usize, 0usize);
        for s in &out {
            match s.kind {
                SymbolKind::Function => functions += 1,
                SymbolKind::Type => types += 1,
                SymbolKind::Namespace => namespaces += 1,
            }
        }
        eprintln!(
            "ZAROXI_SYMBOLS_TRACE: spans={} symbols={} (fn={} type={} ns={}) lines={}",
            spans.len(),
            out.len(),
            functions,
            types,
            namespaces,
            line_starts.len(),
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlight::Highlight;

    fn span(start: usize, end: usize, h: Highlight) -> HighlightSpan {
        HighlightSpan { start, end, highlight: h }
    }

    #[test]
    fn maps_structural_highlights_to_lines() {
        // "fn run() {}\ntype Foo = u8;\nuse std::io;"
        let source = "fn run() {}\ntype Foo = u8;\nuse std::io;";
        let line_starts = [0usize, 12, 27];
        let fn_at = source.find("run").unwrap();
        let ty_at = source.find("Foo").unwrap();
        let ns_at = source.find("std").unwrap();
        let spans = vec![
            span(fn_at, fn_at + 3, Highlight::Function),
            span(ty_at, ty_at + 3, Highlight::Type),
            span(ns_at, ns_at + 3, Highlight::Namespace),
        ];

        let symbols = extract_symbols(&spans, source, &line_starts);
        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].line, 0);
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[0].name, "run");
        assert_eq!(symbols[1].line, 1);
        assert_eq!(symbols[1].kind, SymbolKind::Type);
        assert_eq!(symbols[1].name, "Foo");
        assert_eq!(symbols[2].line, 2);
        assert_eq!(symbols[2].kind, SymbolKind::Namespace);
        assert_eq!(symbols[2].name, "std");
    }

    #[test]
    fn ignores_non_structural_highlights() {
        let source = "let x = \"hi\"; // c";
        let line_starts = [0usize];
        let spans = vec![
            span(0, 3, Highlight::Keyword),
            span(8, 12, Highlight::String),
            span(14, 18, Highlight::Comment),
        ];
        assert!(extract_symbols(&spans, source, &line_starts).is_empty());
    }

    #[test]
    fn dedups_same_kind_on_a_line() {
        // Two function spans on the same line collapse to one symbol; a type on
        // the same line is kept (different kind).
        let source = "a() b() C";
        let line_starts = [0usize];
        let spans = vec![
            span(0, 1, Highlight::Function),
            span(4, 5, Highlight::Function),
            span(8, 9, Highlight::Type),
        ];
        let symbols = extract_symbols(&spans, source, &line_starts);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols.iter().filter(|s| s.kind == SymbolKind::Function).count(), 1);
        assert_eq!(symbols.iter().filter(|s| s.kind == SymbolKind::Type).count(), 1);
        // The leftmost function occurrence is kept.
        let f = symbols.iter().find(|s| s.kind == SymbolKind::Function).unwrap();
        assert_eq!(f.byte_start, 0);
    }

    #[test]
    fn multibyte_name_extraction_is_boundary_safe() {
        // A span whose end is not on a char boundary yields an empty name rather
        // than panicking; a valid multibyte span yields the right text.
        let source = "fn café() {}";
        let line_starts = [0usize];
        let cafe_start = source.find("café").unwrap();
        let cafe_end = cafe_start + "café".len();
        let good = vec![span(cafe_start, cafe_end, Highlight::Function)];
        let symbols = extract_symbols(&good, source, &line_starts);
        assert_eq!(symbols[0].name, "café");

        // End inside the 'é' multibyte sequence -> empty (no panic).
        let bad = vec![span(cafe_start, cafe_end - 1, Highlight::Function)];
        let symbols = extract_symbols(&bad, source, &line_starts);
        assert_eq!(symbols[0].name, "");
    }

    #[test]
    fn empty_spans_yield_no_symbols() {
        assert!(extract_symbols(&[], "anything", &[0]).is_empty());
    }

    #[test]
    fn scales_to_a_large_document() {
        // Synthesize a ~50k-line document with one function token per line, plus
        // a duplicate function token on each line that must be deduped. This
        // exercises the byte→line mapping and dedup at large-file scale and
        // confirms the output stays bounded (one symbol per (line, kind)).
        const LINES: usize = 50_000;
        let mut source = String::with_capacity(LINES * 12);
        for i in 0..LINES {
            if i > 0 {
                source.push('\n');
            }
            source.push_str("fn a() b()"); // two function-like tokens per line
        }
        let line_starts: Vec<usize> = {
            let mut v = Vec::with_capacity(LINES);
            let mut off = 0usize;
            for i in 0..LINES {
                if i > 0 {
                    off += 1;
                }
                v.push(off);
                off += "fn a() b()".len();
            }
            v
        };
        let mut spans = Vec::with_capacity(LINES * 2);
        for &start in &line_starts {
            let a = start + 3; // "a"
            let b = start + 7; // "b"
            spans.push(span(a, a + 1, Highlight::Function));
            spans.push(span(b, b + 1, Highlight::Function));
        }

        let symbols = extract_symbols(&spans, &source, &line_starts);
        // Deduped to exactly one function symbol per line.
        assert_eq!(symbols.len(), LINES);
        assert_eq!(symbols.first().unwrap().line, 0);
        assert_eq!(symbols.last().unwrap().line, LINES - 1);
        // Sorted by line.
        assert!(symbols.windows(2).all(|w| w[0].line <= w[1].line));
    }
}
