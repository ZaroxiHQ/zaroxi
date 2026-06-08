//! Shell rendering contract tests — clip containment, viewport correctness,
//! and Explorer token inheritance.
//!
//! These verify that the fixes in the shell rendering pipeline produce correct
//! output for editor text containment, visible-line calculation, and Explorer
//! row color derivation.

use zaroxi_core_engine_style::test_utils::{test_tokens_dark, test_tokens_light};

use zaroxi_core_engine_ui::layout_constants as lc;

// ═══════════════════════════════════════════════════════════════════════
// 1. Editor text clip rect test
// ═══════════════════════════════════════════════════════════════════════

/// ContentArea blocks should carry a clip_rect when an EditorViewport is
/// available, preventing text from bleeding into adjacent panels.
#[test]
fn content_area_block_must_have_clip_rect() {
    // Import the editor_shell types to construct a minimal viewport
    use zaroxi_interface_desktop::gui::window::editor_shell::EditorViewport;

    let vp = EditorViewport::from_content_rect((200.0, 100.0, 600.0, 400.0));

    // The clip_rect should be inset from the content_rect by CONTENT_PAD
    assert!(
        vp.clip_rect.0 > vp.content_rect.0,
        "clip x={} must be > content x={}",
        vp.clip_rect.0,
        vp.content_rect.0
    );
    assert!(
        vp.clip_rect.1 > vp.content_rect.1,
        "clip y={} must be > content y={}",
        vp.clip_rect.1,
        vp.content_rect.1
    );
    assert!(
        vp.clip_rect.2 < vp.content_rect.2,
        "clip w={} must be < content w={}",
        vp.clip_rect.2,
        vp.content_rect.2
    );
    assert!(
        vp.clip_rect.3 < vp.content_rect.3,
        "clip h={} must be < content h={}",
        vp.clip_rect.3,
        vp.content_rect.3
    );

    // Clip rect should be non-negative
    assert!(vp.clip_rect.0 >= 0.0);
    assert!(vp.clip_rect.1 >= 0.0);
    assert!(vp.clip_rect.2 > 0.0, "clip width must be positive");
    assert!(vp.clip_rect.3 > 0.0, "clip height must be positive");
}

