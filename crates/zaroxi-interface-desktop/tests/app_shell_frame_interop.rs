use zaroxi_interface_app::shell_frame::ShellFrameViewModel;
use zaroxi_interface_app::shell_frame::{
    Position as AppPosition, SelectionView as AppSelectionView,
    ShellFrameModel as AppShellFrameModel, TextView as AppTextView,
};
use zaroxi_interface_desktop::TextView as DesktopTextView;
use zaroxi_interface_desktop::projections::shell_frame::ShellFrameModel as DesktopShellFrameModel;

#[test]
fn lifecycle_absent_before_present_after() {
    // Start with an empty view model: frame must be absent.
    let mut vm = ShellFrameViewModel::new();
    assert!(!vm.is_present(), "frame must be absent initially");

    // Build a minimal desktop TextView (tests may construct desktop types).
    let tv = DesktopTextView {
        top_line: 1,
        total_lines: 1,
        lines: vec!["hello".to_string()],
        cursor_line: Some(1),
        cursor_column: Some(0),
    };

    let viewport =
        "top_visible_line=1 visible_line_count=1 total_lines=1 cursor_visible=true anchoring=None"
            .to_string();
    let desktop_frame = DesktopShellFrameModel {
        session_identity: None,
        shell_chrome: None,
        active_text_view: Some(tv),
        selection_view: None,
        viewport_summary: Some(viewport.clone()),
        status_text: Some("Ready".to_string()),
        last_command: None,
        last_event: None,
    };

    // Convert desktop projection into application-local model and set it.
    let app_frame = AppShellFrameModel {
        viewport_summary: desktop_frame.viewport_summary,
        status_text: desktop_frame.status_text,
        shell_chrome: desktop_frame.shell_chrome,
        last_command: desktop_frame.last_command,
        active_text_view: desktop_frame.active_text_view.map(|t| AppTextView {
            top_line: t.top_line,
            total_lines: t.total_lines,
            lines: t.lines,
            cursor_line: t.cursor_line,
            cursor_column: t.cursor_column,
        }),
        selection_view: desktop_frame.selection_view.map(|s| AppSelectionView {
            start: AppPosition { line: s.start.line, column: s.start.column },
            end: AppPosition { line: s.end.line, column: s.end.column },
        }),
    };

    vm.set(app_frame);
    assert!(vm.is_present(), "frame must be present after set()");

    // Verify read-only accessors expose the semantic pieces (strings cloned for safety).
    assert_eq!(vm.viewport(), Some(viewport));
    assert_eq!(vm.status_text(), Some("Ready".to_string()));
    assert!(vm.active_text_view().is_some());
}
