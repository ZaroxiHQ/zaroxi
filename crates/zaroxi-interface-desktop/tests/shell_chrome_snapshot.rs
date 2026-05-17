use zaroxi_interface_desktop::projections::shell_chrome_snapshot::ShellChromeSnapshot;

#[test]
fn absent_when_any_mandatory_missing() {
    // missing session
    assert!(ShellChromeSnapshot::compose(None, Some("buf".to_string()), Some("1:1".to_string()), Some("OK".to_string()), None).is_none());
    // missing active buffer
    assert!(ShellChromeSnapshot::compose(Some("sid".to_string()), None, Some("1:1".to_string()), Some("OK".to_string()), None).is_none());
    // missing location
    assert!(ShellChromeSnapshot::compose(Some("sid".to_string()), Some("buf".to_string()), None, Some("OK".to_string()), None).is_none());
    // missing status
    assert!(ShellChromeSnapshot::compose(Some("sid".to_string()), Some("buf".to_string()), Some("1:1".to_string()), None, None).is_none());
}

#[test]
fn present_when_all_mandatory_present_and_optional_last_command() {
    let snap = ShellChromeSnapshot::compose(
        Some("session:abc".to_string()),
        Some("buffer.rs".to_string()),
        Some("10:5".to_string()),
        Some("Ready".to_string()),
        None,
    );
    assert!(snap.is_some());
    let s = snap.unwrap();
    assert_eq!(s.session, "session:abc");
    assert_eq!(s.active_buffer, "buffer.rs");
    assert_eq!(s.location, "10:5");
    assert_eq!(s.status, "Ready");
    assert!(s.last_command.is_none());

    let snap2 = ShellChromeSnapshot::compose(
        Some("session:abc".to_string()),
        Some("buffer.rs".to_string()),
        Some("10:5".to_string()),
        Some("Ready".to_string()),
        Some("do thing".to_string()),
    );
    assert!(snap2.is_some());
    let s2 = snap2.unwrap();
    assert_eq!(s2.last_command.as_deref(), Some("do thing"));
}
