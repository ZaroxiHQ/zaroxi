//! Phase 46 integration test: engine style contract for non-IDE use case.
//!
//! Proves that StyleTokens and PanelRole work for a generic document viewer
//! (not an IDE) — verifying the engine style contract is truly app-neutral
//! and host-provided.

use zaroxi_core_engine_style::{PanelRole, StyleTokens, test_tokens_dark, test_tokens_light};

/// A generic document viewer can resolve panel fills using the style tokens
/// without any IDE-specific concepts.
#[test]
fn generic_viewer_uses_panel_role_resolution() {
    let tokens = test_tokens_dark();

    // A document viewer: just a top bar, content area, and status bar.
    let top_fill = PanelRole::TopBar.fill(&tokens);
    let content_fill = PanelRole::ContentArea.fill(&tokens);
    let status_fill = PanelRole::StatusBar.fill(&tokens);

    // Verify all fills are non-transparent (valid colors).
    assert!(top_fill.a > 0.0);
    assert!(content_fill.a > 0.0);
    assert!(status_fill.a > 0.0);
}

/// Style tokens resolve to valid RGBA color arrays suitable for rendering.
#[test]
fn tokens_produce_valid_color_arrays() {
    let tokens = test_tokens_dark();

    let bg = tokens.editor_content_background.to_array();
    assert_eq!(bg.len(), 4);
    for channel in &bg {
        assert!(*channel >= 0.0 && *channel <= 1.0);
    }
    assert_eq!(bg[3], 1.0, "editor_background should be opaque");

    let cursor = tokens.editor_cursor.to_array();
    assert!(cursor[3] > 0.0, "cursor must be visible");

    let selection = tokens.editor_selection.to_array();
    assert!(selection[3] > 0.0, "selection must be visible");
    assert!(selection[3] < 1.0, "selection should be translucent");
}

/// Both dark and light test tokens produce valid colors.
#[test]
fn dark_and_light_tokens_are_valid() {
    let dark = test_tokens_dark();
    let light = test_tokens_light();

    // Dark tokens: text should be bright, backgrounds dark.
    assert!(dark.text_primary.r > 0.5);
    assert!(dark.app_background.r < 0.3);

    // Light tokens: text should be dark, backgrounds light.
    assert!(light.text_primary.r < 0.5);
    assert!(light.app_background.r > 0.5);
}

/// StyleTokens carries all pre-resolved widget colors in valid range.
#[test]
fn style_tokens_widget_colors_are_valid() {
    let tokens = test_tokens_dark();

    // Widget-specific fills should be non-transparent.
    assert!(tokens.rail_item_active.a > 0.0);
    assert!(tokens.rail_item_inactive.a > 0.0);
    assert!(tokens.sidebar_file_item.a > 0.0);
    assert!(tokens.status_pill_fill.a > 0.0);
    assert!(tokens.status_language_badge_fill.a > 0.0);
    assert!(tokens.panel_action_fill.a > 0.0);
    assert!(tokens.toolbar_close_button.a > 0.0);
    assert!(tokens.toolbar_button_default.a > 0.0);
    assert!(tokens.editor_scrollbar_track.a > 0.0);
    assert!(tokens.editor_scrollbar_thumb.a > 0.0);
    assert!(tokens.bottom_scrollbar_track.a > 0.0);
    assert!(tokens.bottom_scrollbar_thumb.a > 0.0);
    assert!(tokens.sidebar_scrollbar_track.a > 0.0);
    assert!(tokens.sidebar_scrollbar_thumb.a > 0.0);
    assert!(tokens.sidebar_border.a > 0.0);
    assert!(tokens.status_divider.a > 0.0);
    assert!(tokens.sidebar_search_divider.a > 0.0);
    assert!(tokens.toolbar_brand_accent.a > 0.0);
    assert!(tokens.rail_item_bottom.a > 0.0);
    assert!(tokens.rail_item_active_accent.a > 0.0);
    assert!(tokens.panel_action_hover.a > 0.0);
}

/// PanelRole covers all generic UI regions without IDE concepts.
#[test]
fn panel_role_is_app_neutral() {
    let roles = [
        PanelRole::TopBar,
        PanelRole::NavigationRail,
        PanelRole::SidePanel,
        PanelRole::ContentTabStrip,
        PanelRole::ContentBreadcrumb,
        PanelRole::ContentArea,
        PanelRole::AuxiliaryPanelHeader,
        PanelRole::AuxiliaryPanelContent,
        PanelRole::BottomPanel,
        PanelRole::StatusBar,
        PanelRole::MinimapLane,
        PanelRole::BottomDock,
    ];

    let tokens = test_tokens_dark();
    for role in &roles {
        let fill = role.fill(&tokens);
        assert!(fill.a > 0.0, "role {:?} fill must be visible", role);
    }
}

