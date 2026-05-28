use zaroxi_interface_desktop::gui::{ShellFrame, Size};

#[test]
fn canonical_layout_contains_expected_regions() {
    let size = Size { width: 1280, height: 800 };
    let shell = ShellFrame::new(size);
    let lines = shell.render_lines();

    // Check that all required region ids/names appear in the transcript.
    let expected = [
        "app_rail",
        "sidebar",
        "editor_header",
        "editor_content",
        "minimap_lane",
        "bottom_dock",
        "ai_panel_header",
        "ai_panel_content",
        "status_bar",
    ];

    for name in expected.iter() {
        let found = lines.iter().any(|l| l.contains(name));
        assert!(found, "missing region '{}' in transcript: {:?}", name, lines);
    }
}
