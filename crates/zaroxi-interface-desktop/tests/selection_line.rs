use zaroxi_interface_desktop::projections::selection_line::SelectionLine;

#[test]
fn absent_before_refresh_and_when_no_selection() {
    // Lifecycle rule (Phase 46): SelectionLine is absent before first refresh and
    // absent after refresh when no selection exists; present only after refresh
    // when a selection exists.
    //
    // We model "no selection" as None for the optional bounds mapping.
    assert!(SelectionLine::compose_from_optional_bounds(None).is_none());
}

#[test]
fn present_when_selection_exists_and_renders_bounds() {
    // When selection bounds are present we expect a concise one-line rendering.
    let sl = SelectionLine::from_bounds(1, 2, 3, 4, true);
    assert_eq!(sl.render(), "Selection: 1:2 -> 3:4 (visible)");
    let sl2 = SelectionLine::from_bounds(10, 0, 10, 5, false);
    assert_eq!(sl2.render(), "Selection: 10:0 -> 10:5");
}