/// Prove a host (non-Zaroxi app) can supply its own StyleTokens without
/// depending on any Zaroxi theme infrastructure.
#[test]
fn custom_style_tokens_drive_engine_without_zaroxi_theme() {
    use zaroxi_core_engine_style::ThemeColor;

    // Build style tokens from scratch — no ZaroxiTheme dependency.
    let custom = StyleTokens {
        app_background: ThemeColor::from_hex("#222222"),
        titlebar_background: ThemeColor::from_hex("#333333"),
        rail_background: ThemeColor::from_hex("#2a2a2a"),
        sidebar_background: ThemeColor::from_hex("#2d2d2d"),
        sidebar_input: ThemeColor::from_hex("#3a3a3a"),
        editor_breadcrumb_background: ThemeColor::from_hex("#2e2e2e"),
        editor_content_background: ThemeColor::from_hex("#282828"),
        assistant_panel_background: ThemeColor::from_hex("#2f2f2f"),
        bottom_panel_background: ThemeColor::from_hex("#2d2d2d"),
        status_bar_background: ThemeColor::from_hex("#333333"),
        panel_header_background: ThemeColor::from_hex("#383838"),
        tab_strip_background: ThemeColor::from_hex("#2a2a2a"),
        tab_active_background: ThemeColor::from_hex("#282828"),
        tab_inactive_background: ThemeColor::from_hex("#303030"),
        text_primary: ThemeColor::from_hex("#EEEEEE"),
        text_secondary: ThemeColor::from_hex("#CCCCCC"),
        text_muted: ThemeColor::from_hex("#AAAAAA"),
        text_faint: ThemeColor::from_hex("#777777"),
        text_disabled: ThemeColor::from_hex("#555555"),
        text_on_accent: ThemeColor::from_hex("#FFFFFF"),
        divider_default: ThemeColor::from_hex("#444444"),
        divider_subtle: ThemeColor::new(0.27, 0.27, 0.27, 0.3),
        sidebar_border: ThemeColor::from_hex("#3a3a3a"),
        sidebar_search_divider: ThemeColor::from_hex("#3a3a3a"),
        status_divider: ThemeColor::from_hex("#3d3d3d"),
        accent: ThemeColor::from_hex("#FF8800"),
        accent_soft_bg: ThemeColor::new(1.0, 0.53, 0.0, 0.08),
        hover_bg: ThemeColor::new(1.0, 1.0, 1.0, 0.06),
        active_bg: ThemeColor::new(1.0, 1.0, 1.0, 0.10),
        selected_bg: ThemeColor::new(1.0, 0.53, 0.0, 0.18),
        focus_ring: ThemeColor::new(1.0, 0.53, 0.0, 0.30),
        status_success: ThemeColor::from_hex("#00CC66"),
        status_warning: ThemeColor::from_hex("#FF8800"),
        status_error: ThemeColor::from_hex("#FF3333"),
        status_info: ThemeColor::from_hex("#3399FF"),
        editor_gutter_bg: ThemeColor::from_hex("#2a2a2a"),
        editor_line_highlight: ThemeColor::new(1.0, 1.0, 1.0, 0.03),
        editor_cursor: ThemeColor::from_hex("#EEEEEE"),
        editor_selection: ThemeColor::new(1.0, 0.53, 0.0, 0.22),
        editor_find_highlight: ThemeColor::new(1.0, 0.60, 0.0, 0.25),
        toolbar_brand_accent: ThemeColor::new(0.82, 0.44, 0.0, 1.0),
        toolbar_close_button: ThemeColor::new(0.90, 0.48, 0.0, 1.0),
        toolbar_button_default: ThemeColor::new(0.12, 0.12, 0.12, 1.0),
        rail_item_active: ThemeColor::new(1.0, 0.85, 0.0, 0.18),
        rail_item_active_accent: ThemeColor::from_hex("#FF8800"),
        rail_item_inactive: ThemeColor::new(0.14, 0.14, 0.14, 1.0),
        rail_item_bottom: ThemeColor::new(0.12, 0.12, 0.12, 1.0),
        sidebar_file_item: ThemeColor::new(0.15, 0.15, 0.15, 1.0),
        sidebar_scrollbar_track: ThemeColor::new(0.15, 0.15, 0.15, 0.3),
        sidebar_scrollbar_thumb: ThemeColor::new(0.17, 0.17, 0.17, 1.0),
        editor_scrollbar_track: ThemeColor::new(0.14, 0.14, 0.14, 0.3),
        editor_scrollbar_thumb: ThemeColor::new(0.19, 0.19, 0.19, 1.0),
        panel_action_fill: ThemeColor::new(0.14, 0.14, 0.14, 1.0),
        panel_action_hover: ThemeColor::new(0.14, 0.14, 0.14, 0.06),
        panel_header_text: ThemeColor::from_hex("#CCCCCC"),
        status_pill_fill: ThemeColor::new(0.11, 0.11, 0.11, 1.0),
        status_pill_text: ThemeColor::from_hex("#CCCCCC"),
        status_language_badge_fill: ThemeColor::new(1.0, 0.85, 0.0, 0.08),
        status_language_badge_text: ThemeColor::from_hex("#FF8800"),
        bottom_scrollbar_track: ThemeColor::new(0.16, 0.16, 0.16, 0.3),
        bottom_scrollbar_thumb: ThemeColor::new(0.23, 0.23, 0.23, 1.0),
    };

    // All roles resolve with this custom theme
    let roles =
        [PanelRole::TopBar, PanelRole::ContentArea, PanelRole::StatusBar, PanelRole::SidePanel];
    for role in &roles {
        let fill = role.fill(&custom);
        assert!(fill.a > 0.0, "custom theme role {:?} fill must be visible", role);
    }

    // The engine layout builder accepts this custom theme
    let layout = zaroxi_core_engine_layout::ShellLayout::from_window_size(800, 600);
    let _surface_set = zaroxi_core_engine_ui::build_shell_surface_set(&layout, &custom);
}
