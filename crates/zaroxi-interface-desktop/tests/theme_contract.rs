//! Phase 40 integration test: engine theme contract for non-IDE use case.
//!
//! Proves that the EngineTheme, ThemeModifiers, and PanelRole types work
//! for a generic document viewer (not an IDE) — verifying the theme contract
//! is truly app-neutral.

use zaroxi_core_engine_style::{EngineTheme, PanelRole, ThemeModifiers};

/// A generic document viewer can resolve panel fills using the theme contract
/// without any IDE-specific concepts.
#[test]
fn generic_viewer_uses_panel_role_resolution() {
    let theme = EngineTheme::dark();
    let mods = ThemeModifiers::default();

    // A document viewer: just a top bar, content area, and status bar.
    let top_fill = PanelRole::TopBar.fill(&theme, &mods);
    let content_fill = PanelRole::ContentArea.fill(&theme, &mods);
    let status_fill = PanelRole::StatusBar.fill(&theme, &mods);

    // Verify all fills are non-transparent (valid colors).
    assert!(top_fill.a > 0.0);
    assert!(content_fill.a > 0.0);
    assert!(status_fill.a > 0.0);

    // Content area should be darker than top bar for visual hierarchy.
    let top_luma = top_fill.r * 0.299 + top_fill.g * 0.587 + top_fill.b * 0.114;
    let content_luma = content_fill.r * 0.299 + content_fill.g * 0.587 + content_fill.b * 0.114;
    assert!(content_luma <= top_luma, "content area should not be brighter than top bar");
}

/// Theme tokens resolve to valid RGBA color arrays suitable for rendering.
#[test]
fn theme_tokens_are_valid_color_arrays() {
    let theme = EngineTheme::dark();

    let bg = theme.editor_background.to_array();
    assert_eq!(bg.len(), 4);
    for channel in &bg {
        assert!(*channel >= 0.0 && *channel <= 1.0);
    }
    assert_eq!(bg[3], 1.0, "editor_background should be opaque");

    let cursor = theme.editor_cursor.to_array();
    assert!(cursor[3] > 0.0, "cursor must be visible");

    let selection = theme.editor_selection.to_array();
    assert!(selection[3] > 0.0, "selection must be visible");
    assert!(selection[3] < 1.0, "selection should be translucent");
}

/// Both dark and light themes produce valid colors.
#[test]
fn dark_and_light_themes_are_valid() {
    let dark = EngineTheme::dark();
    let light = EngineTheme::light();

    // Dark theme: text should be bright, backgrounds dark.
    assert!(dark.text_primary.r > 0.5);
    assert!(dark.app_background.r < 0.3);

    // Light theme: text should be dark, backgrounds light.
    assert!(light.text_primary.r < 0.5);
    assert!(light.app_background.r > 0.5);
}

/// ThemeModifiers default values are in valid range.
#[test]
fn theme_modifiers_are_in_valid_range() {
    let mods = ThemeModifiers::default();

    // Brightness factors should be positive.
    assert!(mods.selected_brighten > 0.0);
    assert!(mods.rail_inactive_fill > 0.0);
    assert!(mods.scrollbar_track_fill > 0.0);
    assert!(mods.scrollbar_thumb_fill > 0.0);

    // Dim factors should be in [0, 1] for darkening.
    assert!(mods.accent_dim <= 1.0);
    assert!(mods.tab_accent_dim <= 1.0);

    // Alpha should be in [0, 1].
    assert!(mods.divider_subtle_alpha >= 0.0 && mods.divider_subtle_alpha <= 1.0);
}

/// PanelRole covers all generic UI regions without IDE concepts.
#[test]
fn panel_role_is_app_neutral() {
    // Each role is generic, no IDE-specific names.
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

    let theme = EngineTheme::dark();
    let mods = ThemeModifiers::default();
    for role in &roles {
        let fill = role.fill(&theme, &mods);
        assert!(fill.a > 0.0, "role {:?} fill must be visible", role);
    }
}
