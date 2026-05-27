use zaroxi_interface_desktop::{
    InterfaceRenderSpan, InterfaceRenderableLine, InterfaceRenderableWindow, InterfaceSpanKind,
    TextView,
};

#[test]
fn text_view_from_window_reports_lines_and_cursor() {
    // Build a single-line renderable window "abcd" with a zero-width cursor at column 2.
    let span1 = InterfaceRenderSpan {
        kind: InterfaceSpanKind::Normal,
        text: "ab".to_string(),
        start_col: 0,
        end_col: 2,
    };
    let span_cursor = InterfaceRenderSpan {
        kind: InterfaceSpanKind::Cursor,
        text: "".to_string(),
        start_col: 2,
        end_col: 2,
    };
    let span2 = InterfaceRenderSpan {
        kind: InterfaceSpanKind::Normal,
        text: "cd".to_string(),
        start_col: 2,
        end_col: 4,
    };

    let line = InterfaceRenderableLine {
        line_number: 1,
        spans: vec![span1, span_cursor, span2],
        total_columns: 4,
    };

    let win = InterfaceRenderableWindow { top_line: 1, total_lines: 1, lines: vec![line] };

    let tv = TextView::from_window(&win).expect("tv present");
    assert_eq!(tv.lines.len(), 1);
    assert_eq!(tv.lines[0], "abcd");
    assert_eq!(tv.cursor_line, Some(1));
    assert_eq!(tv.cursor_column, Some(2));

    let marked = tv.lines_with_cursor_marker("|^|");
    assert_eq!(marked[0], "ab|^|cd");
}
