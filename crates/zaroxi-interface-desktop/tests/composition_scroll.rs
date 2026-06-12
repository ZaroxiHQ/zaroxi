//! Tests for DesktopComposition scroll processing: line-snapped pixel input,
//! single-source-of-truth model, clamping, and state reset.

use zaroxi_application_workspace::workspace_view::ActiveBufferDetails;
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_interface_desktop::desktop::composition::state::DesktopMetadata;

fn make_composition(total_lines: usize) -> DesktopComposition {
    let mut comp = DesktopComposition::new();
    let mut meta = DesktopMetadata::default();
    meta.editor_viewport_line_count = Some(10);
    meta.active_buffer_details = Some(ActiveBufferDetails {
        buffer_id: BufferId::from("buf:test"),
        display: None,
        line_count: total_lines,
    });
    comp.metadata = Some(meta);
    comp
}

#[test]
fn wheel_down_moves_three_lines_per_notch() {
    let mut comp = make_composition(50);

    comp.pending_vscroll_px = -48.0;
    comp.apply_pending_scrolls();

    let meta = comp.metadata.as_ref().unwrap();
    assert_eq!(meta.editor_scroll_top_line, 3, "one wheel-down notch = 3 lines");
    assert!((meta.editor_scroll_px - 48.0).abs() < 1.0, "px must be line-snapped: 3*16=48");
}

#[test]
fn wheel_up_from_middle_moves_up() {
    let mut comp = make_composition(50);
    comp.metadata.as_mut().unwrap().editor_scroll_top_line = 9;
    comp.metadata.as_mut().unwrap().editor_scroll_px = 144.0;

    comp.pending_vscroll_px = 48.0;
    comp.apply_pending_scrolls();

    let meta = comp.metadata.as_ref().unwrap();
    assert_eq!(meta.editor_scroll_top_line, 6, "wheel-up from line 9 should go to line 6");
    assert!((meta.editor_scroll_px - 96.0).abs() < 1.0);
}

#[test]
fn editor_scroll_px_is_always_line_snapped() {
    let mut comp = make_composition(50);

    for px in &[-48.0, 48.0, -96.0, 96.0, -17.0, 31.0] {
        comp.pending_vscroll_px += *px;
    }
    comp.apply_pending_scrolls();

    let meta = comp.metadata.as_ref().unwrap();
    let remainder = meta.editor_scroll_px % 16.0;
    assert!(
        remainder.abs() < 0.01,
        "editor_scroll_px={} must be multiple of 16 after line-snap",
        meta.editor_scroll_px
    );
}

#[test]
fn integer_scroll_fallback_when_pixel_empty() {
    let mut comp = make_composition(50);

    comp.pending_scroll_lines = 5;
    comp.pending_vscroll_px = 0.0;
    comp.apply_pending_scrolls();

    let meta = comp.metadata.as_ref().unwrap();
    assert_eq!(meta.editor_scroll_top_line, 5, "integer path should apply");
    assert!((meta.editor_scroll_px - 80.0).abs() < 1.0);
    assert_eq!(comp.pending_scroll_lines, 0);
}

#[test]
fn scroll_clamped_to_range() {
    let mut comp = make_composition(20);

    comp.pending_scroll_lines = 100;
    comp.apply_pending_scrolls();

    let meta = comp.metadata.as_ref().unwrap();
    let visible = meta.editor_viewport_line_count.unwrap_or(10);
    let max_allowed = (20usize).saturating_sub(visible);
    assert!(
        meta.editor_scroll_top_line <= max_allowed,
        "top_line={} should be clamped to max {}",
        meta.editor_scroll_top_line,
        max_allowed
    );
}

#[test]
fn pixel_scroll_clears_line_accumulator() {
    let mut comp = make_composition(50);

    comp.pending_vscroll_px = -16.0;
    comp.pending_scroll_lines = 10;
    comp.apply_pending_scrolls();

    assert_eq!(
        comp.pending_scroll_lines, 0,
        "line accumulator should be cleared even when pixel path runs"
    );
}

#[test]
fn horizontal_scroll_updates_offset() {
    let mut comp = DesktopComposition::new();
    comp.metadata = Some(DesktopMetadata::default());

    comp.pending_hscroll_px = 50.0;
    comp.apply_pending_scrolls();

    let meta = comp.metadata.as_ref().unwrap();
    assert!(
        meta.editor_horizontal_offset_px.unwrap_or(0.0) >= 50.0 - 0.01,
        "horizontal offset should be updated"
    );
}

#[test]
fn reset_scroll_state_clears_pending_and_resets_top_line() {
    let mut comp = make_composition(50);
    comp.pending_vscroll_px = 500.0;
    comp.pending_scroll_lines = 20;
    comp.metadata.as_mut().unwrap().editor_scroll_top_line = 15;
    comp.metadata.as_mut().unwrap().editor_scroll_px = 240.0;

    comp.reset_scroll_state();

    assert_eq!(comp.pending_vscroll_px, 0.0);
    assert_eq!(comp.pending_scroll_lines, 0);
    let meta = comp.metadata.as_ref().unwrap();
    assert_eq!(meta.editor_scroll_top_line, 0);
    assert!((meta.editor_scroll_px - 0.0).abs() < 0.01);
}

#[test]
fn small_file_no_scroll_range_stays_at_zero() {
    let mut comp = make_composition(5);

    comp.pending_vscroll_px = -96.0;
    comp.apply_pending_scrolls();

    let meta = comp.metadata.as_ref().unwrap();
    assert_eq!(meta.editor_scroll_top_line, 0, "5-line file with 10 visible should not scroll");
    assert!((meta.editor_scroll_px - 0.0).abs() < 0.01);
}

#[test]
fn exact_fit_file_no_scroll() {
    let mut comp = make_composition(10);

    comp.pending_vscroll_px = -160.0;
    comp.apply_pending_scrolls();

    let meta = comp.metadata.as_ref().unwrap();
    assert_eq!(meta.editor_scroll_top_line, 0, "exact-fit should not scroll");
}
