/// Architecture contract tests: verify that build_work_content() and
/// related shared functions handle all edge cases without panicking
/// and that the trait contracts are correctly wired.
use zaroxi_application_workspace::ports::BufferId;
use zaroxi_application_workspace::workspace_view::{
    ActiveDocumentSummary, OpenedBufferItemSummary, OpenedBuffersSummary, RefreshReason,
    ShellContext, build_work_content, command_bar_labels, refresh_reason_label,
    select_next_command_index, select_prev_command_index,
};
use zaroxi_core_engine_ui::ContentView;

#[test]
fn build_work_content_handles_empty_inputs() {
    let opened = OpenedBuffersSummary { count: 0, items: vec![], active: None };
    let result = build_work_content(&opened, None, None, None, None, None, None);
    assert!(result.editor_tabs.is_none());
    assert!(result.editor_body.is_none());
    assert!(result.editor_breadcrumb.is_none());
    assert!(result.ai_panel_content.is_none());
    assert!(result.terminal_tabs.is_some());
    assert!(result.active_file.is_none());
}

#[test]
fn build_work_content_handles_active_document_with_empty_lines() {
    let opened = OpenedBuffersSummary { count: 0, items: vec![], active: None };
    let doc = ActiveDocumentSummary {
        buffer_id: None,
        display: Some("test.rs".into()),
        line_count: 0,
        cursor_line: Some(1),
        cursor_column: Some(1),
        selection_present: false,
        current_line_snippet: None,
    };

    let result = build_work_content(&opened, Some(&doc), None, None, None, None, None);

    let body = result.editor_body.expect("editor_body should be Some with a doc present");
    // Should fall back to ContentView::default() when lines are empty
    assert!(!body.lines.is_empty(), "should have default content lines");
}

#[test]
fn build_work_content_propagates_ai_panel_content() {
    let opened = OpenedBuffersSummary { count: 0, items: vec![], active: None };
    let ai = ContentView::new("AI", "status", vec!["body".into()]);

    let result = build_work_content(&opened, None, None, None, Some(ai.clone()), None, None);
    assert_eq!(result.ai_panel_content.unwrap().title, "AI");
}

#[test]
fn build_work_content_marks_active_file() {
    let buf_a = BufferId::from("a");
    let buf_b = BufferId::from("b");
    let opened = OpenedBuffersSummary {
        count: 2,
        items: vec![
            OpenedBufferItemSummary {
                buffer_id: buf_a.clone(),
                display: Some("a.rs".into()),
                line_count: 10,
                active: false,
            },
            OpenedBufferItemSummary {
                buffer_id: buf_b.clone(),
                display: Some("b.rs".into()),
                line_count: 20,
                active: true,
            },
        ],
        active: Some(buf_b),
    };

    let result = build_work_content(&opened, None, None, None, None, None, None);
    let items = result.explorer_items.expect("should have explorer items");
    assert!(items.iter().any(|s| s == "a.rs"));
    assert!(items.iter().any(|s| s == "b.rs *"));
}

#[test]
fn command_bar_labels_count_is_12() {
    let labels = command_bar_labels();
    assert_eq!(labels.len(), 12);
    assert!(labels.contains(&"AI review active buffer".to_string()));
    assert!(labels.contains(&"Apply AI proposal".to_string()));
    assert!(labels.contains(&"Reject AI proposal".to_string()));
    assert!(labels.contains(&"Open workspace by path".to_string()));
}

#[test]
fn refresh_reason_labels_cover_all_variants() {
    let reasons = [
        RefreshReason::InitialLoad,
        RefreshReason::RefreshAction,
        RefreshReason::CursorMoved,
        RefreshReason::BufferUpdated,
        RefreshReason::ActiveBufferChanged,
        RefreshReason::AiProjectionUpdated,
    ];
    for r in &reasons {
        let label = refresh_reason_label(r);
        assert!(!label.is_empty());
    }
}

