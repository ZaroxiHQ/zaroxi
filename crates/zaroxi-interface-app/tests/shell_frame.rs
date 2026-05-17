use zaroxi_interface_app::shell_frame::ShellFrameViewModel;
use zaroxi_interface_desktop::projections::shell_frame::ShellFrameModel;
use zaroxi_interface_desktop::TextView;

#[test]
fn lifecycle_absent_before_present_after() {
    // Start with an empty view model: frame must be absent.
    let mut vm = ShellFrameViewModel::new();
    assert!(!vm.is_present(), "frame must be absent initially");

    // Build a minimal ShellFrameModel with an active TextView (mandatory piece).
    let tv = TextView {
        top_line: 1,
        total_lines: 1,
        lines: vec!["hello".to_string()],
        cursor_line: Some(1),
        cursor_column: Some(0),
    };

    let viewport = "top_visible_line=1 visible_line_count=1 total_lines=1 cursor_visible=true anchoring=None".to_string();
    let frame = ShellFrameModel {
        session_identity: None,
        shell_chrome: None,
        active_text_view: Some(tv),
        selection_view: None,
        viewport_summary: Some(viewport.clone()),
        status_text: Some("Ready".to_string()),
        last_command: None,
        last_event: None,
    };

    vm.set(frame);
    assert!(vm.is_present(), "frame must be present after set()");

    // Verify read-only accessors expose the semantic pieces (strings cloned for safety).
    assert_eq!(vm.viewport(), Some(viewport));
    assert_eq!(vm.status_text(), Some("Ready".to_string()));
    assert!(vm.active_text_view().is_some());
}
