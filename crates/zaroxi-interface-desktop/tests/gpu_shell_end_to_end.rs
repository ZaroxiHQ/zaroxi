use zaroxi_interface_desktop::gpu_shell_runtime::apply_action_and_get_regions;
use zaroxi_interface_desktop::events::Action;

#[test]
fn apply_set_active_buffer_returns_ordered_regions() {
    let width: u32 = 200;
    let height: u32 = 100;

    let before = zaroxi_interface_desktop::gpu_shell_adapter::view_model_to_regions_from_scratch(width, height);
    let after = apply_action_and_get_regions(Action::SetActiveBuffer("test_buffer".to_string()), width, height);

    // Basic structural assertions: origins and widths preserved, and vertical ordering.
    assert_eq!(after.chrome.x, 0);
    assert_eq!(after.content.x, 0);
    assert_eq!(after.status.x, 0);

    assert_eq!(after.chrome.width, width);
    assert_eq!(after.content.width, width);
    assert_eq!(after.status.width, width);

    assert!(after.chrome.y < after.content.y);
    assert!(after.content.y < after.status.y);

    // Sanity: regions should be derived from some model; at minimum they should
    // match presenter's ordering invariants (above).
    // It's acceptable if 'after' equals 'before' in this phase; the test proves
    // the end-to-end path exercised and returns valid regions.
    let _ = (before, after);
}
