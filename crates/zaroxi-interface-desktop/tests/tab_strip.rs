use zaroxi_interface_desktop::presenters::gpu_shell::TabStrip;

#[test]
fn no_buffers_empty_tab_strip() {
    let opened: Vec<(String, String)> = vec![];
    let ts = TabStrip::from_opened_and_active(&opened, None);
    assert!(ts.tabs.is_empty());
}

#[test]
fn one_buffer_active() {
    let opened = vec![("buf1".to_string(), "main.rs".to_string())];
    let ts = TabStrip::from_opened_and_active(&opened, Some("buf1"));
    assert_eq!(ts.tabs.len(), 1);
    let t = &ts.tabs[0];
    assert_eq!(t.id, "buf1");
    assert_eq!(t.display, "main.rs");
    assert!(t.active);
    assert_eq!(t.index, 0);
}

#[test]
fn multiple_buffers_order_preserved_and_single_active() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    let ts = TabStrip::from_opened_and_active(&opened, Some("b"));
    assert_eq!(ts.tabs.len(), 3);
    assert_eq!(ts.tabs[0].id, "a");
    assert_eq!(ts.tabs[1].id, "b");
    assert_eq!(ts.tabs[2].id, "c");
    let active_count = ts.tabs.iter().filter(|t| t.active).count();
    assert_eq!(active_count, 1);
    assert_eq!(ts.tabs[1].display, "B");
}

#[test]
fn active_none_when_no_match() {
    let opened = vec![
        ("x".to_string(), "X".to_string()),
        ("y".to_string(), "Y".to_string()),
    ];
    let ts = TabStrip::from_opened_and_active(&opened, Some("missing"));
    assert!(ts.tabs.iter().all(|t| !t.active));
}

/// New tests for deterministic tab navigation behavior.
///
/// Navigation rules (chosen deterministically):
/// - Wrap-around behavior is configurable per-invocation (wrap=true will wrap).
/// - next: advances forward in ordering; at end -> wrap to first when wrap==true, else stay.
/// - prev: moves backward; at start -> wrap to last when wrap==true, else stay.
/// - When there is no active tab:
///     - next -> selects first tab
///     - prev -> selects last tab
/// - Empty tab list -> no-op (None)
#[test]
fn navigation_no_buffers_is_noop() {
    let opened: Vec<(String, String)> = vec![];
    let ts = TabStrip::from_opened_and_active(&opened, None);
    assert_eq!(ts.next_active_id(true), None);
    assert_eq!(ts.prev_active_id(true), None);
}

#[test]
fn navigation_one_buffer_stays_same() {
    let opened = vec![("solo".to_string(), "solo.rs".to_string())];
    let ts = TabStrip::from_opened_and_active(&opened, Some("solo"));
    // With wrap true or false, still the same single buffer.
    assert_eq!(ts.next_active_id(true).as_deref(), Some("solo"));
    assert_eq!(ts.prev_active_id(true).as_deref(), Some("solo"));
    assert_eq!(ts.next_active_id(false).as_deref(), Some("solo"));
    assert_eq!(ts.prev_active_id(false).as_deref(), Some("solo"));
}

#[test]
fn navigation_multiple_next_and_prev_with_wrap() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    // active = b
    let ts = TabStrip::from_opened_and_active(&opened, Some("b"));
    // next should be c
    assert_eq!(ts.next_active_id(true).as_deref(), Some("c"));
    // prev should be a
    assert_eq!(ts.prev_active_id(true).as_deref(), Some("a"));
    // next from c wraps to a when wrap=true
    let ts_c = TabStrip::from_opened_and_active(&opened, Some("c"));
    assert_eq!(ts_c.next_active_id(true).as_deref(), Some("a"));
    // prev from a wraps to c when wrap=true
    let ts_a = TabStrip::from_opened_and_active(&opened, Some("a"));
    assert_eq!(ts_a.prev_active_id(true).as_deref(), Some("c"));
}

#[test]
fn navigation_multiple_next_and_prev_without_wrap_clamps() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    // active = c (last). next with wrap=false => stays c
    let ts_c = TabStrip::from_opened_and_active(&opened, Some("c"));
    assert_eq!(ts_c.next_active_id(false).as_deref(), Some("c"));
    // active = a (first). prev with wrap=false => stays a
    let ts_a = TabStrip::from_opened_and_active(&opened, Some("a"));
    assert_eq!(ts_a.prev_active_id(false).as_deref(), Some("a"));
}

#[test]
fn navigation_when_no_active_selects_first_or_last_deterministically() {
    let opened = vec![
        ("one".to_string(), "One".to_string()),
        ("two".to_string(), "Two".to_string()),
        ("three".to_string(), "Three".to_string()),
    ];
    // active = none
    let ts = TabStrip::from_opened_and_active(&opened, None);
    // next should pick first deterministically
    assert_eq!(ts.next_active_id(true).as_deref(), Some("one"));
    // prev should pick last deterministically
    assert_eq!(ts.prev_active_id(true).as_deref(), Some("three"));
}
