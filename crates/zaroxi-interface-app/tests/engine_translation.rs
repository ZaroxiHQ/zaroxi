use zaroxi_interface_app::ShellFrameViewModel;
use zaroxi_interface_desktop::TextView;
use zaroxi_interface_desktop::projections::shell_frame::ShellFrameModel;

/// Ensure the conversion from ShellFrameViewModel -> EngineShellViewInput preserves
/// the semantic, non-visual pieces (visible lines, cursor, viewport, status, shell chrome).
#[test]
fn translation_from_shell_frame_view_model_preserves_semantic_data() {
    let tv = TextView {
        top_line: 1,
        total_lines: 2,
        lines: vec!["line-a".to_string(), "line-b".to_string()],
        cursor_line: Some(2),
        cursor_column: Some(4),
    };

    let frame = ShellFrameModel {
        session_identity: None,
        shell_chrome: Some("chrome-v1".to_string()),
        active_text_view: Some(tv.clone()),
        selection_view: None,
        viewport_summary: Some("top_visible_line=1 visible_line_count=2 total_lines=2 cursor_visible=true anchoring=None".to_string()),
        status_text: Some("Ready".to_string()),
        last_command: Some("echo hi".to_string()),
        last_event: None,
    };

    let mut vm = ShellFrameViewModel::new();
    vm.set(frame);

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
