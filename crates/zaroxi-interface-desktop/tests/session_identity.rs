use zaroxi_interface_desktop::projections::session_identity_line::SessionIdentityLine;

#[test]
fn absent_before_refresh() {
    let line = SessionIdentityLine::new(None, None, None);
    assert!(line.is_empty());
    assert_eq!(line.render(), "session=<none> workspace=<none> path=<none>");
}

#[test]
fn populated_after_open_workspace() {
    let line = SessionIdentityLine::new(
        Some("sess-abc".to_string()),
        Some("ws-42".to_string()),
        Some("/tmp/ws".to_string()),
    );
    assert!(!line.is_empty());
    let s = line.render();
    assert!(s.contains("sess-abc"));
    assert!(s.contains("ws-42"));
    assert!(s.contains("/tmp/ws"));
}

#[test]
fn coherent_after_later_actions() {
    let line = SessionIdentityLine::new(Some("sess-abc".to_string()), Some("ws-42".to_string()), None);
    assert!(!line.is_empty());
    assert_eq!(line.session_id.as_deref(), Some("sess-abc"));
    assert_eq!(line.workspace_id.as_deref(), Some("ws-42"));
}
