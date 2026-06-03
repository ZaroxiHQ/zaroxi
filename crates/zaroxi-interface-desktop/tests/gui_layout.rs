use zaroxi_interface_desktop::desktop::DesktopComposition;
use zaroxi_interface_desktop::gui::{ShellFrame, Size};

#[test]
fn canonical_layout_contains_expected_regions() {
    let size = Size { width: 1280, height: 800 };
    let shell = ShellFrame::new(size, false);
    let comp = DesktopComposition::new();
    let lines = shell.render_lines(Some(&comp));

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

    // GUI-2: assert presence of chrome/widget placeholders produced by widgets module.
    let widget_expect = [
        "toolbar.brand",
        "app_rail.icons",
        "app_rail.avatar_slot",
        "sidebar.search_field",
        "sidebar.section: PROJECT",
        "bottom_dock.tabs",
        "bottom_dock.problems_count",
        "status.line_col",
        "ai.header.title",
    ];

    for name in widget_expect.iter() {
        let found = lines.iter().any(|l| l.contains(name));
        assert!(found, "missing widget/chrome '{}' in transcript: {:?}", name, lines);
    }
}
