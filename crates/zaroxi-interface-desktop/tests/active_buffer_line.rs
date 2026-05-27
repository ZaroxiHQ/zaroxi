use zaroxi_interface_desktop::projections::active_buffer_line::ActiveBufferLine;

#[test]
fn absent_before_first_refresh() {
    // revision == 0 => before first refresh => absent even if active_buffer present.
    let got =
        ActiveBufferLine::from_parts(0, Some("buf-123".to_string()), Some("main.rs".to_string()));
    assert!(got.is_none(), "Expected ActiveBufferLine to be absent before first refresh");
}

#[test]
fn present_after_first_refresh() {
    // revision > 0 and active_buffer present => projection should be present.
    let got =
        ActiveBufferLine::from_parts(1, Some("buf-123".to_string()), Some("main.rs".to_string()));
    assert!(got.is_some(), "Expected ActiveBufferLine to be present after first refresh");
    let abl = got.unwrap();
    assert_eq!(abl.buffer_id, "buf-123");
    assert_eq!(abl.display.as_deref(), Some("main.rs"));
    assert_eq!(abl.render(), "active_buffer=buf-123 display=main.rs");
}
