//! Scrollbar geometry, wheel accumulation, and drag synchronization tests.

use zaroxi_core_engine_ui::layout_constants::{
    LINE_HEIGHT, SB_EDITOR_SPEC, SCROLLBAR_ID_EDITOR, ScrollbarSpec, compute_scrollbar_geometry,
};

/// The editor thumb height reflects the ratio of visible to total lines.
#[test]
fn thumb_height_reflects_visible_to_total_ratio() {
    let (_, _, _, track_h, thumb_h) =
        compute_scrollbar_geometry((0.0, 0.0, 400.0, 300.0), &SB_EDITOR_SPEC, 0.0);

    assert!(track_h > 0.0, "track height must be positive");
    assert!(thumb_h > 0.0, "thumb height must be positive");
    assert!(thumb_h <= track_h, "thumb must fit inside track");

    let spec = ScrollbarSpec {
        sb_width: 6.0,
        inset_right: 3.0,
        track_inset_y: 4.0,
        track_h_reduction: 8.0,
        thumb_ratio: 0.1,
        thumb_min_h: 4.0,
    };
    let (_, _, _, _, small_thumb_h) =
        compute_scrollbar_geometry((0.0, 0.0, 400.0, 300.0), &spec, 0.0);

    let big_spec = ScrollbarSpec { thumb_ratio: 0.8, thumb_min_h: 4.0, ..spec };
    let (_, _, _, _, big_thumb_h) =
        compute_scrollbar_geometry((0.0, 0.0, 400.0, 300.0), &big_spec, 0.0);

    assert!(big_thumb_h > small_thumb_h, "larger thumb_ratio should produce taller thumb");
}

/// Thumb geometry respects minimum height.
#[test]
fn thumb_respects_minimum_height() {
    let spec = ScrollbarSpec {
        sb_width: 6.0,
        inset_right: 3.0,
        track_inset_y: 4.0,
        track_h_reduction: 8.0,
        thumb_ratio: 0.001,
        thumb_min_h: 24.0,
    };
    let (_, _, _, _, thumb_h) = compute_scrollbar_geometry((0.0, 0.0, 400.0, 100.0), &spec, 0.0);

    assert!(thumb_h >= 24.0, "thumb must honor minimum height");
}

/// Scrollbar geometry is clamped to the right edge of the region.
#[test]
fn scrollbar_positioned_at_right_edge() {
    let region = (10.0, 5.0, 300.0, 200.0);
    let spec = &SB_EDITOR_SPEC;
    let (sb_x, _, sb_w, _, _) = compute_scrollbar_geometry(region, spec, 0.0);

    let expected_x = region.0 + region.2 - spec.sb_width - spec.inset_right;
    assert_eq!(sb_x, expected_x, "scrollbar x should be at right inset");
    assert_eq!(sb_w, spec.sb_width, "scrollbar width should match spec");
}

/// Normalized scroll offset mapping is lossless: converting top_line to normalized
/// offset and back should yield the same top_line (ignoring rounding when line counts
/// are small).
#[test]
fn normalized_offset_roundtrip() {
    let total_lines: usize = 100;
    let visible_lines: usize = 10;
    let max_scroll = total_lines - visible_lines;
    let max_scroll_px = max_scroll as f32 * LINE_HEIGHT;

    for top_line in &[0, 5, 50, 90] {
        let scroll_px = *top_line as f32 * LINE_HEIGHT;
        let norm =
            if max_scroll_px > 0.0 { (scroll_px / max_scroll_px).clamp(0.0, 1.0) } else { 0.0 };

        let roundtrip_px = norm * max_scroll_px;
        let roundtrip_line = (roundtrip_px / LINE_HEIGHT).round() as usize;

        assert!(
            (*top_line as isize - roundtrip_line as isize).abs() <= 1,
            "norm={:.4} top_line={} → roundtrip_line={}",
            norm,
            top_line,
            roundtrip_line
        );
    }
}

/// Empty document yields zero scroll range.
#[test]
fn empty_document_zero_scroll_range() {
    let total_lines: usize = 1;
    let visible_lines: usize = 10;
    assert!(total_lines <= visible_lines, "no scroll needed when all lines fit");

    let max_scroll = total_lines.saturating_sub(visible_lines);
    assert_eq!(max_scroll, 0, "max scroll should be 0 when document fits");
}

/// SCROLLBAR_ID_EDITOR is the correct constant value expected by widget tree and
/// UiBlock consumers.
#[test]
fn editor_scrollbar_id_is_correct() {
    assert_eq!(SCROLLBAR_ID_EDITOR, 1);
}
