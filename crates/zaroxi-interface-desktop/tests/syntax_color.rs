//! Integration tests for applying stored highlight spans to editor source
//! lines (Phase 1 syntax highlighting presentation layer).

use std::collections::HashMap;

use zaroxi_core_platform_syntax::highlight::{Highlight, HighlightEngine, HighlightSpan};
use zaroxi_core_platform_syntax::language::LanguageId;
use zaroxi_core_platform_syntax::parser::ParserPool;
use zaroxi_interface_desktop::gui::window::syntax_color::{
    colorize_source, colorize_source_incremental, colorize_window, coverage_end,
    remap_good_spans_across_edit, validate_spans_for_source,
};
use zaroxi_interface_theme::theme::SemanticColors;

fn default_color(sem: &SemanticColors) -> [f32; 4] {
    [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a]
}

fn keyword_color(sem: &SemanticColors) -> [f32; 4] {
    [sem.syntax_keyword.r, sem.syntax_keyword.g, sem.syntax_keyword.b, sem.syntax_keyword.a]
}

/// Whether any span overlaps the half-open byte range `[start, end)`.
fn any_span_intersects(spans: &[HighlightSpan], start: usize, end: usize) -> bool {
    spans.iter().any(|s| s.start < end && s.end > start)
}

/// Full-buffer byte range `[start, end)` of the `line_idx`-th line of `text`
/// (line content only, excluding the trailing newline).
fn line_byte_range(text: &str, line_idx: usize) -> (usize, usize) {
    let mut off = 0usize;
    for (i, line) in text.split('\n').enumerate() {
        if i == line_idx {
            return (off, off + line.len());
        }
        off += line.len() + 1;
    }
    (off, off)
}

#[test]
fn empty_spans_render_plain_text() {
    let sem = SemanticColors::debug();
    let lines = vec!["let x = 1".to_string()];
    let out = colorize_source(&lines, &sem, &[]);
    // One default-colored text segment plus the line terminator.
    assert_eq!(
        out,
        vec![
            ("let x = 1".to_string(), default_color(&sem)),
            ("\n".to_string(), default_color(&sem))
        ]
    );
}

#[test]
fn span_colorizes_matching_bytes() {
    let sem = SemanticColors::debug();
    let lines = vec!["let x = 1".to_string()];
    // "let" occupies bytes 0..3.
    let spans = vec![HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword }];
    let out = colorize_source(&lines, &sem, &spans);
    // First segment is the keyword, remainder is default-colored.
    assert_eq!(out[0], ("let".to_string(), keyword_color(&sem)));
    assert_eq!(out[1], (" x = 1".to_string(), default_color(&sem)));
    assert_eq!(out.last().unwrap().0, "\n");
}

#[test]
fn spans_map_to_correct_line_by_byte_offset() {
    let sem = SemanticColors::debug();
    let lines = vec!["aaa".to_string(), "bbb".to_string()];
    // Second line "bbb" starts at byte offset 4 (3 + newline). Color it.
    let spans = vec![HighlightSpan { start: 4, end: 7, highlight: Highlight::Keyword }];
    let out = colorize_source(&lines, &sem, &spans);
    // Line 0 untouched (default), line 1 colored.
    assert_eq!(out[0], ("aaa".to_string(), default_color(&sem)));
    assert_eq!(out[1], ("\n".to_string(), default_color(&sem)));
    assert_eq!(out[2], ("bbb".to_string(), keyword_color(&sem)));
}

#[test]
fn incremental_matches_full_for_changed_lines() {
    let sem = SemanticColors::debug();
    let lines = vec!["let x = 1".to_string()];
    let spans = vec![HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword }];
    let mut cache = HashMap::new();
    // All lines treated as changed (prev hash 0).
    let per_line = vec![1u64];
    let cached = vec![0u64];
    let inc = colorize_source_incremental(&lines, &sem, &spans, &mut cache, &per_line, &cached);
    assert_eq!(inc[0], ("let".to_string(), keyword_color(&sem)));
    assert_eq!(inc[1], (" x = 1".to_string(), default_color(&sem)));
}

// Document: "aaa\nbbb\nccc\nddd" — byte offsets aaa[0,3] bbb[4,7] ccc[8,11] ddd[12,15].

