use zaroxi_interface_desktop::{DesktopComposition, TextView};
use zaroxi_interface_desktop::projections::shell_frame::ShellFrameModel;

#[test]
fn lifecycle_absent_before_present_after() {
    // Before any refresh/population the composition should not produce a frame.
    let comp = DesktopComposition::new();
    assert!(ShellFrameModel::from_composition(&comp).is_none(), "frame must be absent before composition refresh/population");

    // Simulate the 'after' state by providing the mandatory visible text model directly.
    let tv = TextView {
        top_line: 1,
        total_lines: 1,
        lines: vec!["hello".to_string()],
        cursor_line: Some(1),
        cursor_column: Some(0),
    };

    // The minimal rule: with an active TextView present the frame may be constructed.
    let frame = ShellFrameModel::from_parts(Some(tv), None, None, None, None, None, None, None);
    assert!(frame.is_some(), "frame must be present when mandatory pieces (TextView) exist");
}
