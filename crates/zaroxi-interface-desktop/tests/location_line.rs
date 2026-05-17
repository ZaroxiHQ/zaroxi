use zaroxi_interface_desktop::projections::location_line::LocationLine;

#[test]
fn absent_before_first_refresh() {
    // revision == 0 => before first refresh => absent even if cursor present.
    let got = LocationLine::from_parts(0, Some(10), Some(5), Some("main.rs".to_string()));
    assert!(got.is_none(), "Expected LocationLine to be absent before first refresh");
}

#[test]
fn present_after_first_refresh() {
    // revision > 0 and cursor present => projection should be present.
    let got = LocationLine::from_parts(1, Some(10), Some(5), Some("main.rs".to_string()));
    assert!(got.is_some(), "Expected LocationLine to be present after first refresh");
    let ll = got.unwrap();
    assert_eq!(ll.line, 10);
    assert_eq!(ll.column, 5);
    assert_eq!(ll.display.as_deref(), Some("main.rs"));
    assert_eq!(ll.render(), "cursor=10:5 display=main.rs");
}
