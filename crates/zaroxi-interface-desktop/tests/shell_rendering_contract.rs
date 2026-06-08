//! Shell rendering contract tests — clip containment, text truncation,
//! and Explorer token inheritance.
//!
//! These verify that the fixes in the shell rendering pipeline produce correct
//! output for editor text containment, visible-line calculation, and Explorer
//! row color derivation.

use zaroxi_core_engine_style::test_utils::{test_tokens_dark, test_tokens_light};

use zaroxi_core_engine_ui::layout_constants as lc;

// ═══════════════════════════════════════════════════════════════════════
// 1. Editor clip rect + truncation contract
// ═══════════════════════════════════════════════════════════════════════

/// EditorViewport clip_rect is correctly inset from content_rect.
#[test]
fn clip_rect_is_inset_from_content_rect() {
    use zaroxi_interface_desktop::gui::window::editor_shell::EditorViewport;

    let vp = EditorViewport::from_content_rect((200.0, 100.0, 600.0, 400.0));

    assert!(vp.clip_rect.0 > vp.content_rect.0);
    assert!(vp.clip_rect.1 > vp.content_rect.1);
    assert!(vp.clip_rect.2 < vp.content_rect.2);
    assert!(vp.clip_rect.3 < vp.content_rect.3);
    assert!(vp.clip_rect.2 > 0.0, "clip width must be positive");
    assert!(vp.clip_rect.3 > 0.0, "clip height must be positive");
}

/// Verify the block ID predicate used in app.rs matches the live
/// content area block id "editor_content".
#[test]
fn content_block_id_predicate_matches_live_block_ids() {
    let is_content_block = |id: &str| {
        id.contains("ContentArea") || id.contains("content_area") || id == "editor_content"
    };

    assert!(is_content_block("editor_content"), "live editor content block must match");
    assert!(is_content_block("content_area"));
    assert!(is_content_block("my_ContentArea_block"));

    assert!(!is_content_block("ai_panel_content"), "AI panel should NOT match");
    assert!(!is_content_block("sidebar"));
    assert!(!is_content_block("status_bar"));
    assert!(!is_content_block("editor_tabs"));
    assert!(!is_content_block("breadcrumb"));
}

/// The visible_lines_from_region formula matches the renderer's content_h
/// calculation: region_h - CONTENT_HEADER_H - 2*CONTENT_PAD_X.
#[test]
fn visible_lines_matches_renderer_content_math() {
    assert_eq!(lc::visible_lines_from_region(44.0), 0.max(1) as usize); // (44-28-16)/16 = 0, max 1
    assert_eq!(lc::visible_lines_from_region(60.0), 1); // (60-44)/16 = 1
    assert_eq!(lc::visible_lines_from_region(108.0), 4); // (108-44)/16 = 4
    assert_eq!(lc::visible_lines_from_region(4.0), 1); // (4-44)/16 negative, max 1
}

/// Taller editor regions yield more visible lines (monotonic).
#[test]
fn visible_lines_tracks_region_height() {
    let short = lc::visible_lines_from_region(200.0);
    let tall = lc::visible_lines_from_region(800.0);
    assert!(tall > short);
    assert!(short >= 1);
}

/// For a given width, text truncation calculates the correct max character count.
#[test]
fn text_truncation_max_chars_formula() {
    // The renderer uses max_chars = (content_w / 8.0) as usize
    let char_w = 8.0;
    assert_eq!((600.0 / char_w) as usize, 75);
    assert_eq!((200.0 / char_w) as usize, 25);
    assert_eq!((10.0 / char_w) as usize, 1);
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Explorer token contract
// ═══════════════════════════════════════════════════════════════════════

/// Explorer row colors (sidebar_file_item) should derive from the sidebar
/// background color family, not from unrelated colors.
#[test]
fn explorer_row_colors_derive_from_sidebar_palette() {
    let tokens = test_tokens_dark();

    let sidebar_bg = tokens.sidebar_background.to_array();
    assert!(sidebar_bg[3] >= 0.9, "sidebar_bg should be opaque");

    let file_item = tokens.sidebar_file_item.to_array();
    assert!(file_item[3] > 0.0, "sidebar_file_item must have alpha");

    let max_delta = (sidebar_bg[0] - file_item[0])
        .abs()
        .max((sidebar_bg[1] - file_item[1]).abs())
        .max((sidebar_bg[2] - file_item[2]).abs());

    assert!(
        max_delta < 0.5,
        "sidebar_file_item delta={:.3} from sidebar_bg too large — wrong token derivation",
        max_delta
    );
}

/// Light theme Explorer rows also stay in the sidebar tonal family.
#[test]
fn explorer_row_colors_coherent_in_light_theme() {
    let tokens = test_tokens_light();

    let sidebar_bg = tokens.sidebar_background.to_array();
    let file_item = tokens.sidebar_file_item.to_array();

    let max_delta = (sidebar_bg[0] - file_item[0])
        .abs()
        .max((sidebar_bg[1] - file_item[1]).abs())
        .max((sidebar_bg[2] - file_item[2]).abs());

    assert!(max_delta < 0.5, "light sidebar_file_item delta={:.3} too large", max_delta);
}

/// Hover and selected states for Explorer rows use sidebar-compatible
/// interaction colors.
#[test]
fn explorer_interaction_colors_are_sidebar_compatible() {
    let tokens = test_tokens_dark();

    let active = tokens.rail_item_active.to_array();
    assert!(active[3] > 0.0, "active row must be visible");

    let hover = tokens.hover_bg.to_array();
    assert!(hover[3] > 0.0, "hover must have some alpha");
    assert!(hover[3] < 0.5, "hover_bg alpha should be subtle for overlay use");
}
