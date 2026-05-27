use zaroxi_application_workspace::ports::{EditorCursor, EditorDocument};
use zaroxi_application_workspace::view::project_visible_lines;
use zaroxi_core_editor_buffer::ports::BufferId;

#[test]
fn project_centered_window_and_cursor_marking() {
    // Build a document with 20 lines and place cursor on line 10 (0-based -> 11th line).
    let mut content = String::new();
    for i in 1..=20 {
        content.push_str(&format!("line {}\n", i));
    }

    let cursor = EditorCursor { line: 10, column: 0 }; // 11th line
    let doc = EditorDocument {
        buffer_id: BufferId::from("buf:main.rs"),
        content: Some(content.clone()),
        cursor,
        selection: None,
        line_count: 20,
        current_line: Some("line 11".to_string()),
    };

    // window_size 5 centered on cursor should yield lines 9..13 (1-based 10..14)
    let win = project_visible_lines(&doc, 5, true);
    assert_eq!(win.total_lines, 20);
    assert_eq!(win.lines.len(), 5);
    // top_line should be 9 + 1 = 10 (1-based)
    assert_eq!(win.top_line, 9 + 1); // start index is 9 (0-based) -> top_line 10
    // Ensure one of the lines is marked as cursor
    let cursor_marked = win.lines.iter().any(|l| l.is_cursor_line);
    assert!(cursor_marked);
    // Find the cursor line and verify it corresponds to line 11
    let cur = win.lines.iter().find(|l| l.is_cursor_line).unwrap();
    assert_eq!(cur.line_number, 11);
    assert_eq!(cur.text.trim(), "line 11");
}

#[test]
fn project_top_window_when_not_centered() {
    let mut content = String::new();
    for i in 1..=3 {
        content.push_str(&format!("r{}\n", i));
    }
    let doc = EditorDocument {
        buffer_id: BufferId::from("buf:small"),
        content: Some(content.clone()),
        cursor: EditorCursor { line: 0, column: 0 },
        selection: None,
        line_count: 3,
        current_line: Some("r1".to_string()),
    };

    // Request window of 5 but doc has only 3 lines -> should return all starting at top
    let win = project_visible_lines(&doc, 5, false);
    assert_eq!(win.total_lines, 3);
    assert_eq!(win.top_line, 1);
    assert_eq!(win.lines.len(), 3);
    assert_eq!(win.lines[0].line_number, 1);
    assert!(win.lines.iter().any(|l| l.is_cursor_line));
}
