//! Phase 39 integration test: editor render contract formalization.
//!
//! Tests for the canonical `EditorRenderContract` → `EditorPrimitiveSet`
//! projection path. Verifies invariants: caret position, selection bbox,
//! gutter labels, and scroll consistency.

use zaroxi_core_editor_view::{EditorRenderContract, EditorRenderMetrics};
use zaroxi_core_engine_scene::{CaretItem, EditorPrimitiveSet, SelectionRect, TextPrimitive};
use zaroxi_interface_desktop::presenters::transcript::build_primitives_from_contract;

/// Verify caret position is correct for cursor on the first visible line.
#[test]
fn caret_projected_correctly_on_first_line() {
    let contract = EditorRenderContract::new(
        vec!["hello world".to_string(), "second line".to_string()],
        1,
        Some(1), // cursor on line 1
        Some(5), // column 5
        None,
    );
    let metrics = EditorRenderMetrics::DEFAULT;

    let set = build_primitives_from_contract(200, 100, &contract, &metrics);

    // Should have exactly one caret
    assert_eq!(set.carets.len(), 1, "one caret expected on the cursor line");
    let caret = &set.carets[0];

    // Caret x: content_x + content_inset + column * char_width
    // 200 + 6 + 5*8 = 246
    assert_eq!(caret.x, 200 + 6 + 5 * 8, "caret x at column 5");
    // Caret y: base_y + 0*line_height = 100 (cursor is the first visible row)
    assert_eq!(caret.y, 100, "caret y on the first row");
    assert_eq!(caret.height, metrics.line_height);
}

/// Verify selection produces correct rectangles for multi-line selections.
#[test]
fn selection_produces_per_line_rects() {
    let contract = EditorRenderContract::new(
        vec!["line one".to_string(), "line two".to_string(), "line three".to_string()],
        1,
        None,
        None,
        Some((1, 2, 3, 6)), // select from line 1 col 2 to line 3 col 6
    );
    let metrics = EditorRenderMetrics::DEFAULT;

    let set = build_primitives_from_contract(200, 100, &contract, &metrics);
    let content_text_x = 200 + metrics.content_inset;

    // Should have 3 selection rects (one per line in range)
    assert_eq!(set.selections.len(), 3, "expect 3 selection rects for 3-line range");

    // Line 1: cols 2 to end=8 (chars.count = 8)
    let r0 = &set.selections[0];
    assert_eq!(r0.x, content_text_x + 2 * 8); // start col 2
    assert_eq!(r0.y, 100);
    assert_eq!(r0.width, (8 - 2) * 8); // end col 8 - start 2 = 6 chars
    assert_eq!(r0.height, metrics.line_height);

    // Line 2: full line (cols 0 to end=8)
    let r1 = &set.selections[1];
    assert_eq!(r1.x, content_text_x); // start col 0
    assert_eq!(r1.y, 100 + 16);
    assert_eq!(r1.width, 8 * 8); // full line width

    // Line 3: cols 0 to 6
    let r2 = &set.selections[2];
    assert_eq!(r2.x, content_text_x); // start col 0
    assert_eq!(r2.y, 100 + 32);
    assert_eq!(r2.width, 6 * 8); // end col 6
}

/// Verify gutter labels follow visible lines with correct 1-based numbering.
#[test]
fn gutter_labels_match_visible_lines() {
    let contract = EditorRenderContract::new(
        vec!["a".to_string(), "b".to_string(), "c".to_string()],
        10, // top line is line 10
        None,
        None,
        None,
    );
    let metrics = EditorRenderMetrics::DEFAULT;

    let set = build_primitives_from_contract(200, 100, &contract, &metrics);

    assert_eq!(set.gutter_labels.len(), 3);
    assert_eq!(set.gutter_labels[0].text.trim(), "10");
    assert_eq!(set.gutter_labels[1].text.trim(), "11");
    assert_eq!(set.gutter_labels[2].text.trim(), "12");
}

/// Verify text primitives are emitted in the correct order.
#[test]
fn text_primitives_preserve_content_and_order() {
    let contract = EditorRenderContract::new(
        vec!["first".to_string(), "second".to_string()],
        5,
        None,
        None,
        None,
    );
    let metrics = EditorRenderMetrics::DEFAULT;

    let set = build_primitives_from_contract(200, 100, &contract, &metrics);

    assert_eq!(set.texts.len(), 2);
    assert_eq!(set.texts[0].text, "first");
    assert_eq!(set.texts[1].text, "second");

    // Text entries use content_inset
    let expected_x = 200 + metrics.content_inset;
    assert_eq!(set.texts[0].x, expected_x);
    assert_eq!(set.texts[0].y, 100);
    assert_eq!(set.texts[1].y, 100 + metrics.line_height);
}

/// Verify absent contract produces an empty primitive set.
#[test]
fn absent_contract_produces_empty_set() {
    let contract = EditorRenderContract::absent();
    let metrics = EditorRenderMetrics::DEFAULT;

    let set = build_primitives_from_contract(200, 100, &contract, &metrics);

    assert!(set.texts.is_empty());
    assert!(set.carets.is_empty());
    assert!(set.selections.is_empty());
    assert!(set.gutter_labels.is_empty());
}

/// Verify scroll offset: when top_line is >1, caret y is offset correctly.
#[test]
fn caret_y_respects_scroll_offset() {
    let contract = EditorRenderContract::new(
        (1..=15).map(|i| format!("line {}", i)).collect::<Vec<_>>(),
        6,        // scroll: top visible is line 6
        Some(10), // cursor on line 10
        Some(0),
        None,
    );
    let metrics = EditorRenderMetrics::DEFAULT;

    let set = build_primitives_from_contract(200, 100, &contract, &metrics);

    assert_eq!(set.carets.len(), 1);
    // Cursor line 10 relative to visible top line 6: offset = 10 - 6 = 4 rows
    // y = base_y + 4 * line_height = 100 + 64 = 164
    assert_eq!(set.carets[0].y, 100 + 4 * metrics.line_height);
}
