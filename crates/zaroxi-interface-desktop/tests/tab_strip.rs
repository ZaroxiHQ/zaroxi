use zaroxi_interface_desktop::presenters::gpu_shell::{TabStrip, TabAction, compute_tab_action_target, apply_tab_action};

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

#[test]
fn action_activate_next_updates_active_id() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    // active = a -> next should be b
    let target = compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, Some("a"));
    assert_eq!(target.as_deref(), Some("b"));
}

#[test]
fn action_activate_previous_updates_active_id() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    // active = b -> prev should be a
    let target = compute_tab_action_target(TabAction::ActivatePrevious { wrap: true }, &opened, Some("b"));
    assert_eq!(target.as_deref(), Some("a"));
}

#[test]
fn action_no_buffers_is_noop() {
    let opened: Vec<(String, String)> = vec![];
    let t_next = compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, None);
    let t_prev = compute_tab_action_target(TabAction::ActivatePrevious { wrap: true }, &opened, None);
    assert_eq!(t_next, None);
    assert_eq!(t_prev, None);
}

#[test]
fn action_one_buffer_stays_same() {
    let opened = vec![("solo".to_string(), "solo.rs".to_string())];
    // With wrap true or false, still the same single buffer for both directions.
    let t_next = compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, Some("solo"));
    let t_prev = compute_tab_action_target(TabAction::ActivatePrevious { wrap: false }, &opened, Some("solo"));
    assert_eq!(t_next.as_deref(), Some("solo"));
    assert_eq!(t_prev.as_deref(), Some("solo"));
}

#[test]
fn action_deterministic_fallback_when_active_missing() {
    let opened = vec![
        ("one".to_string(), "One".to_string()),
        ("two".to_string(), "Two".to_string()),
        ("three".to_string(), "Three".to_string()),
    ];
    // active = none -> next picks first, prev picks last
    let t_next = compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, None);
    let t_prev = compute_tab_action_target(TabAction::ActivatePrevious { wrap: true }, &opened, None);
    assert_eq!(t_next.as_deref(), Some("one"));
    assert_eq!(t_prev.as_deref(), Some("three"));
}

// ----- New focused tests: ensure TabAction -> apply_tab_action updates active buffer via the
//           presenter's deterministic resolution path. These tests are small, additive and
//           exercise the normal desktop-side mutation seam by providing a closure that
//           performs the activation (simulating the application-side active-buffer setter).

#[test]
fn action_path_activate_next_updates_active_buffer() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    // Simulate outer application state for active id.
    let mut active: Option<String> = Some("a".to_string());

    // Apply the next action; closure simulates applying the chosen id into app state.
    let res = apply_tab_action(
        TabAction::ActivateNext { wrap: true },
        &opened,
        active.as_deref(),
        |id| {
            active = Some(id.to_string());
        },
    );

    assert_eq!(res.as_deref(), Some("b"));
    assert_eq!(active.as_deref(), Some("b"));
}

#[test]
fn action_path_activate_previous_updates_active_buffer() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    let mut active: Option<String> = Some("b".to_string());

    let res = apply_tab_action(
        TabAction::ActivatePrevious { wrap: true },
        &opened,
        active.as_deref(),
        |id| {
            active = Some(id.to_string());
        },
    );

    assert_eq!(res.as_deref(), Some("a"));
    assert_eq!(active.as_deref(), Some("a"));
}

#[test]
fn action_path_empty_buffers_is_noop() {
    let opened: Vec<(String, String)> = vec![];
    let mut applied = false;
    let res = apply_tab_action(
        TabAction::ActivateNext { wrap: true },
        &opened,
        None,
        |_id| {
            applied = true;
        },
    );
    assert_eq!(res, None);
    assert!(!applied);
}

#[test]
fn action_path_single_buffer_stays_same() {
    let opened = vec![("solo".to_string(), "solo.rs".to_string())];
    let mut active: Option<String> = Some("solo".to_string());

    let res = apply_tab_action(
        TabAction::ActivateNext { wrap: true },
        &opened,
        active.as_deref(),
        |id| {
            active = Some(id.to_string());
        },
    );

    assert_eq!(res.as_deref(), Some("solo"));
    assert_eq!(active.as_deref(), Some("solo"));
}

#[test]
fn action_path_fallback_when_active_missing_selects_deterministically() {
    let opened = vec![
        ("one".to_string(), "One".to_string()),
        ("two".to_string(), "Two".to_string()),
        ("three".to_string(), "Three".to_string()),
    ];
    // active = none
    let mut active: Option<String> = None;

    let res_next = apply_tab_action(
        TabAction::ActivateNext { wrap: true },
        &opened,
        active.as_deref(),
        |id| {
            active = Some(id.to_string());
        },
    );
    assert_eq!(res_next.as_deref(), Some("one"));
    assert_eq!(active.as_deref(), Some("one"));

    // reset and test prev -> should pick last deterministically
    active = None;
    let res_prev = apply_tab_action(
        TabAction::ActivatePrevious { wrap: true },
        &opened,
        active.as_deref(),
        |id| {
            active = Some(id.to_string());
        },
    );
    assert_eq!(res_prev.as_deref(), Some("three"));
    assert_eq!(active.as_deref(), Some("three"));
}
