use zaroxi_interface_desktop::{SelectionView, view_adapter::InterfaceRenderableWindow, view_adapter::InterfaceRenderableLine, view_adapter::InterfaceRenderSpan, view_adapter::InterfaceSpanKind};

#[test]
fn selection_view_present_reports_bounds_and_visibility() {
    // Build spans: "ab" normal, "cd" selection, "ef" normal
    let span1 = InterfaceRenderSpan {
        kind: InterfaceSpanKind::Normal,
        text: "ab".to_string(),
        start_col: 0,
        end_col: 2,
    };
    let span_sel = InterfaceRenderSpan {
        kind: InterfaceSpanKind::Selection,
        text: "cd".to_string(),
        start_col: 2,
        end_col: 4,
    };
    let span2 = InterfaceRenderSpan {
        kind: InterfaceSpanKind::Normal,
        text: "ef".to_string(),
        start_col: 4,
        end_col: 6,
    };

    let line = InterfaceRenderableLine {
        line_number: 1,
        spans: vec![span1, span_sel, span2],
        total_columns: 6,
    };

    let win = InterfaceRenderableWindow {
        top_line: 1,
        total_lines: 1,
        lines: vec![line],
    };

    let sv = SelectionView::from_window(&win).expect("selection present");
    assert_eq!(sv.start.line, 1);
    assert_eq!(sv.start.column, 2);
    assert_eq!(sv.end.line, 1);
    assert_eq!(sv.end.column, 4);
    assert!(sv.visible_in_window);
}

#[test]
fn selection_view_absent_returns_none() {
    let span1 = InterfaceRenderSpan {
        kind: InterfaceSpanKind::Normal,
        text: "abcd".to_string(),
        start_col: 0,
        end_col: 4,
    };
    let span_cursor = InterfaceRenderSpan {
        kind: InterfaceSpanKind::Cursor,
        text: "".to_string(),
        start_col: 2,
        end_col: 2,
    };
    let line = InterfaceRenderableLine {
        line_number: 1,
        spans: vec![span1, span_cursor],
        total_columns: 4,
    };
    let win = InterfaceRenderableWindow {
        top_line: 1,
        total_lines: 1,
        lines: vec![line],
    };

    let sv = SelectionView::from_window(&win);
    assert!(sv.is_none());
}