#[test]
fn window_rebases_document_spans() {
    let sem = SemanticColors::debug();
    let window = vec!["bbb".to_string(), "ccc".to_string()];
    let window_base = 4; // byte offset of "bbb" in the full document
    let spans = vec![
        HighlightSpan { start: 4, end: 7, highlight: Highlight::Keyword }, // bbb
        HighlightSpan { start: 8, end: 11, highlight: Highlight::Keyword }, // ccc
    ];
    let out = colorize_window(&window, window_base, &spans, &sem);
    assert_eq!(out[0], ("bbb".to_string(), keyword_color(&sem)));
    assert_eq!(out[1], ("\n".to_string(), default_color(&sem)));
    assert_eq!(out[2], ("ccc".to_string(), keyword_color(&sem)));
}

#[test]
fn window_clips_span_crossing_top_boundary() {
    let sem = SemanticColors::debug();
    let window = vec!["bbb".to_string(), "ccc".to_string()];
    let window_base = 4;
    // Span starts inside "aaa" (byte 2), extends into "bbb" — only the in-window
    // portion ("bbb") should be colored.
    let spans = vec![HighlightSpan { start: 2, end: 7, highlight: Highlight::Keyword }];
    let out = colorize_window(&window, window_base, &spans, &sem);
    assert_eq!(out[0], ("bbb".to_string(), keyword_color(&sem)));
}

#[test]
fn window_clips_span_crossing_bottom_boundary() {
    let sem = SemanticColors::debug();
    let window = vec!["bbb".to_string(), "ccc".to_string()];
    let window_base = 4;
    // Span covers "ccc" and extends past the window into "ddd"; only "ccc"
    // (clipped to the window end) should be colored.
    let spans = vec![HighlightSpan { start: 8, end: 14, highlight: Highlight::Keyword }];
    let out = colorize_window(&window, window_base, &spans, &sem);
    assert_eq!(out[2], ("ccc".to_string(), keyword_color(&sem)));
}

// ── Strict span-ownership / plain-text-default coverage ──

#[test]
fn validate_drops_only_invalid_spans() {
    let source = "let x = 1";
    let spans = vec![
        HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword }, // valid "let"
        HighlightSpan { start: 5, end: 5, highlight: Highlight::Keyword }, // empty range
        HighlightSpan { start: 3, end: 2, highlight: Highlight::Keyword }, // start >= end
        HighlightSpan { start: 4, end: 99, highlight: Highlight::Keyword }, // out of bounds
    ];
    let valid = validate_spans_for_source(&spans, source);
    assert_eq!(valid.len(), 1, "only the well-formed span survives");
    assert_eq!((valid[0].start, valid[0].end), (0, 3));
}

#[test]
fn out_of_bounds_span_renders_plain_not_panic() {
    let sem = SemanticColors::debug();
    let lines = vec!["abc".to_string()];
    // Span extends far past the 3-byte line: must be rejected, text stays plain,
    // and slicing must never panic.
    let spans = vec![HighlightSpan { start: 0, end: 999, highlight: Highlight::Keyword }];
    let out = colorize_source(&lines, &sem, &spans);
    assert_eq!(out[0], ("abc".to_string(), default_color(&sem)));
}

#[test]
fn shifted_non_char_boundary_span_is_rejected() {
    let sem = SemanticColors::debug();
    // "héllo": 'é' is a 2-byte char occupying bytes [1,3).
    let lines = vec!["héllo".to_string()];
    // start=2 lands in the MIDDLE of 'é' — not a UTF-8 boundary → rejected,
    // so the whole line renders plain rather than slicing mid-codepoint.
    let spans = vec![HighlightSpan { start: 2, end: 6, highlight: Highlight::Keyword }];
    let out = colorize_source(&lines, &sem, &spans);
    assert_eq!(out[0], ("héllo".to_string(), default_color(&sem)));
}

#[test]
fn newly_typed_uncovered_char_stays_plain() {
    let sem = SemanticColors::debug();
    // Simulate typing "X" right after the keyword "let": the stored span still
    // only covers [0,3). The trailing "X" is NOT covered by any current span and
    // must render plain — old token color must not bleed onto new bytes.
    let lines = vec!["letX".to_string()];
    let spans = vec![HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword }];
    let out = colorize_source(&lines, &sem, &spans);
    assert_eq!(out[0], ("let".to_string(), keyword_color(&sem)));
    assert_eq!(out[1], ("X".to_string(), default_color(&sem)));
}

