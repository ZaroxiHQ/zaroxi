use zaroxi_interface_desktop::presenters::gpu_shell::{
    GpuShellPresenter, TabAction, TabStrip, apply_tab_action, compute_tab_action_target,
};

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
    let opened = vec![("x".to_string(), "X".to_string()), ("y".to_string(), "Y".to_string())];
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
    let target =
        compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, Some("a"));
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
    let target =
        compute_tab_action_target(TabAction::ActivatePrevious { wrap: true }, &opened, Some("b"));
    assert_eq!(target.as_deref(), Some("a"));
}

#[test]
fn action_no_buffers_is_noop() {
    let opened: Vec<(String, String)> = vec![];
    let t_next = compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, None);
    let t_prev =
        compute_tab_action_target(TabAction::ActivatePrevious { wrap: true }, &opened, None);
    assert_eq!(t_next, None);
    assert_eq!(t_prev, None);
}

#[test]
fn action_one_buffer_stays_same() {
    let opened = vec![("solo".to_string(), "solo.rs".to_string())];
    // With wrap true or false, still the same single buffer for both directions.
    let t_next =
        compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, Some("solo"));
    let t_prev = compute_tab_action_target(
        TabAction::ActivatePrevious { wrap: false },
        &opened,
        Some("solo"),
    );
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
    let t_prev =
        compute_tab_action_target(TabAction::ActivatePrevious { wrap: true }, &opened, None);
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
    let res = apply_tab_action(TabAction::ActivateNext { wrap: true }, &opened, None, |_id| {
        applied = true;
    });
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

