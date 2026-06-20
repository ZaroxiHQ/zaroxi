//! Integration tests for applying stored highlight spans to editor source
//! lines (Phase 1 syntax highlighting presentation layer).

use std::collections::HashMap;

use zaroxi_core_platform_syntax::highlight::{Highlight, HighlightSpan};
use zaroxi_interface_desktop::gui::window::syntax_color::{
    colorize_source, colorize_source_incremental,
};
use zaroxi_interface_theme::theme::SemanticColors;

fn default_color(sem: &SemanticColors) -> [f32; 4] {
    [sem.text_primary.r, sem.text_primary.g, sem.text_primary.b, sem.text_primary.a]
}

fn keyword_color(sem: &SemanticColors) -> [f32; 4] {
    [sem.syntax_keyword.r, sem.syntax_keyword.g, sem.syntax_keyword.b, sem.syntax_keyword.a]
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