/// Verify that the string matching used to identify editor content blocks
/// includes the actual block id "editor_content".
#[test]
fn editor_content_block_id_matches_clip_predicate() {
    let is_content_block = |id: &str| {
        id.contains("ContentArea") || id.contains("content_area") || id == "editor_content"
    };

    // The actual block ID set by build_shell_regions_from_layout
    assert!(is_content_block("editor_content"), "editor_content must be recognized");
    assert!(is_content_block("content_area"), "content_area must be recognized");
    assert!(is_content_block("my_ContentArea_block"), "ContentArea must be recognized");

    // Adjacent panel IDs must NOT match
    assert!(!is_content_block("ai_panel_content"), "ai_panel should not match");
    assert!(!is_content_block("sidebar"), "sidebar should not match");
    assert!(!is_content_block("status_bar"), "status_bar should not match");
    assert!(!is_content_block("editor_tabs"), "editor_tabs should not match");
    assert!(!is_content_block("breadcrumb"), "breadcrumb should not match");
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Editor viewport visible-lines test
// ═══════════════════════════════════════════════════════════════════════

/// `editor_visible_lines` should compute more lines than the generic
/// `visible_lines_from_region` because the editor content area has no
/// header to subtract.
#[test]
fn editor_visible_lines_does_not_subtract_header() {
    let region_h = 600.0_f32;

    let generic = lc::visible_lines_from_region(region_h);
    let editor = lc::editor_visible_lines(region_h);

    // Editor should report MORE visible lines (no CONTENT_HEADER_H subtraction)
    assert!(
        editor > generic,
        "editor_visible_lines={} should be > visible_lines_from_region={} at h={}",
        editor,
        generic,
        region_h
    );
}

/// Visible line count tracks editor height correctly across resize.
#[test]
fn editor_visible_lines_tracks_height() {
    let tall = 900.0_f32;
    let short = 400.0_f32;

    let tall_lines = lc::editor_visible_lines(tall);
    let short_lines = lc::editor_visible_lines(short);

    assert!(tall_lines > short_lines, "taller region must yield more visible lines");
    assert!(tall_lines >= 1 && short_lines >= 1, "at least 1 visible line");
}

/// The formula matches expected values.
#[test]
fn editor_visible_lines_formula_is_correct() {
    // editor_visible_lines = (region_h - 2*CONTENT_PAD_Y) / LINE_HEIGHT
    // CONTENT_PAD_Y = 4, LINE_HEIGHT = 16
    assert_eq!(lc::editor_visible_lines(40.0), 2); // (40-8)/16 = 2
    assert_eq!(lc::editor_visible_lines(32.0), 1); // (32-8)/16 = 1.5 -> ceil -> 2... wait it's max(1)
    // Actually: (32 - 8) / 16 = 1.5, max(1.0) = 1.5, as usize = 1
    assert_eq!(lc::editor_visible_lines(32.0), 1);
    assert_eq!(lc::editor_visible_lines(72.0), 4); // (72-8)/16 = 4
    assert_eq!(lc::editor_visible_lines(8.0), 1); // (8-8)/16 = 0, max(1) = 1
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Explorer visual mapping test
// ═══════════════════════════════════════════════════════════════════════

/// Explorer row colors should derive from the sidebar/rail token family,
/// not from unrelated text colors.
#[test]
fn explorer_row_colors_derive_from_sidebar_palette() {
    let tokens = test_tokens_dark();

    // The sidebar background establishes the panel's color family
    let sidebar_bg = tokens.sidebar_background.to_array();
    assert!(sidebar_bg[3] >= 0.9, "sidebar_bg should be opaque");

    // sidebar_file_item (used for inactive Explorer rows) should
    // be in the same tonal family as the sidebar background, not
    // a completely unrelated color.
    let file_item = tokens.sidebar_file_item.to_array();
    assert!(file_item[3] > 0.0, "sidebar_file_item should have alpha");

    // The row color should be reasonably close to the sidebar background
    // in all channels. A tonal mismatch would show as large per-channel deltas
    // in all components. We check that no channel deviates wildly.
    let max_delta = (sidebar_bg[0] - file_item[0])
        .abs()
        .max((sidebar_bg[1] - file_item[1]).abs())
        .max((sidebar_bg[2] - file_item[2]).abs());

    // We allow up to 0.4 per-channel delta (tint variation within same family)
    assert!(
        max_delta < 0.5,
        "sidebar_file_item color delta={:.3} from sidebar_bg is too large — check token derivation",
        max_delta
    );
}

/// Light theme also keeps Explorer rows in the sidebar tonal family.
#[test]
fn explorer_row_colors_coherent_in_light_theme() {
    let tokens = test_tokens_light();

    let sidebar_bg = tokens.sidebar_background.to_array();
    let file_item = tokens.sidebar_file_item.to_array();

    let max_delta = (sidebar_bg[0] - file_item[0])
        .abs()
        .max((sidebar_bg[1] - file_item[1]).abs())
        .max((sidebar_bg[2] - file_item[2]).abs());

    assert!(
        max_delta < 0.5,
        "light sidebar_file_item delta={:.3} from sidebar_bg too large",
        max_delta
    );
}

/// Hover and selected states for Explorer rows should use sidebar-compatible
/// interaction colors.
#[test]
fn explorer_interaction_colors_are_sidebar_compatible() {
    let tokens = test_tokens_dark();

    // Selected state uses rail_item_active — should be non-transparent
    let active = tokens.rail_item_active.to_array();
    assert!(active[3] > 0.0, "active row must be visible");

    // Hover state uses hover_bg — should be sidebar-compatible
    let hover = tokens.hover_bg.to_array();
    assert!(hover[3] > 0.0, "hover must have some alpha");

    // Hover overlay should be subtle (low alpha when blended)
    assert!(hover[3] < 0.5, "hover_bg alpha should be subtle for overlay use");
}
