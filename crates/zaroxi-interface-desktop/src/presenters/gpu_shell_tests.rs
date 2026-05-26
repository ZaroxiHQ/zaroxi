use super::*;
use std::rc::Rc;
use std::cell::RefCell;

/// Verify that region mapping produces three ordered regions (chrome above
/// content above status). This keeps the test crate-local and avoids
/// depending on the binary-scoped adapter module.
#[test]
fn map_regions_preserves_order() {
    let width: u32 = 200;
    let height: u32 = 100;
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    // Use the presenter's pure mapping function directly.
    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);

    // Basic structural assertions: x origin, widths, and vertical ordering.
    assert_eq!(regions.chrome.x, 0);
    assert_eq!(regions.content.x, 0);
    assert_eq!(regions.status.x, 0);

    assert_eq!(regions.chrome.width, width);
    assert_eq!(regions.content.width, width);
    assert_eq!(regions.status.width, width);

    // Vertical ordering: chrome starts at 0, content starts after chrome,
    // status starts after content.
    assert!(regions.chrome.y < regions.content.y);
    assert!(regions.content.y < regions.status.y);
}

/// Focused test: ensure semantic region kinds produce deterministic visible
/// differences (thin interior borders) while preserving ordering and marker.
#[test]
fn region_kind_borders_are_deterministic() {
    let width: u32 = 200;
    let height: u32 = 100;
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

    // Paint using the presenter's pure API.
    GpuShellPresenter::paint_to_buffer(width, height, &mut buf, &regions);

    // Helper to sample a pixel (x,y).
    let sample = |x: u32, y: u32| -> [u8; 4] {
        let idx = ((y * width + x) * 4) as usize;
        [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
    };

    // Coordinates chosen to fall inside the 1-pixel interior border for each region.
    // Sample at the top-left edge of each region (x=0) which is the border row when
    // border_thickness == 1. This preserves prior interior-fill samples (e.g. (1,1))
    // while still proving that region-kind borders are rendered deterministically.
    let chrome_pixel = sample(0, 0);
    let content_pixel = sample(0, regions.content.y);
    let status_pixel = sample(0, regions.status.y);

    // Expect the deterministic border colors defined in kind_border_color above.
    assert_eq!(chrome_pixel, [200u8, 80u8, 80u8, 255u8]);
    assert_eq!(content_pixel, [80u8, 140u8, 200u8, 255u8]);
    assert_eq!(status_pixel, [80u8, 200u8, 120u8, 255u8]);

    // Sanity: borders must differ between region kinds.
    assert_ne!(chrome_pixel, content_pixel);
    assert_ne!(content_pixel, status_pixel);
}

/// Focused contract test: ensure the presenter can produce the explicit
/// GpuShellView from ShellRegions and that tiny semantic payloads survive
/// the conversion. This proves the new output contract is available to
/// downstream consumers while preserving prior invariants.
#[test]
fn produce_gpu_shell_view_contract() {
    let width: u32 = 200;
    let height: u32 = 100;
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    // Default mapping produces no semantic payloads.
    let view = GpuShellView::from_shell_regions(&regions);
    assert_eq!(view.chrome.x, regions.chrome.x);
    assert_eq!(view.content.y, regions.content.y);
    assert_eq!(view.marker, regions.marker);
    assert!(view.chrome_label.is_none() && view.status_text.is_none() && view.content_preview.is_none());

    // Ensure payloads propagate through the conversion.
    let mut r2 = regions.clone();
    r2.chrome_label = Some("buf".to_string());
    r2.status_text = Some("status".to_string());
    let view2 = GpuShellView::from_shell_regions(&r2);
    assert_eq!(view2.chrome_label, Some("buf".to_string()));
    assert_eq!(view2.status_text, Some("status".to_string()));
}

/// Focused test: ensure converting a GpuShellView -> GpuPaintPlan produces
/// the expected leading operations (base fills and borders) and preserves
/// rects/colors deterministically.
#[test]
fn paint_plan_from_view_sequence() {
    let width: u32 = 200;
    let height: u32 = 100;
    let chrome_h: u32 = 60;
    let status_h: u32 = 24;

    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let view = GpuShellView::from_shell_regions(&regions);

    let plan = GpuPaintPlan::from_view(&view);

    // Expect at least: background, content fill, chrome fill, status fill, then three borders.
    assert!(plan.ops.len() >= 7);

    // First op should be the full-viewport background FillRect.
    match &plan.ops[0] {
        GpuPaintOp::FillRect(r) => {
            let total_h = regions.chrome.height.saturating_add(regions.content.height).saturating_add(regions.status.height);
            assert_eq!(r.x, 0);
            assert_eq!(r.y, 0);
            assert_eq!(r.width, regions.chrome.width);
            assert_eq!(r.height, total_h);
            assert_eq!(r.color, [220u8, 220u8, 225u8, 255u8]);
        }
        _ => panic!("expected full-viewport background FillRect as first op"),
    }

    // Next three ops should be FillRect for content, chrome, status respectively.
    match &plan.ops[1] {
        GpuPaintOp::FillRect(r) => {
            assert_eq!(r.x, regions.content.x);
            assert_eq!(r.y, regions.content.y);
            assert_eq!(r.width, regions.content.width);
            assert_eq!(r.height, regions.content.height);
            assert_eq!(r.color, [220u8, 220u8, 225u8, 255u8]);
        }
        _ => panic!("expected content FillRect as second op"),
    }

    match &plan.ops[2] {
        GpuPaintOp::FillRect(r) => {
            assert_eq!(r.x, regions.chrome.x);
            assert_eq!(r.y, regions.chrome.y);
            assert_eq!(r.width, regions.chrome.width);
            assert_eq!(r.height, regions.chrome.height);
            assert_eq!(r.color, [32u8, 32u8, 40u8, 255u8]);
        }
        _ => panic!("expected chrome FillRect as third op"),
    }

    match &plan.ops[3] {
        GpuPaintOp::FillRect(r) => {
            assert_eq!(r.x, regions.status.x);
            assert_eq!(r.y, regions.status.y);
            assert_eq!(r.width, regions.status.width);
            assert_eq!(r.height, regions.status.height);
            assert_eq!(r.color, [48u8, 48u8, 56u8, 255u8]);
        }
        _ => panic!("expected status FillRect as fourth op"),
    }

    // Next three should be BorderRect entries (chrome, content, status).
    match &plan.ops[4] {
        GpuPaintOp::BorderRect { rect, thickness } => {
            assert_eq!(thickness, &1u32);
            assert_eq!(rect.x, regions.chrome.x);
        }
        _ => panic!("expected chrome BorderRect as fifth op"),
    }
    match &plan.ops[5] {
        GpuPaintOp::BorderRect { rect, thickness } => {
            assert_eq!(thickness, &1u32);
            assert_eq!(rect.x, regions.content.x);
        }
        _ => panic!("expected content BorderRect as sixth op"),
    }
    match &plan.ops[6] {
        GpuPaintOp::BorderRect { rect, thickness } => {
            assert_eq!(thickness, &1u32);
            assert_eq!(rect.x, regions.status.x);
        }
        _ => panic!("expected status BorderRect as seventh op"),
    }
}

/// Focused test: ensure executing a small, explicit GpuPaintPlan writes the
/// expected pixels into the buffer.
#[test]
fn execute_paint_plan_writes_pixels() {
    let width: u32 = 10;
    let height: u32 = 5;
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

    // Single small rect: at (1,1) size 2x2 with a distinctive color.
    let rect = GpuPaintRect {
        x: 1,
        y: 1,
        width: 2,
        height: 2,
        color: [11u8, 22u8, 33u8, 44u8],
    };
    let plan = GpuPaintPlan {
        ops: vec![GpuPaintOp::FillRect(rect.clone())],
    };

    // Execute the plan directly (executor should be dumb and follow ops).
    execute_paint_plan(&plan, &mut buf, width, height);

    // Helper to read RGBA at (x,y)
    let read_pixel = |x: u32, y: u32| -> [u8; 4] {
        let idx = ((y * width + x) * 4) as usize;
        [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
    };

    // Pixels inside rect should match color.
    assert_eq!(read_pixel(1, 1), rect.color);
    assert_eq!(read_pixel(2, 1), rect.color);
    assert_eq!(read_pixel(1, 2), rect.color);
    assert_eq!(read_pixel(2, 2), rect.color);

    // Pixel outside should remain zero.
    assert_eq!(read_pixel(0, 0), [0u8, 0u8, 0u8, 0u8]);
}

/// Executor size-mismatch remains a no-op (does not panic and does not mutate).
#[test]
fn execute_paint_plan_size_mismatch_is_noop() {
    let width: u32 = 8;
    let height: u32 = 4;
    // Wrong sized buffer intentionally.
    let mut buf = vec![7u8; (width as usize) * (height as usize) * 4 - 4];

    let rect = GpuPaintRect {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
        color: [9u8, 9u8, 9u8, 9u8],
    };
    let plan = GpuPaintPlan {
        ops: vec![GpuPaintOp::FillRect(rect)],
    };

    // Should silently return without modifying `buf`.
    execute_paint_plan(&plan, &mut buf, width, height);

    // Ensure buffer unchanged (all bytes still 7).
    assert!(buf.iter().all(|&b| b == 7u8));
}

/// Focused test: ensure the debug transcript reflects the final view + plan
/// in deterministic order and contains essential viewport information.
#[test]
fn shell_render_transcript_reflects_plan_order_and_viewport() {
    let width: u32 = 120;
    let height: u32 = 80;
    let chrome_h: u32 = 10;
    let status_h: u32 = 6;

    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let view = GpuShellView::from_shell_regions(&regions);
    let plan = GpuPaintPlan::from_view(&view);

    // Construct transcript via the new seam.
    let transcript = ShellRenderTranscript::from_view_and_plan(width, height, &view, &plan);
    let txt = transcript.to_string();

    // Sanity: viewport line present.
    assert!(txt.contains(&format!("viewport: {}x{}", width, height)));

    // The transcript.plan_lines must match the plan operation count and order.
    assert_eq!(transcript.plan_lines.len(), plan.ops.len());

    // First op in the plan should be the full-viewport background FillRect.
    assert!(transcript.plan_lines[0].starts_with("FillRect"));

    // Ensure the sequence preserves order: the first three non-background fills
    // are content, chrome, status in that sequence within the plan_lines slice.
    // Find the first occurrence of the content fill (it should exist).
    let mut found_content = false;
    let mut found_status_after = false;
    for (i, line) in transcript.plan_lines.iter().enumerate() {
        if line.contains("FillRect") && !line.contains("content:") {
            // no-op: placeholder to keep logic explicit and readable.
        }
        // Identify chrome fill by matching the chrome region coordinates.
        if line.contains(&format!("x={} y={} w={} h={}", regions.chrome.x, regions.chrome.y, regions.chrome.width, regions.chrome.height)) {
            found_content = true;
            // ensure subsequent lines still contain status later
            for later in transcript.plan_lines.iter().skip(i+1) {
                if later.contains(&format!("x={} y={} w={} h={}", regions.status.x, regions.status.y, regions.status.width, regions.status.height)) {
                    found_status_after = true;
                    break;
                }
            }
            break;
        }
    }
    // At minimum, ensure we detected chrome and found status after chrome.
    assert!(found_content);
    assert!(found_status_after);
}

/// New focused test: ensure the transcript includes richer semantic payloads
/// projected from ShellRegions -> GpuShellView -> ShellRenderTranscript.
#[test]
fn transcript_includes_semantic_payloads() {
    let width: u32 = 120;
    let height: u32 = 80;
    let chrome_h: u32 = 10;
    let status_h: u32 = 6;

    let mut regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    // Populate explicit semantic payloads (additive; doesn't affect painting).
    regions.chrome_label = Some("buf".to_string());
    regions.status_text = Some("status".to_string());
    regions.content_preview = Some("preview".to_string());
    regions.ai_indicator = Some("ai:available".to_string());
    regions.active_buffer_label = Some("active".to_string());

    let view = GpuShellView::from_shell_regions(&regions);
    let plan = GpuPaintPlan::from_view(&view);

    // Construct transcript via the new seam.
    let transcript = ShellRenderTranscript::from_view_and_plan(width, height, &view, &plan);
    let txt = transcript.to_string();

    // Ensure semantic payloads are present in the deterministic transcript output.
    assert!(txt.contains("chrome_label: buf"));
    assert!(txt.contains("status_text: status"));
    assert!(txt.contains("content_preview: preview"));
    assert!(txt.contains("ai_indicator: ai:available"));
}

#[test]
fn direct_activation_by_id_selects_tab() {
    // Ordered opened buffers (id, display)
    let opened = vec![
        ("id1".to_string(), "one".to_string()),
        ("id2".to_string(), "two".to_string()),
        ("id3".to_string(), "three".to_string()),
    ];

    let mut applied: Option<String> = None;
    let apply = |id: &str| {
        applied = Some(id.to_string());
    };

    // Activate id2 when no active currently
    let res = apply_tab_action(TabAction::ActivateById { id: "id2".to_string() }, &opened, None, apply);

    assert_eq!(res, Some("id2".to_string()));
    assert_eq!(applied, Some("id2".to_string()));
}

#[test]
fn direct_activation_out_of_range_is_noop() {
    let opened = vec![("id1".to_string(), "one".to_string())];
    let mut applied = false;
    let res = apply_tab_action(
        TabAction::ActivateById { id: "missing".to_string() },
        &opened,
        None,
        |_id| { applied = true; },
    );
    assert_eq!(res, None);
    assert!(!applied);
}

#[test]
fn activating_already_active_tab_is_noop() {
    let opened = vec![
        ("id1".to_string(), "one".to_string()),
        ("id2".to_string(), "two".to_string()),
    ];
    let mut applied = false;
    let res = apply_tab_action(
        TabAction::ActivateById { id: "id1".to_string() },
        &opened,
        Some("id1"),
        |_id| { applied = true; },
    );
    assert_eq!(res, None);
    assert!(!applied);
}

#[test]
fn focus_navigation_deterministic_and_apply() {
    let opened = vec![
        ("id1".to_string(), "one".to_string()),
        ("id2".to_string(), "two".to_string()),
        ("id3".to_string(), "three".to_string()),
    ];

    // No current focus -> FocusNext selects first
    let res = compute_focus_action_target(FocusAction::FocusNext { wrap: false }, &opened, None);
    assert_eq!(res, Some("id1".to_string()));

    // Apply focus and ensure setter invoked
    let mut applied: Option<String> = None;
    let apply = |id: &str| {
        applied = Some(id.to_string());
    };
    let res2 = apply_focus_action(FocusAction::FocusNext { wrap: false }, &opened, None, apply);
    assert_eq!(res2, Some("id1".to_string()));
    assert_eq!(applied, Some("id1".to_string()));

    // With focus on last, FocusNext with wrap -> cycles to first
    let res3 = compute_focus_action_target(FocusAction::FocusNext { wrap: true }, &opened, Some("id3"));
    assert_eq!(res3, Some("id1".to_string()));

    // FocusPrevious with wrap on first -> goes to last
    let res4 = compute_focus_action_target(FocusAction::FocusPrevious { wrap: true }, &opened, Some("id1"));
    assert_eq!(res4, Some("id3".to_string()));
}

/// New tests: ensure the keyboard/UI event path routes focus movement and
/// focused activation through the existing presenter helpers.
///
/// These tests exercise:
/// - handle_focus_key_event mapping of Tab -> focus movement
/// - Enter -> activate focused via activate_focused
/// - Ctrl+Tab -> existing activation cycling remains intact (handle_key_event)
#[test]
fn keyboard_focus_and_activation_event_path() {
    let opened = vec![
        ("id1".to_string(), "one".to_string()),
        ("id2".to_string(), "two".to_string()),
        ("id3".to_string(), "three".to_string()),
    ];

    // Prepare apply closures to capture invocations using interior mutability so
    // the closures can be reused and inspected without borrow conflicts.
    let applied_focus = Rc::new(RefCell::new(None::<String>));
    let applied_activate = Rc::new(RefCell::new(None::<String>));
    let mut apply_focus = {
        let af = applied_focus.clone();
        move |id: &str| {
            *af.borrow_mut() = Some(id.to_string());
        }
    };
    let mut apply_activate = {
        let aa = applied_activate.clone();
        move |id: &str| {
            *aa.borrow_mut() = Some(id.to_string());
        }
    };

    // Plain Tab (no ctrl) should move focus to first when no focus exists.
    let ev_tab = KeyEvent { ctrl: false, shift: false, key: "Tab".to_string() };
    let res = handle_focus_key_event(&ev_tab, &opened, None, None, &mut apply_focus, &mut apply_activate);
    assert_eq!(res, Some("id1".to_string()));
    assert_eq!(applied_focus.borrow().clone(), Some("id1".to_string()));
    assert!(applied_activate.borrow().is_none());

    // Enter should activate the currently-focused id (simulate focused=id2)
    *applied_activate.borrow_mut() = None;
    let ev_enter = KeyEvent { ctrl: false, shift: false, key: "Enter".to_string() };
    let res2 = handle_focus_key_event(&ev_enter, &opened, Some("id1"), Some("id2"), &mut apply_focus, &mut apply_activate);
    assert_eq!(res2, Some("id2".to_string()));
    assert_eq!(applied_activate.borrow().clone(), Some("id2".to_string()));
}

#[test]
fn ctrl_tab_still_activates_next() {
    let opened = vec![
        ("id1".to_string(), "one".to_string()),
        ("id2".to_string(), "two".to_string()),
        ("id3".to_string(), "three".to_string()),
    ];

    let mut applied: Option<String> = None;
    let ev = KeyEvent { ctrl: true, shift: false, key: "Tab".to_string() };
    let res = handle_key_event(&ev, &opened, Some("id1"), |id| { applied = Some(id.to_string()); });
    assert_eq!(res, Some("id2".to_string()));
    assert_eq!(applied, Some("id2".to_string()));
}

#[test]
fn focus_events_no_tabs_and_one_tab_safe() {
    // No tabs -> focus events are no-op
    let opened_empty: Vec<(String, String)> = Vec::new();
    let applied_focus = Rc::new(RefCell::new(None::<String>));
    let applied_activate = Rc::new(RefCell::new(None::<String>));
    let mut apply_focus = {
        let af = applied_focus.clone();
        move |id: &str| { *af.borrow_mut() = Some(id.to_string()); }
    };
    let mut apply_activate = {
        let aa = applied_activate.clone();
        move |id: &str| { *aa.borrow_mut() = Some(id.to_string()); }
    };

    let ev_tab = KeyEvent { ctrl: false, shift: false, key: "Tab".to_string() };
    let res = handle_focus_key_event(&ev_tab, &opened_empty, None, None, &mut apply_focus, &mut apply_activate);
    assert_eq!(res, None);
    assert!(applied_focus.borrow().is_none());
    assert!(applied_activate.borrow().is_none());

    // One tab -> focus/select returns that id
    let opened_one = vec![("only".to_string(), "one".to_string())];
    let applied_focus2 = Rc::new(RefCell::new(None::<String>));
    let mut apply_focus2 = {
        let af2 = applied_focus2.clone();
        move |id: &str| { *af2.borrow_mut() = Some(id.to_string()); }
    };
    let mut nop = |_id: &str| {};
    let res2 = handle_focus_key_event(&ev_tab, &opened_one, None, None, &mut apply_focus2, &mut nop);
    assert_eq!(res2, Some("only".to_string()));
}

#[test]
fn activate_focused_dispatches_activatebyid() {
    let opened = vec![
        ("id1".to_string(), "one".to_string()),
        ("id2".to_string(), "two".to_string()),
    ];

    // Current active is id1; focused is id2 -> should activate id2 via ActivateById path
    let mut applied: Option<String> = None;
    let apply = |id: &str| {
        applied = Some(id.to_string());
    };
    let res = activate_focused(&opened, Some("id1"), Some("id2"), apply);
    assert_eq!(res, Some("id2".to_string()));
    assert_eq!(applied, Some("id2".to_string()));
}

#[test]
fn no_tabs_and_one_tab_focus_activation_behaviour() {
    // No tabs -> focus actions are no-op
    let opened_empty: Vec<(String, String)> = Vec::new();
    let res = compute_focus_action_target(FocusAction::FocusNext { wrap: true }, &opened_empty, None);
    assert_eq!(res, None);

    // One tab -> focus/select returns that id
    let opened_one = vec![("only".to_string(), "one".to_string())];
    let res2 = compute_focus_action_target(FocusAction::FocusNext { wrap: false }, &opened_one, None);
    assert_eq!(res2, Some("only".to_string()));

    // Activating focused that equals active -> no-op
    let mut applied = false;
    let res3 = activate_focused(&opened_one, Some("only"), Some("only"), |_id| { applied = true; });
    assert_eq!(res3, None);
    assert!(!applied);

    // Focused tab no longer present -> activation is no-op
    let res4 = activate_focused(&opened_one, Some("only"), Some("missing"), |_id| { applied = true; });
    assert_eq!(res4, None);
}

#[test]
fn render_plan_includes_focused_and_active_colors() {
    let width: u32 = 300;
    let height: u32 = 100;
    let chrome_h: u32 = 30;
    let status_h: u32 = 10;

    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let mut view = GpuShellView::from_shell_regions(&regions);

    // Construct a TabStrip with one active and a different focused tab
    let tabs = TabStrip {
        tabs: vec![
            TabEntry { id: "a".to_string(), display: "A".to_string(), active: false, focused: true, index: 0 },
            TabEntry { id: "b".to_string(), display: "B".to_string(), active: true, focused: false, index: 1 },
            TabEntry { id: "c".to_string(), display: "C".to_string(), active: false, focused: false, index: 2 },
        ],
    };
    view.tabs = tabs.clone();

    let plan = GpuPaintPlan::from_view(&view);

    // Ensure there is at least one FillRect with focused color and one with active color.
    let mut found_focused = false;
    let mut found_active = false;
    for op in &plan.ops {
        if let GpuPaintOp::FillRect(r) = op {
            if r.color == [120u8, 160u8, 255u8, 255u8] {
                found_focused = true;
            }
            if r.color == [255u8, 200u8, 0u8, 255u8] {
                found_active = true;
            }
        }
    }
    assert!(found_focused, "expected focused tab fill color in plan");
    assert!(found_active, "expected active tab fill color in plan");
}

/// New test: ensure `GpuPaintPlan` emits Text ops for visible tab labels.
#[test]
fn paint_plan_includes_text_for_tabs() {
    let width: u32 = 200;
    let height: u32 = 80;
    let chrome_h: u32 = 20;
    let status_h: u32 = 6;

    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let mut view = GpuShellView::from_shell_regions(&regions);

    // Provide visible tabs
    view.tabs = TabStrip {
        tabs: vec![
            TabEntry { id: "a".to_string(), display: "one".to_string(), active: true, focused: false, index: 0 },
            TabEntry { id: "b".to_string(), display: "two".to_string(), active: false, focused: true, index: 1 },
        ],
    };

    let plan = GpuPaintPlan::from_view(&view);

    // There must be at least one Text op representing tab labels.
    let has_text = plan.ops.iter().any(|op| matches!(op, GpuPaintOp::Text { .. }));
    assert!(has_text, "expected at least one Text op in paint plan for tabs");
}

/// Ensure executor actually paints the label rect for Text ops (pixel-level).
#[test]
fn execute_paint_plan_renders_label_rect() {
    let width: u32 = 120;
    let height: u32 = 40;
    let chrome_h: u32 = 12;
    let status_h: u32 = 6;

    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let mut view = GpuShellView::from_shell_regions(&regions);

    view.tabs = TabStrip {
        tabs: vec![
            TabEntry { id: "x".to_string(), display: "only".to_string(), active: false, focused: true, index: 0 },
        ],
    };

    let plan = GpuPaintPlan::from_view(&view);
    let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

    // Execute plan
    execute_paint_plan(&plan, &mut buf, width, height);

    // Find the first Text op and sample its top-left pixel; it should match the color used.
    let mut found = false;
    for op in plan.ops.iter() {
        if let GpuPaintOp::Text { x, y, text, color, .. } = op {
            if text.is_empty() {
                continue;
            }
            let idx = (((*y) * width + (*x)) * 4) as usize;
            if idx + 4 <= buf.len() {
                let px = [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]];
                assert_eq!(px, *color, "expected label rect pixel to equal text color");
                found = true;
                break;
            }
        }
    }
    assert!(found, "expected to find and validate a Text op rendered into buffer");
}

/// Ensure when no tabs are present there are no Text ops emitted.
#[test]
fn no_tabs_emits_no_text_ops() {
    let width: u32 = 120;
    let height: u32 = 80;
    let chrome_h: u32 = 10;
    let status_h: u32 = 6;

    let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
    let view = GpuShellView::from_shell_regions(&regions);
    let plan = GpuPaintPlan::from_view(&view);

    assert!(!plan.ops.iter().any(|op| matches!(op, GpuPaintOp::Text { .. })));
}