#[test]
fn editing_one_line_keeps_unrelated_line_colored() {
    let sem = SemanticColors::debug();
    // Doc "fn\nlet": fn[0,2), '\n' at 2, let[3,6). Both keyword-colored.
    let lines = vec!["fn".to_string(), "let".to_string()];
    let spans = vec![
        HighlightSpan { start: 0, end: 2, highlight: Highlight::Keyword },
        HighlightSpan { start: 3, end: 6, highlight: Highlight::Keyword },
    ];
    let out = colorize_source(&lines, &sem, &spans);
    assert_eq!(out[0], ("fn".to_string(), keyword_color(&sem)));
    assert_eq!(out[1], ("\n".to_string(), default_color(&sem)));
    assert_eq!(out[2], ("let".to_string(), keyword_color(&sem)));
}

// ── extract_line_spans: coverage continues after plain gaps ──

#[test]
fn spans_apply_before_across_and_after_a_plain_gap() {
    let sem = SemanticColors::debug();
    // Doc: "aaa\nbbb\nccc" — color aaa and ccc, leave bbb (the "edited" line)
    // uncovered. The gap on line 1 must NOT stop consumption of the ccc span.
    let lines = vec!["aaa".to_string(), "bbb".to_string(), "ccc".to_string()];
    let spans = vec![
        HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword }, // aaa
        HighlightSpan { start: 8, end: 11, highlight: Highlight::Keyword }, // ccc
    ];
    let out = colorize_source(&lines, &sem, &spans);
    assert_eq!(out[0], ("aaa".to_string(), keyword_color(&sem)));
    assert_eq!(out[1], ("\n".to_string(), default_color(&sem)));
    assert_eq!(out[2], ("bbb".to_string(), default_color(&sem))); // plain gap
    assert_eq!(out[3], ("\n".to_string(), default_color(&sem)));
    assert_eq!(out[4], ("ccc".to_string(), keyword_color(&sem))); // resumes!
}

#[test]
fn sparse_spans_with_many_gaps_all_apply() {
    let sem = SemanticColors::debug();
    // Five lines; color only 0, 2, 4 — many plain gaps between.
    let lines: Vec<String> = (0..5).map(|i| format!("ln{i}")).collect(); // "ln0".."ln4", 3 bytes each
    // byte offsets: ln0[0,3] ln1[4,7] ln2[8,11] ln3[12,15] ln4[16,19]
    let spans = vec![
        HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword },
        HighlightSpan { start: 8, end: 11, highlight: Highlight::Keyword },
        HighlightSpan { start: 16, end: 19, highlight: Highlight::Keyword },
    ];
    let out = colorize_source(&lines, &sem, &spans);
    assert_eq!(out[0], ("ln0".to_string(), keyword_color(&sem)));
    assert_eq!(out[4], ("ln2".to_string(), keyword_color(&sem)));
    assert_eq!(out[8], ("ln4".to_string(), keyword_color(&sem)));
    // The interleaved gap lines stay plain.
    assert_eq!(out[2], ("ln1".to_string(), default_color(&sem)));
    assert_eq!(out[6], ("ln3".to_string(), default_color(&sem)));
}

// ── remap_good_spans_across_edit: downstream coverage retention ──

#[test]
fn coverage_end_reports_last_covered_byte() {
    let spans = vec![
        HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword },
        HighlightSpan { start: 8, end: 11, highlight: Highlight::Keyword },
    ];
    assert_eq!(coverage_end(&spans), 11);
    assert_eq!(coverage_end(&[]), 0);
}

#[test]
fn remap_retains_downstream_but_holes_the_edited_line() {
    // Baseline (clean, full coverage): "aaa\nbbb\nccc".
    let old = "aaa\nbbb\nccc";
    let good = vec![
        HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword }, // aaa
        HighlightSpan { start: 4, end: 7, highlight: Highlight::Keyword }, // bbb
        HighlightSpan { start: 8, end: 11, highlight: Highlight::Keyword }, // ccc
    ];
    // User typed "X" at the start of line 1 → "aaa\nXbbb\nccc" (degraded).
    let new = "aaa\nXbbb\nccc";
    let remapped = remap_good_spans_across_edit(old, new, &good);
    // Downstream coverage restored to EOF (ccc), NOT collapsed to the fresh
    // degraded prefix.
    assert_eq!(coverage_end(&remapped), new.len());

    let sem = SemanticColors::debug();
    let lines: Vec<String> = new.split('\n').map(|s| s.to_string()).collect();
    let out = colorize_source(&lines, &sem, &remapped);
    // line 0 "aaa": unchanged prefix, keyword.
    assert_eq!(out[0], ("aaa".to_string(), keyword_color(&sem)));
    // line 1 "Xbbb": the ENTIRE edited line is plain (the whole line is the
    // hole) — no partial coloring of the retained "bbb" tail.
    assert_eq!(out[2], ("Xbbb".to_string(), default_color(&sem)));
    // line 2 "ccc": downstream unchanged, still keyword.
    assert_eq!(out[4], ("ccc".to_string(), keyword_color(&sem)));
}

