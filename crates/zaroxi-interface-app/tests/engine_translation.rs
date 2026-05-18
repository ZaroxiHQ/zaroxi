use zaroxi_interface_app::ShellFrameViewModel;
use zaroxi_interface_app::shell_frame::{
    ShellFrameModel as AppShellFrameModel, TextView as AppTextView, SelectionView as AppSelectionView,
    Position as AppPosition,
};
use zaroxi_interface_desktop::TextView as DesktopTextView;
use zaroxi_interface_desktop::projections::shell_frame::ShellFrameModel as DesktopShellFrameModel;

/// Ensure the conversion from ShellFrameViewModel -> EngineShellViewInput preserves
/// the semantic, non-visual pieces (visible lines, cursor, viewport, status, shell chrome).
#[test]
fn translation_from_shell_frame_view_model_preserves_semantic_data() {
    let tv = DesktopTextView {
        top_line: 1,
        total_lines: 2,
        lines: vec!["line-a".to_string(), "line-b".to_string()],
        cursor_line: Some(2),
        cursor_column: Some(4),
    };

    // Build a desktop projection instance (tests may construct desktop types).
    let desktop_frame = DesktopShellFrameModel {
        session_identity: None,
        shell_chrome: Some("chrome-v1".to_string()),
        active_text_view: Some(tv.clone()),
        selection_view: None,
        viewport_summary: Some("top_visible_line=1 visible_line_count=2 total_lines=2 cursor_visible=true anchoring=None".to_string()),
        status_text: Some("Ready".to_string()),
        last_command: Some("echo hi".to_string()),
        last_event: None,
    };

    // Convert the desktop projection into the application-local ShellFrameModel
    // before supplying it to the application view-model API.
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

    let mut vm = ShellFrameViewModel::new();
    vm.set(app_frame);

    let input = vm.to_engine_input();
    assert_eq!(input.top_line, tv.top_line as u32);
    assert_eq!(input.total_lines, tv.total_lines as u32);
    assert_eq!(input.lines, tv.lines);
    assert_eq!(input.cursor_line, tv.cursor_line.map(|c| c as u32));
    assert_eq!(input.cursor_column, tv.cursor_column.map(|c| c as u32));
    assert!(input.selection.is_none());
    assert_eq!(input.viewport_summary, vm.viewport());
    assert_eq!(input.status_text, vm.status_text());
    assert_eq!(input.shell_chrome, vm.shell_chrome());
    assert_eq!(input.last_command, vm.last_command());
}
