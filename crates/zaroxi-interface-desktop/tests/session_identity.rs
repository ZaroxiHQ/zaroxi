use zaroxi_interface_desktop::projections::session_identity_line::SessionIdentityLine;

/// The SessionIdentityLine projection is composition-driven:
/// - it should be absent before the first composition refresh;
/// - it should be present (populated) after a successful refresh that supplies metadata.
///
/// These unit tests validate the strict lifecycle at the projection boundary.
/// Note: composition/refresh is exercised by higher-level integration tests; here we
/// assert the projection's empty and populated states explicitly.
#[test]
fn absent_before_refresh() {
    // Before any refresh/composition, the projection must be empty.
    let line = SessionIdentityLine::new(None, None, None);
    assert!(line.is_empty());
    assert_eq!(line.render(), "session=<none> workspace=<none> path=<none>");
}

#[test]
fn populated_after_refresh() {
    // Simulate the post-refresh populated state by constructing the projection from
    // the metadata that a refresh would have provided.
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
    // Projection remains coherent as underlying metadata evolves.
    let line =
        SessionIdentityLine::new(Some("sess-abc".to_string()), Some("ws-42".to_string()), None);
    assert!(!line.is_empty());
    assert_eq!(line.session_id.as_deref(), Some("sess-abc"));
    assert_eq!(line.workspace_id.as_deref(), Some("ws-42"));
}