#[test]
fn remap_retains_downstream_after_delete() {
    // Deletion variant: baseline "kw1\nkw2\nkw3" → delete a char on line 1.
    let old = "kw1\nkw2\nkw3";
    let good = vec![
        HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword },
        HighlightSpan { start: 4, end: 7, highlight: Highlight::Keyword },
        HighlightSpan { start: 8, end: 11, highlight: Highlight::Keyword },
    ];
    // "kw2" -> "k2" (deleted 'w'): "kw1\nk2\nkw3".
    let new = "kw1\nk2\nkw3";
    let remapped = remap_good_spans_across_edit(old, new, &good);
    assert_eq!(coverage_end(&remapped), new.len());
    let sem = SemanticColors::debug();
    let lines: Vec<String> = new.split('\n').map(|s| s.to_string()).collect();
    let out = colorize_source(&lines, &sem, &remapped);
    // Edited line "k2" is plain; last line "kw3" still a keyword.
    assert_eq!(out.iter().find(|(t, _)| t == "k2").map(|(_, c)| *c), Some(default_color(&sem)));
    assert_eq!(out.iter().find(|(t, _)| t == "kw3").map(|(_, c)| *c), Some(keyword_color(&sem)));
}

#[test]
fn remap_multibyte_suffix_stays_on_char_boundaries() {
    // Ensure the prefix/suffix diff never splits a multi-byte codepoint.
    let old = "a=1\nx: héllo\n";
    let good = vec![HighlightSpan { start: 4, end: 5, highlight: Highlight::Keyword }]; // "x"
    // Insert a char on the first line: "aa=1\nx: héllo\n".
    let new = "aa=1\nx: héllo\n";
    let remapped = remap_good_spans_across_edit(old, new, &good);
    // Whatever survives must validate cleanly against the new text (no panic,
    // no mid-codepoint slice).
    let valid = validate_spans_for_source(&remapped, new);
    assert_eq!(valid.len(), remapped.len(), "all remapped spans are byte-valid");
}

// ── End-to-end with the REAL YAML grammar: the actual regression ──

fn real_spans(lang: LanguageId, src: &str) -> (Vec<HighlightSpan>, bool) {
    let pool = ParserPool::new();
    let mut parser = match pool.acquire(&lang) {
        Some(p) => p,
        None => return (Vec::new(), false),
    };
    let tree = parser.parse(src, None).expect("parse");
    let had_error = tree.root_node().has_error();
    let spans = HighlightEngine::new().highlight(lang, src, &tree).unwrap_or_default();
    pool.release(&lang, parser);
    (spans, had_error)
}

#[test]
fn yaml_midedit_retains_downstream_highlighting_via_remap() {
    let lang = LanguageId::Dynamic("yaml");
    let valid =
        "name: app\nversion: 1\nservices:\n  web:\n    image: nginx\n    ports:\n      - 80\n";
    let (good, good_err) = real_spans(lang, valid);
    if good.is_empty() {
        // Grammar not installed in this environment — skip rather than false-fail.
        eprintln!("yaml grammar unavailable; skipping");
        return;
    }
    assert!(!good_err, "baseline YAML parses cleanly");
    let good_cov = coverage_end(&good);
    assert!(good_cov >= valid.len() - 2, "baseline covers ~whole buffer");

    // Mid-edit: a bareword line inserted before `services:` makes the doc
    // temporarily invalid — Tree-sitter collapses coverage.
    let midedit =
        "name: app\nversion: 1\nfoo\nservices:\n  web:\n    image: nginx\n    ports:\n      - 80\n";
    let (fresh, fresh_err) = real_spans(lang, midedit);
    assert!(fresh_err, "mid-edit YAML is degraded");
    let fresh_cov = coverage_end(&fresh);
    assert!(
        fresh_cov < good_cov,
        "degraded fresh coverage ({fresh_cov}) collapses below baseline ({good_cov})",
    );

    // The fix: remap the retained baseline across the edit.
    let remapped = remap_good_spans_across_edit(valid, midedit, &good);
    let remapped_cov = coverage_end(&remapped);
    assert!(
        remapped_cov > fresh_cov,
        "remap restores downstream coverage ({remapped_cov}) beyond the degraded parse ({fresh_cov})",
    );
    // Downstream coverage should reach near the end of the edited buffer.
    assert!(
        remapped_cov >= midedit.len() - 2,
        "remap covers to ~EOF ({remapped_cov} of {})",
        midedit.len(),
    );

    // And the remapped spans must all be byte-valid against the current text.
    let valid_spans = validate_spans_for_source(&remapped, midedit);
    assert_eq!(valid_spans.len(), remapped.len());
}

