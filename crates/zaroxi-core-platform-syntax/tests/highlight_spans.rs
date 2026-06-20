//! Highlight-engine integration tests: representative snippets must produce
//! non-empty spans for supported languages (rust, nix, markdown, toml), and
//! plain text must explicitly produce no spans.
//!
//! These exercise the real bundled grammar loader (the crate's default
//! `dynamic-loading` feature) end-to-end: grammar load -> parse -> query.

use zaroxi_core_platform_syntax::highlight::HighlightEngine;
use zaroxi_core_platform_syntax::language::LanguageId;
use zaroxi_core_platform_syntax::parser::ParserPool;

fn highlight_span_count(lang: LanguageId, src: &str) -> usize {
    let pool = ParserPool::new();
    let mut parser = match pool.acquire(&lang) {
        Some(p) => p,
        None => return 0,
    };
    let tree = parser.parse(src, None).expect("grammar should parse snippet");
    let engine = HighlightEngine::new();
    let spans = engine.highlight(lang, src, &tree).unwrap_or_default();
    pool.release(&lang, parser);
    spans.len()
}

#[test]
fn rust_produces_spans() {
    assert!(highlight_span_count(LanguageId::Rust, "fn main() { let x = 1; }\n") > 0);
}

#[test]
fn nix_produces_spans() {
    assert!(highlight_span_count(LanguageId::Dynamic("nix"), "{ pkgs }: { foo = \"bar\"; }\n") > 0);
}

#[test]
fn markdown_produces_spans() {
    assert!(highlight_span_count(LanguageId::Markdown, "# Title\n\nSome **bold** text.\n") > 0);
}

#[test]
fn toml_produces_spans() {
    assert!(highlight_span_count(LanguageId::Toml, "[package]\nname = \"x\"\nversion = 1\n") > 0);
}

#[test]
fn plain_text_produces_no_spans() {
    // PlainText is the explicit, intentional fallback: no grammar, no spans.
    assert_eq!(highlight_span_count(LanguageId::PlainText, "just some text\n"), 0);
}