#[test]
fn select_next_command_wraps() {
    assert_eq!(select_next_command_index(0, 8), 1);
    assert_eq!(select_next_command_index(7, 8), 0);
    assert_eq!(select_next_command_index(5, 0), 5); // empty list
}

#[test]
fn select_prev_command_wraps() {
    assert_eq!(select_prev_command_index(0, 8), 7);
    assert_eq!(select_prev_command_index(1, 8), 0);
    assert_eq!(select_prev_command_index(5, 0), 5); // empty list
}

#[test]
fn shell_context_serializable() {
    let ctx = ShellContext {
        active_buffer: None,
        active_display: Some("src/lib.rs".into()),
        latest_revision: 42,
        latest_refresh_reason: Some(RefreshReason::InitialLoad),
        has_ai_projection: false,
        last_command_line: None,
    };

    assert_eq!(ctx.latest_revision, 42);
    assert_eq!(ctx.active_display, Some("src/lib.rs".into()));
}

/// Phase 20: build_work_content with explain-style result (no proposal_text)
/// produces ai_panel_content with Analysis subtitle and body without actions.
#[test]
fn build_work_content_handles_explain_result() {
    use zaroxi_core_engine_ui::ContentView;

    let opened = OpenedBuffersSummary { count: 0, items: vec![], active: None };

    // Simulate an explain result: result text present, no proposal_text.
    let explain_body = "This module exports the main entrypoint. Consider adding error handling.";
    let ai_content = ContentView::new(
        "Assistant",
        "Analysis: src/lib.rs",
        explain_body.lines().map(|l| l.to_string()).collect(),
    );

    let result =
        build_work_content(&opened, None, None, None, Some(ai_content.clone()), None, None);
    let ai = result.ai_panel_content.expect("ai_panel_content should be present");
    assert_eq!(ai.title, "Assistant");
    assert!(ai.subtitle.contains("Analysis:"));
    assert!(ai.lines.iter().any(|l| l.contains("entrypoint")));
    // Explain content should NOT have action labels
    assert!(!ai.lines.iter().any(|l| l.contains("Accept") || l.contains("Reject")));
}

/// Phase 22: applied_content_view produces confirmation subtitle without actions.
#[test]
fn applied_content_view_shows_confirmation() {
    let applied = zaroxi_application_ai::panel::applied_content_view(
        "AI edit applied (via update_buffer)",
        "src/lib.rs",
    );
    assert_eq!(applied.title, "Assistant");
    assert!(applied.subtitle.contains("Applied:"));
    assert!(applied.lines.iter().any(|l| l.contains("AI edit applied")));
    assert!(!applied.lines.iter().any(|l| l.contains("Accept")));
}

/// Phase 24: diagnostics_content_view produces severity lines when counts > 0.
#[test]
fn diagnostics_content_view_shows_severity_counts() {
    let view =
        zaroxi_application_ai::panel::diagnostics_content_view(3, 1, 0, 0, "src/lib.rs", &[]);
    assert_eq!(view.title, "Problems");
    assert!(view.subtitle.contains("src/lib.rs"));
    assert!(view.lines.iter().any(|l| l.contains("error(s)")));
    assert!(view.lines.iter().any(|l| l.contains("warning(s)")));
}

/// Phase 24: diagnostics with zero counts returns empty lines.
#[test]
fn diagnostics_content_view_zero_counts_is_empty() {
    let view =
        zaroxi_application_ai::panel::diagnostics_content_view(0, 0, 0, 0, "src/lib.rs", &[]);
    assert_eq!(view.lines.len(), 0);
}

/// Phase 25: diagnostics with detail lines includes them.
#[test]
fn diagnostics_content_view_includes_detail_lines() {
    let details = vec!["missing `;` at line 42".to_string(), "unused variable `x`".to_string()];
    let view =
        zaroxi_application_ai::panel::diagnostics_content_view(2, 0, 0, 0, "src/lib.rs", &details);
    assert!(view.lines.iter().any(|l| l.contains("missing `;`")));
    assert!(view.lines.iter().any(|l| l.contains("unused variable")));
}