// ── Edit-hole regression tests (Part 5) ──

#[test]
fn degraded_edit_line_stays_plain() {
    // Real YAML: a transiently-invalid inserted line must render fully plain,
    // while later unchanged lines keep their highlighting.
    let lang = LanguageId::Dynamic("yaml");
    let valid = "name: app\nversion: 1\nservices:\n  web:\n    image: nginx\n";
    let (good, good_err) = real_spans(lang, valid);
    if good.is_empty() {
        eprintln!("yaml grammar unavailable; skipping");
        return;
    }
    assert!(!good_err);

    // Insert a bareword line "foo" after "version: 1" → degraded parse.
    let midedit = "name: app\nversion: 1\nfoo\nservices:\n  web:\n    image: nginx\n";
    let (_fresh, fresh_err) = real_spans(lang, midedit);
    assert!(fresh_err, "mid-edit YAML must be degraded");

    let remapped = remap_good_spans_across_edit(valid, midedit, &good);

    // The edited line ("foo", line index 2) is entirely plain: no span touches
    // its byte range.
    let (edit_ls, edit_le) = line_byte_range(midedit, 2);
    assert!(
        !any_span_intersects(&remapped, edit_ls, edit_le),
        "edited line must have no syntax spans (range {edit_ls}..{edit_le})",
    );
    // A later unchanged line ("image: nginx", line index 5) still has spans.
    let (later_ls, later_le) = line_byte_range(midedit, 5);
    assert!(
        any_span_intersects(&remapped, later_ls, later_le),
        "later unchanged line must retain highlighting (range {later_ls}..{later_le})",
    );
}

#[test]
fn degraded_multiline_edit_hole_stays_plain() {
    // A degraded edit that removes a newline (merging lines 1 & 2) must leave
    // the whole merged line plain, while surrounding lines keep their colors.
    let old = "aa\nbb\ncc\ndd";
    let good = vec![
        HighlightSpan { start: 0, end: 2, highlight: Highlight::Keyword }, // aa
        HighlightSpan { start: 3, end: 5, highlight: Highlight::Keyword }, // bb
        HighlightSpan { start: 6, end: 8, highlight: Highlight::Keyword }, // cc
        HighlightSpan { start: 9, end: 11, highlight: Highlight::Keyword }, // dd
    ];
    // Replace "bb\ncc" with "bXYZc": "aa\nbXYZc\ndd".
    let new = "aa\nbXYZc\ndd";
    let remapped = remap_good_spans_across_edit(old, new, &good);

    // The merged edited line (index 1) is fully plain.
    let (ls, le) = line_byte_range(new, 1);
    assert!(!any_span_intersects(&remapped, ls, le), "merged edited line must be plain");

    // Surrounding lines keep their colors.
    let sem = SemanticColors::debug();
    let lines: Vec<String> = new.split('\n').map(|s| s.to_string()).collect();
    let out = colorize_source(&lines, &sem, &remapped);
    assert_eq!(out[0], ("aa".to_string(), keyword_color(&sem)));
    assert_eq!(out[2], ("bXYZc".to_string(), default_color(&sem)));
    assert_eq!(out[4], ("dd".to_string(), keyword_color(&sem)));
}

#[test]
fn clean_parse_restores_line_highlighting() {
    // Invalid edit → the line is plain (via retained remap). Fixing the syntax →
    // a CLEAN full-buffer parse is authoritative and the line regains color.
    let lang = LanguageId::Dynamic("yaml");
    let valid = "name: app\nversion: 1\nservices:\n  web:\n    image: nginx\n";
    let (good, good_err) = real_spans(lang, valid);
    if good.is_empty() {
        eprintln!("yaml grammar unavailable; skipping");
        return;
    }
    assert!(!good_err);

    // 1) Invalid: bareword line → degraded → remap → edited line plain.
    let invalid = "name: app\nversion: 1\nfoo\nservices:\n  web:\n    image: nginx\n";
    let (_fi, invalid_err) = real_spans(lang, invalid);
    assert!(invalid_err);
    let remapped = remap_good_spans_across_edit(valid, invalid, &good);
    let (ls, le) = line_byte_range(invalid, 2);
    assert!(!any_span_intersects(&remapped, ls, le), "invalid edited line is plain");

    // 2) Fixed: "foo: bar" → clean parse (had_error=false). The clean fresh
    //    result is used directly (no remap) and colors the previously-plain line.
    let fixed = "name: app\nversion: 1\nfoo: bar\nservices:\n  web:\n    image: nginx\n";
    let (fresh_fixed, fixed_err) = real_spans(lang, fixed);
    assert!(!fixed_err, "fixed YAML parses cleanly");
    let (fls, fle) = line_byte_range(fixed, 2);
    assert!(
        any_span_intersects(&fresh_fixed, fls, fle),
        "clean parse restores highlighting on the formerly-plain line",
    );
}