fn sample_pixel(buf: &Vec<u8>, width: u32, x: u32, y: u32) -> [u8; 4] {
    let idx = ((y * width + x) * 4) as usize;
    [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
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

    // Derive tab-bar geometry using the same rules as the presenter to avoid
    // depending on fixed pixel offsets that changed after the chrome tightening.
    let num = ts.tabs.len() as u32;
    let tab_bar_h = std::cmp::min(14u32, regions.chrome.height.saturating_sub(4));
    let tab_bar_y = regions.chrome.y + (regions.chrome.height.saturating_sub(tab_bar_h) / 2);
    let base_w = if num > 0 { regions.chrome.width / num } else { 0 };
    let mut x0 = regions.chrome.x;

    // Active tab color per presenter: [255,200,0,255]
    let active_color = [255u8, 200u8, 0u8, 255u8];

    // For each tab compute its bounds and search a small interior box for the
    // active color. For the single-tab case we expect the active color to
    // appear somewhere inside the tab body.
    let mut found_active = false;
    for i in 0..num {
        let mut w = base_w;
        if (i + 1) == num {
            // last tab takes remainder
            let consumed = base_w.saturating_mul(num.saturating_sub(1));
            w = regions.chrome.width.saturating_sub(consumed);
        }
        if w == 0 {
            continue;
        }

        // define an interior sampling box (avoid borders)
        let sx = x0.saturating_add(w / 8);
        let ex = x0.saturating_add((w * 7) / 8);
        let sy = tab_bar_y.saturating_add(tab_bar_h / 8);
        let ey = tab_bar_y.saturating_add((tab_bar_h * 7) / 8);

        for yy in sy..=ey {
            for xx in sx..=ex {
                let px = sample_pixel(&buf, width, xx, yy);
                if px == active_color {
                    found_active = true;
                    break;
                }
            }
            if found_active {
                break;
            }
        }

        x0 = x0.saturating_add(w);
    }

    assert!(
        found_active,
        "expected to find active-tab color somewhere inside the single tab region"
    );
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

    // Derive tab geometry the same way presenter does.
    let num = ts.tabs.len() as u32;
    let tab_bar_h = std::cmp::min(14u32, regions.chrome.height.saturating_sub(4));
    let tab_bar_y = regions.chrome.y + (regions.chrome.height.saturating_sub(tab_bar_h) / 2);
    let base_w = if num > 0 { regions.chrome.width / num } else { 0 };
    let mut x0 = regions.chrome.x;

    let active_color = [255u8, 200u8, 0u8, 255u8];
    let inactive_color = [180u8, 180u8, 180u8, 255u8];

    // For each tab, sample an interior box and assert that the expected color
    // appears somewhere inside that tab's region. This avoids brittle single-pixel
    // assertions while still verifying both ordering and visual markers.
    for (i, tab) in ts.tabs.iter().enumerate() {
        let mut w = base_w;
        if (i as u32) + 1 == num {
            let consumed = base_w.saturating_mul(num.saturating_sub(1));
            w = regions.chrome.width.saturating_sub(consumed);
        }
        if w == 0 {
            x0 = x0.saturating_add(w);
            continue;
        }

        let sx = x0.saturating_add(w / 8);
        let ex = x0.saturating_add((w * 7) / 8);
        let sy = tab_bar_y.saturating_add(tab_bar_h / 8);
        let ey = tab_bar_y.saturating_add((tab_bar_h * 7) / 8);

        let mut found = false;
        for yy in sy..=ey {
            for xx in sx..=ex {
                let px = sample_pixel(&buf, width, xx, yy);
                if tab.active {
                    if px == active_color {
                        found = true;
                        break;
                    }
                } else {
                    if px == inactive_color {
                        found = true;
                        break;
                    }
                }
            }
            if found {
                break;
            }
        }

        assert!(
            found,
            "expected to find expected color inside tab index {} (display={})",
            i, tab.display
        );
        x0 = x0.saturating_add(w);
    }
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

    // Derive tab geometry like the presenter does.
    let num = ts.tabs.len() as u32;
    let tab_bar_h = std::cmp::min(14u32, regions.chrome.height.saturating_sub(4));
    let tab_bar_y = regions.chrome.y + (regions.chrome.height.saturating_sub(tab_bar_h) / 2);
    let base_w = if num > 0 { regions.chrome.width / num } else { 0 };

    // Render initial state (active = a)
    let mut buf1 = vec![0u8; (width as usize) * (height as usize) * 4];
    GpuShellPresenter::paint_to_buffer_with_tabs(width, height, &mut buf1, &regions, &ts);

    // Helper to assert that a given tab index has the active color somewhere inside it.
    let active_color = [255u8, 200u8, 0u8, 255u8];
    let mut x0 = regions.chrome.x;
    for i in 0..(num as usize) {
        let mut w = base_w;
        if (i as u32) + 1 == num {
            let consumed = base_w.saturating_mul(num.saturating_sub(1));
            w = regions.chrome.width.saturating_sub(consumed);
        }
        if w == 0 {
            x0 = x0.saturating_add(w);
            continue;
        }

        if i == 0 {
            // verify tab 0 (initial active) contains active color
            let sx = x0.saturating_add(w / 8);
            let ex = x0.saturating_add((w * 7) / 8);
            let sy = tab_bar_y.saturating_add(tab_bar_h / 8);
            let ey = tab_bar_y.saturating_add((tab_bar_h * 7) / 8);
            let mut found = false;
            for yy in sy..=ey {
                for xx in sx..=ex {
                    if sample_pixel(&buf1, width, xx, yy) == active_color {
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
            assert!(found, "expected initial active tab (index 0) to contain active color");
            break;
        }
        x0 = x0.saturating_add(w);
    }

    // Compute next active and update TabStrip (simulate Ctrl+Tab)
    let target =
        compute_tab_action_target(TabAction::ActivateNext { wrap: true }, &opened, Some("a"));
    assert_eq!(target.as_deref(), Some("b"));
    ts = ts.with_active_id(target.as_deref().unwrap());

    // Render new state (active = b)
    let mut buf2 = vec![0u8; (width as usize) * (height as usize) * 4];
    GpuShellPresenter::paint_to_buffer_with_tabs(width, height, &mut buf2, &regions, &ts);

    // Verify tab 1 (index 1) now contains the active color.
    let mut x0 = regions.chrome.x;
    let mut found_active_on_index1 = false;
    for i in 0..(num as usize) {
        let mut w = base_w;
        if (i as u32) + 1 == num {
            let consumed = base_w.saturating_mul(num.saturating_sub(1));
            w = regions.chrome.width.saturating_sub(consumed);
        }
        if w == 0 {
            x0 = x0.saturating_add(w);
            continue;
        }

        if i == 1 {
            let sx = x0.saturating_add(w / 8);
            let ex = x0.saturating_add((w * 7) / 8);
            let sy = tab_bar_y.saturating_add(tab_bar_h / 8);
            let ey = tab_bar_y.saturating_add((tab_bar_h * 7) / 8);
            for yy in sy..=ey {
                for xx in sx..=ex {
                    if sample_pixel(&buf2, width, xx, yy) == active_color {
                        found_active_on_index1 = true;
                        break;
                    }
                }
                if found_active_on_index1 {
                    break;
                }
            }
            break;
        }

        x0 = x0.saturating_add(w);
    }

    assert!(
        found_active_on_index1,
        "expected tab index 1 to contain active color after navigation"
    );
}

#[test]
fn render_status_region_and_text() {
    // Ensure status/footer region renders and that status text produces an observable change
    // inside the status band (i.e. text glyphs or indicator alter pixels from the base fill).
    let width: u32 = 240;
    let height: u32 = 120;
    let chrome_h: u32 = 20;
    let status_h: u32 = 12;

    let mut regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    // Provide a visible status string so the presenter emits the status Text op.
    regions.status_text = Some("status: OK".to_string());

    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];
    GpuShellPresenter::paint_to_buffer(width, height, &mut buf, &regions);

    // Sample interior of the status band (avoid the 1px border row at its top).
    let sx = regions.status.x.saturating_add(4);
    let sy = regions.status.y.saturating_add(2);
    let ex = regions.status.x.saturating_add(regions.status.width.saturating_sub(4));
    let ey = regions.status.y.saturating_add(regions.status.height.saturating_sub(2));

    let mut found_changed = false;
    // The default status fill color from the presenter is [48,48,56,255]
    let base_color = [48u8, 48u8, 56u8, 255u8];

    for y in sy..std::cmp::min(ey, height - 1) {
        for x in sx..std::cmp::min(ex, width - 1) {
            let idx = ((y * width + x) * 4) as usize;
            if idx + 4 <= buf.len() {
                let px = [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]];
                if px != base_color {
                    found_changed = true;
                    break;
                }
            }
        }
        if found_changed {
            break;
        }
    }

    assert!(
        found_changed,
        "expected status text or indicator to modify at least one pixel inside the status band"
    );
}
