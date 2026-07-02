use super::editor_projection::{DEFAULT_CHAR_WIDTH, DEFAULT_LINE_HEIGHT};
use super::*;

#[test]
fn initial_refresh_small_file() {
    let lines = vec!["one".to_string(), "two".to_string(), "three".to_string()];
    // content_x/base_y chosen to exercise gutter_x > 0 branch
    let set = build_editor_primitives_from_lines(100, 50, &lines, None);
    assert_eq!(set.gutter_labels.len(), 3, "expected 3 gutter labels");
    assert_eq!(set.texts.len(), 3, "expected 3 text runs");
    assert_eq!(set.gutter_labels[0].text.trim(), "1", "first gutter should label line 1");
    assert_eq!(set.texts[0].text, "one", "first content line mismatch");
    assert!(set.carets.is_empty(), "no caret expected without layout");
    assert!(set.selections.is_empty(), "no selection expected without layout");
}

#[test]
fn caret_projection_inside_visible_range() {
    let lines = vec!["line1".to_string(), "line2".to_string(), "line3".to_string()];
    let layout = EditorLayoutSpec {
        top_line: Some(1),
        cursor_line: Some(2),
        cursor_column: Some(3),
        selection: None,
    };
    let set = build_editor_primitives_from_lines(100, 50, &lines, Some(&layout));
    assert_eq!(set.carets.len(), 1, "caret should be present");
    // compute expected caret position according to presenter math:
    // content_text_x = 100 + 6 = 106, char_w = 8 => x = 106 + 3*8 = 130
    // caret_y = base_y + (cursor_line - top_line) * line_h = 50 + 1*16 = 66
    assert_eq!(set.carets[0].x, 130, "caret x mismatch");
    assert_eq!(set.carets[0].y, 66, "caret y mismatch");
    assert_eq!(set.carets[0].height, DEFAULT_LINE_HEIGHT, "caret height should match line height");
}

#[test]
fn top_line_offset_changes_gutter_numbers() {
    let lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let layout = EditorLayoutSpec {
        top_line: Some(3),
        cursor_line: None,
        cursor_column: None,
        selection: None,
    };
    let set = build_editor_primitives_from_lines(200, 10, &lines, Some(&layout));
    assert_eq!(set.gutter_labels.len(), 3);
    assert_eq!(set.gutter_labels[0].text.trim(), "3", "gutter should start at top_line value");
    assert_eq!(set.gutter_labels[1].text.trim(), "4", "gutter should increment per visible row");
}

#[test]
fn selection_projection_multi_line() {
    let lines = vec!["first line".to_string(), "second line".to_string(), "third line".to_string()];
    // selection from line 1 col 1 to line 2 col 3 (1-based lines)
    let layout = EditorLayoutSpec {
        top_line: Some(1),
        cursor_line: None,
        cursor_column: None,
        selection: Some((1, 1, 2, 3)),
    };
    let set = build_editor_primitives_from_lines(80, 20, &lines, Some(&layout));
    // Expect at least one selection rect (should cover two rows intersecting)
    assert!(!set.selections.is_empty(), "expected selection rect(s) for intersecting visible rows");
    // verify selection rects are inside content x area (inset by +6)
    for s in &set.selections {
        assert!(
            s.x >= 86 || s.x == 0,
            "selection x should be at/after content inset (x >= content_x+6)"
        );
        assert_eq!(s.height, DEFAULT_LINE_HEIGHT, "selection height must equal line height");
    }
}

#[test]
fn active_buffer_switch_reflects_new_text() {
    let buf_a = vec!["alpha".to_string(), "beta".to_string()];
    let buf_b = vec!["uno".to_string(), "dos".to_string(), "tres".to_string()];

    let set_a = build_editor_primitives_from_lines(120, 30, &buf_a, None);
    let set_b = build_editor_primitives_from_lines(120, 30, &buf_b, None);

    assert_eq!(set_a.texts[0].text, "alpha");
    assert_eq!(set_b.texts[0].text, "uno");
    assert_eq!(set_a.texts.len(), 2);
    assert_eq!(set_b.texts.len(), 3);
}

#[test]
fn end_to_end_click_and_type_updates_scene_and_transcript() {
    // prepare a small scene with two lines
    let model = zaroxi_core_engine_scene::ShellSceneModel {
        text_lines: vec!["ab".to_string(), "cd".to_string()],
        viewport_top_line: 1,
        viewport_total_lines: 2,
        viewport_summary: None,
        cursor_line: Some(1),
        cursor_column: Some(0),
        selection_present: false,
        status_text: None,
        decoration_text: None,
    };
    zaroxi_core_engine_scene::set_current_scene(model);

    // simulate a click to place cursor at line 1, column 1:
    // content_x = 100, base_y = 50, content_inset = 6, char_w = 8, line_h = 16
    // content_text_x = 106; to place at column 1 click_x = 106 + 1*8 = 114
    handle_mouse_click_and_place_cursor(
        114,
        50,
        100,
        50,
        DEFAULT_CHAR_WIDTH,
        DEFAULT_LINE_HEIGHT,
        6,
    );

    // type 'X' at the caret
    handle_key_char('X');

    // check the live engine scene was updated
    let s = zaroxi_core_engine_scene::get_current_scene();
    assert_eq!(s.text_lines[0], "aXb");
    assert_eq!(s.cursor_line.unwrap(), 1);
    assert_eq!(s.cursor_column.unwrap(), 2);

    // verify the transcript scene summary reflects the live changes
    let summary = ShellRenderTranscript::engine_scene_summary();
    assert!(summary.contains("engine_line 1: aXb"));
    assert!(summary.contains("engine_cursor: Some(1):Some(2)"));
}