#[test]
fn no_span_crosses_edit_hole() {
    // A keyword on the SAME line as the edit but in the unchanged prefix must be
    // dropped — the full edited line is a plain hole, so nothing crosses into it.
    let old = "let x = 1\nnext\n";
    let good = vec![
        HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword }, // "let"
        HighlightSpan { start: 10, end: 14, highlight: Highlight::Keyword }, // "next"
    ];
    // Type 'z' after the "1" → "let x = 1z\nnext\n" (degraded).
    let new = "let x = 1z\nnext\n";
    let remapped = remap_good_spans_across_edit(old, new, &good);

    // No remapped span may intersect the edited line (index 0) — not even the
    // "let" keyword that preceded the edit point.
    let (ls, le) = line_byte_range(new, 0);
    assert!(
        !any_span_intersects(&remapped, ls, le),
        "no span may cross into the edit hole (line0 {ls}..{le})",
    );
    // The downstream "next" line keeps its keyword.
    let (nls, nle) = line_byte_range(new, 1);
    assert!(any_span_intersects(&remapped, nls, nle), "downstream line retained");
}

#[test]
fn multibyte_boundary_hole_safe() {
    // Multibyte chars in both the edited line and the retained suffix: the hole
    // must stay char-boundary safe and no span may cross it.
    let old = "ab\nx = café\n";
    let good = vec![
        HighlightSpan { start: 3, end: 4, highlight: Highlight::Keyword }, // "x"
        HighlightSpan { start: 7, end: 12, highlight: Highlight::Type },   // "café" (é = 2 bytes)
    ];
    // Insert 'Z' on line 0 → "abZ\nx = café\n" (degraded).
    let new = "abZ\nx = café\n";
    let remapped = remap_good_spans_across_edit(old, new, &good);

    // Every remapped span is byte-valid (on char boundaries, in-bounds).
    let valid = validate_spans_for_source(&remapped, new);
    assert_eq!(valid.len(), remapped.len(), "no span split a multi-byte codepoint");

    // The edited line (index 0) is a plain hole.
    let (ls, le) = line_byte_range(new, 0);
    assert!(!any_span_intersects(&remapped, ls, le), "edited line is plain");

    // The multibyte suffix ("café") is retained (shifted onto the same chars).
    let (sls, sle) = line_byte_range(new, 1);
    assert!(any_span_intersects(&remapped, sls, sle), "multibyte suffix retained");
}

#[test]
fn incremental_cache_not_reused_across_span_generations() {
    let sem = SemanticColors::debug();
    let lines = vec!["abcdef".to_string()];
    let mut cache = HashMap::new();

    // Generation 1: color [0,3) as keyword. First pass (cached hash 0) treats
    // the line as changed and populates the per-line cache.
    let per_line = vec![42u64];
    let spans_g1 = vec![HighlightSpan { start: 0, end: 3, highlight: Highlight::Keyword }];
    let out1 = colorize_source_incremental(&lines, &sem, &spans_g1, &mut cache, &per_line, &[0u64]);
    assert_eq!(out1[0], ("abc".to_string(), keyword_color(&sem)));
    assert_eq!(out1[1], ("def".to_string(), default_color(&sem)));

    // Generation 2: identical line text (same hash → reuse branch), but the
    // spans now cover [0,6). The spans-fingerprint in the cache key must force a
    // rebuild instead of returning the stale generation-1 payload.
    let spans_g2 = vec![HighlightSpan { start: 0, end: 6, highlight: Highlight::Keyword }];
    let out2 =
        colorize_source_incremental(&lines, &sem, &spans_g2, &mut cache, &per_line, &[42u64]);
    assert_eq!(out2[0], ("abcdef".to_string(), keyword_color(&sem)));
}
