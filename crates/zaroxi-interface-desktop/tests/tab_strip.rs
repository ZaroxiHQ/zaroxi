use zaroxi_interface_desktop::presenters::gpu_shell::{TabStrip, TabAction, compute_tab_action_target, apply_tab_action, GpuShellPresenter};

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
    // Snapshot the current active value to avoid simultaneous immutable/mutable borrows
    // when passing into `apply_tab_action` while the closure mutates `active`.
    let current_active = active.clone();

    // Apply the next action; closure simulates applying the chosen id into app state.
    let res = apply_tab_action(
        TabAction::ActivateNext { wrap: true },
        &opened,
        current_active.as_deref(),
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
    let current_active = active.clone();

    let res = apply_tab_action(
        TabAction::ActivatePrevious { wrap: true },
        &opened,
        current_active.as_deref(),
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
    let current_active = active.clone();

    let res = apply_tab_action(
        TabAction::ActivateNext { wrap: true },
        &opened,
        current_active.as_deref(),
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
    let current_active = active.clone();

    let res_next = apply_tab_action(
        TabAction::ActivateNext { wrap: true },
        &opened,
        current_active.as_deref(),
        |id| {
            active = Some(id.to_string());
        },
    );
    assert_eq!(res_next.as_deref(), Some("one"));
    assert_eq!(active.as_deref(), Some("one"));

    // reset and test prev -> should pick last deterministically
    active = None;
    let current_active = active.clone();
    let res_prev = apply_tab_action(
        TabAction::ActivatePrevious { wrap: true },
        &opened,
        current_active.as_deref(),
        |id| {
            active = Some(id.to_string());
        },
    );
    assert_eq!(res_prev.as_deref(), Some("three"));
    assert_eq!(active.as_deref(), Some("three"));
}

// ----------------- New renderer-visible tab strip tests -----------------

fn sample_pixel(buf: &Vec<u8>, width: u32, x: u32, y: u32) -> [u8;4] {
    let idx = ((y * width + x) * 4) as usize;
    [buf[idx], buf[idx+1], buf[idx+2], buf[idx+3]]
}

#[test]
fn render_tab_strip_no_buffers_shows_chrome_only() {
    let opened: Vec<(String, String)> = vec![];
    let ts = TabStrip::from_opened_and_active(&opened, None);

    let width: u32 = 200;
    let height: u32 = 100;
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;
    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

    // Render with an empty TabStrip.
    GpuShellPresenter::paint_to_buffer_with_tabs(width, height, &mut buf, &regions, &ts);

    // Sample a pixel in the chrome area that would be overlapped by a tab when tabs exist.
    let px = sample_pixel(&buf, width, 10, regions.chrome.y + 2);
    // Expect chrome base color (no tabs painted).
    assert_eq!(px, [32u8, 32u8, 40u8, 255u8]);
}

#[test]
fn render_tab_strip_one_buffer_shows_active_tab() {
    let opened = vec![("solo".to_string(), "solo.rs".to_string())];
    let ts = TabStrip::from_opened_and_active(&opened, Some("solo"));

    let width: u32 = 200;
    let height: u32 = 100;
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;
    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

    GpuShellPresenter::paint_to_buffer_with_tabs(width, height, &mut buf, &regions, &ts);

    // For single tab, it spans the chrome width; pick a center x inside chrome.
    let x = regions.chrome.width / 2;
    let y = regions.chrome.y + 3;
    let px = sample_pixel(&buf, width, x, y);
    // Active tab fill color per presenter: [255,200,0,255]
    assert_eq!(px, [255u8, 200u8, 0u8, 255u8]);
}

#[test]
fn render_tab_strip_multiple_buffers_order_and_active_marker() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    // active = b
    let ts = TabStrip::from_opened_and_active(&opened, Some("b"));

    let width: u32 = 210; // choose divisible width to make deterministic per-tab widths simple
    let height: u32 = 100;
    let chrome_h: u32 = 30;
    let status_h: u32 = 10;
    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

    GpuShellPresenter::paint_to_buffer_with_tabs(width, height, &mut buf, &regions, &ts);

    // Each tab width should be roughly chrome.width / 3.
    let tab_w = regions.chrome.width / 3;
    // Sample center of first tab (inactive), second tab (active), third tab (inactive)
    let x1 = regions.chrome.x + (tab_w / 2);
    let x2 = regions.chrome.x + tab_w + (tab_w / 2);
    let x3 = regions.chrome.x + tab_w * 2 + (tab_w / 2);
    let y = regions.chrome.y + 3;

    let p1 = sample_pixel(&buf, width, x1, y);
    let p2 = sample_pixel(&buf, width, x2, y);
    let p3 = sample_pixel(&buf, width, x3, y);

    // inactive color: [180,180,180,255], active color: [255,200,0,255]
    assert_eq!(p1, [180u8, 180u8, 180u8, 255u8]);
    assert_eq!(p2, [255u8, 200u8, 0u8, 255u8]);
    assert_eq!(p3, [180u8, 180u8, 180u8, 255u8]);
}

#[test]
fn render_tab_strip_reflects_keyboard_navigation() {
    let opened = vec![
        ("a".to_string(), "A".to_string()),
        ("b".to_string(), "B".to_string()),
        ("c".to_string(), "C".to_string()),
    ];
    // start active = a
    let mut ts = TabStrip::from_opened_and_active(&opened, Some("a"));

    let width: u32 = 210;
    let height: u32 = 100;
    let chrome_h: u32 = 30;
    let status_h: u32 = 10;
    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);

    // Render initial state (active = a)
    let mut buf1 = vec![0u8; (width as usize) * (height as usize) * 4];
    GpuShellPresenter::paint_to_buffer_with_tabs(width, height, &mut buf1, &regions, &ts);
    let tab_w = regions.chrome.width / 3;
    let x_a = regions.chrome.x + (tab_w / 2);
    let y = regions.chrome.y + 3;
    let p_a = sample_pixel(&buf1, width, x_a, y);
    assert_eq!(p_a, [255u8, 200u8, 0u8, 255u8]);

    // Compute next active and update TabStrip (simulate Ctrl+Tab)
    let target = compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, Some("a"));
    assert_eq!(target.as_deref(), Some("b"));
    ts = ts.with_active_id(target.as_deref().unwrap());

    // Render new state (active = b)
    let mut buf2 = vec![0u8; (width as usize) * (height as usize) * 4];
    GpuShellPresenter::paint_to_buffer_with_tabs(width, height, &mut buf2, &regions, &ts);
    let x_b = regions.chrome.x + tab_w + (tab_w / 2);
    let p_b = sample_pixel(&buf2, width, x_b, y);
    assert_eq!(p_b, [255u8, 200u8, 0u8, 255u8]);
}
