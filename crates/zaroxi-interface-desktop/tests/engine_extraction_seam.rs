//! Phase 38 integration test: proving the engine extraction seam works for
//! a non-IDE, generic document viewer use case.
//!
//! This test constructs a plain document viewer (no tabs, no AI panel,
//! no explorer, no terminal) using only the app-neutral engine contract
//! and the adapter seam. It verifies the engine types are valid and
//! convertible to the engine scene model.
//!
//! No IDE-specific concepts are used anywhere in the engine pipeline.

use zaroxi_core_engine_scene::ShellSceneModel;
use zaroxi_interface_desktop::engine_adapter;

/// A minimal non-IDE document viewer produces a RenderedDocument and feeds it
/// into an engine scene model.
#[test]
fn generic_document_viewer_to_scene() {
    // 1. Simulate a stand-alone document viewer: plain text, no IDE chrome.
    let lines = vec![
        "# README".to_string(),
        "".to_string(),
        "This is a plain text document.".to_string(),
        "".to_string(),
        "- Item 1".to_string(),
        "- Item 2".to_string(),
        "- Item 3".to_string(),
    ];
    let n_lines = lines.len() as u32;

    // Build the RenderedDocument via the adapter seam (app-neutral contract).
    let doc = engine_adapter::build_rendered_document(
        &lines,
        1,
        n_lines,
        Some("Ln 1/7"),
        Some((3, 5)), // caret at line 3, col 5
        None,         // no selection
        n_lines,
    );

    // 2. Convert to engine scene model using only generic fields.
    let viewport = &doc.viewport;
    let caret = doc.caret.as_ref();

    let scene = ShellSceneModel {
        text_lines: viewport.visible_lines.clone(),
        viewport_top_line: viewport.top_line,
        viewport_total_lines: viewport.total_lines,
        viewport_summary: viewport.summary.clone(),
        cursor_line: caret.map(|c| c.line),
        cursor_column: caret.map(|c| c.column),
        selection_present: doc.selection.is_some(),
        status_text: doc.scroll.as_ref().map(|s| format!("Ln {}/{}", s.top_line, s.total_lines)),
        decoration_text: None,
    };

    // Verify the scene model contains the document content.
    assert_eq!(scene.text_lines.len(), 7);
    assert_eq!(scene.text_lines[0], "# README");
    assert_eq!(scene.cursor_line, Some(3));
    assert_eq!(scene.cursor_column, Some(5));
    assert!(!scene.selection_present);
    assert_eq!(scene.viewport_top_line, 1);
    assert_eq!(scene.viewport_total_lines, 7);
}

/// Prove that the generic engine contract supports rendering with only
/// cursor (no selection) — a common simplified viewer mode.
#[test]
fn generic_viewer_cursor_only_no_ide_concepts() {
    let lines: Vec<String> = (1..=20).map(|i| format!("Line {}", i)).collect();
    let n_lines = lines.len() as u32;

    let doc = engine_adapter::build_rendered_document(
        &lines,
        5, // scrolled to line 5
        n_lines,
        None,
        Some((12, 0)), // caret at line 12
        None,
        10, // 10 visible lines
    );

    // Verify scroll model
    let scroll = doc.scroll.as_ref().unwrap();
    assert_eq!(scroll.top_line, 5);
    assert_eq!(scroll.total_lines, 20);
    assert_eq!(scroll.visible_line_count, 10);
    assert!(scroll.viewport_ratio() > 0.0); // 10/20 = 0.5
    assert!(scroll.scroll_proportion() > 0.0); // (5-1)/(20-10) = 4/10 = 0.4

    // Feed through to scene
    let scene = ShellSceneModel {
        text_lines: doc.viewport.visible_lines.clone(),
        viewport_top_line: doc.viewport.top_line,
        viewport_total_lines: doc.viewport.total_lines,
        viewport_summary: doc.viewport.summary.clone(),
        cursor_line: doc.caret.as_ref().map(|c| c.line),
        cursor_column: doc.caret.as_ref().map(|c| c.column),
        selection_present: doc.selection.is_some(),
        status_text: None,
        decoration_text: None,
    };

    assert_eq!(scene.cursor_line, Some(12));
    assert_eq!(scene.viewport_top_line, 5);
    assert!(!scene.selection_present);
}

/// Prove the contract works with an absent/empty document (no active view).
#[test]
fn generic_viewer_absent_document_produces_empty_scene() {
    let doc = engine_adapter::build_absent_document();

    let scene = ShellSceneModel {
        text_lines: doc.viewport.visible_lines.clone(),
        viewport_top_line: doc.viewport.top_line,
        viewport_total_lines: doc.viewport.total_lines,
        viewport_summary: doc.viewport.summary.clone(),
        cursor_line: doc.caret.as_ref().map(|c| c.line),
        cursor_column: doc.caret.as_ref().map(|c| c.column),
        selection_present: doc.selection.is_some(),
        status_text: None,
        decoration_text: None,
    };

    assert!(scene.text_lines.is_empty());
    assert_eq!(scene.viewport_top_line, 0);
    assert_eq!(scene.viewport_total_lines, 0);
    assert!(scene.cursor_line.is_none());
    assert!(!scene.selection_present);
}

/// Prove the generic SpanKind and TextSpan types work for syntax coloring
/// in a non-IDE context (syntax highlighting in a plain log viewer).
#[test]
fn generic_syntax_spans_for_plain_viewer() {
    use zaroxi_core_engine_scene::{SpanKind, SyntaxSpan, TextSpan};

    let spans = vec![
        SyntaxSpan::new(
            0,
            vec![TextSpan::new(0, 5, SpanKind::Emphasis), TextSpan::new(7, 13, SpanKind::Plain)],
        ),
        SyntaxSpan::new(
            2,
            vec![TextSpan::new(0, 4, SpanKind::Keyword), TextSpan::new(5, 10, SpanKind::Highlight)],
        ),
    ];

    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].line_index, 0);
    assert_eq!(spans[0].spans.len(), 2);
    assert_eq!(spans[1].spans[0].kind, SpanKind::Keyword);
    assert_eq!(spans[1].spans[1].kind, SpanKind::Highlight);
}
