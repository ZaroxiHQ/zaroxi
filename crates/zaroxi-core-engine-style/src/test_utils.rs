//! Convenience constructors for tests and quick development iteration.
//!
//! These do NOT constitute engine-owned theme policy. Production code should
//! source `StyleTokens` from the host application's theme system via a
//! host-side adapter.
//!
//! This module is `#[doc(hidden)]` — the functions are available but not
//! intended for public API documentation.
#![doc(hidden)]

use crate::{StyleTokens, ThemeColor};

/// Create a dark-flavored `StyleTokens` for testing and development.
pub fn test_tokens_dark() -> StyleTokens {
    let accent = ThemeColor::from_hex("#5B8CFF");
    let text_faint = ThemeColor::from_hex("#7E8794");
    let text_secondary = ThemeColor::from_hex("#C8CDD6");
    let text_primary = ThemeColor::from_hex("#E6EAF2");
    let text_muted = ThemeColor::from_hex("#AAB2BF");
    let text_disabled = ThemeColor::from_hex("#5A6270");
    let text_on_accent = ThemeColor::from_hex("#FFFFFF");
    let divider_default = ThemeColor::from_hex("#343944");
    let divider_subtle = ThemeColor::new(0.20, 0.22, 0.27, 0.3);
    let hover_bg = ThemeColor::new(1.0, 1.0, 1.0, 0.06);
    let active_bg = ThemeColor::new(1.0, 1.0, 1.0, 0.10);
    let selected_bg = ThemeColor::new(0.36, 0.55, 1.0, 0.18);
    let focus_ring = ThemeColor::new(0.36, 0.55, 1.0, 0.30);
    let accent_soft_bg = ThemeColor::new(0.36, 0.55, 1.0, 0.08);

    let app_bg = ThemeColor::from_hex("#0D0E11");
    let titlebar_bg = ThemeColor::from_hex("#1A1B21");
    let rail_bg = ThemeColor::from_hex("#16171C");
    let sidebar_bg = ThemeColor::from_hex("#1A1B21");
    let editor_bg = ThemeColor::from_hex("#15161A");
    let asst_bg = ThemeColor::from_hex("#1C1D23");
    let bottom_bg = ThemeColor::from_hex("#1A1B21");
    let status_bg = ThemeColor::from_hex("#1A1B21");
    let panel_hdr_bg = ThemeColor::from_hex("#1E1F25");
    let tab_strip_bg = ThemeColor::from_hex("#121318");
    let tab_active_bg = ThemeColor::from_hex("#15161A");
    let tab_inactive_bg = ThemeColor::from_hex("#18191F");
    let sidebar_input = ThemeColor::from_hex("#1E1F25");
    let editor_gutter_bg = ThemeColor::from_hex("#1E1F24");

    let editor_breadcrumb_bg = editor_bg.adjust_brightness(0.97);
    let sidebar_border = divider_default.adjust_brightness(0.85);
    let sidebar_search_divider = divider_subtle.adjust_brightness(0.8);
    let status_divider = divider_default.adjust_brightness(0.9);

    StyleTokens {
        app_background: app_bg,
        titlebar_background: titlebar_bg,
        rail_background: rail_bg,
        sidebar_background: sidebar_bg,
        sidebar_input,
        editor_breadcrumb_background: editor_breadcrumb_bg,
        editor_content_background: editor_bg,
        assistant_panel_background: asst_bg,
        bottom_panel_background: bottom_bg,
        bottom_panel_header_background: bottom_bg,
        assistant_panel_header_background: asst_bg,
        status_bar_background: status_bg,
        panel_header_background: panel_hdr_bg,
        panel_background: sidebar_bg,
        tab_strip_background: tab_strip_bg,
        tab_active_background: tab_active_bg,
        tab_inactive_background: tab_inactive_bg,
        text_primary,
        text_secondary,
        text_muted,
        text_faint,
        text_disabled,
        text_on_accent,
        divider_default,
        divider_subtle,
        sidebar_border,
        sidebar_search_divider,
        status_divider,
        accent,
        accent_soft_bg,
        hover_bg,
        active_bg,
        selected_bg,
        focus_ring,
        status_success: ThemeColor::from_hex("#4CAF50"),
        status_warning: ThemeColor::from_hex("#FF9800"),
        status_error: ThemeColor::from_hex("#F44336"),
        status_info: accent,
        editor_gutter_bg,
        editor_line_highlight: ThemeColor::new(1.0, 1.0, 1.0, 0.03),
        editor_cursor: text_primary,
        editor_selection: ThemeColor::new(0.36, 0.55, 1.0, 0.22),
        editor_find_highlight: ThemeColor::new(1.0, 0.60, 0.0, 0.25),
        toolbar_brand_accent: accent.adjust_brightness(0.82),
        toolbar_close_button: accent.adjust_brightness(0.9),
        toolbar_button_default: text_faint.adjust_brightness(0.15),
        rail_item_active: selected_bg.adjust_brightness(1.6),
        rail_item_active_accent: accent,
        rail_item_inactive: text_faint.adjust_brightness(0.18),
        rail_item_bottom: text_faint.adjust_brightness(0.16),
        sidebar_file_item: text_faint.adjust_brightness(0.20),
        sidebar_scrollbar_track: divider_subtle.adjust_brightness(0.55),
        sidebar_scrollbar_thumb: text_faint.adjust_brightness(0.22),
        editor_scrollbar_track: divider_subtle.adjust_brightness(0.5),
        editor_scrollbar_thumb: text_faint.adjust_brightness(0.25),
        panel_action_fill: text_faint.adjust_brightness(0.18),
        panel_action_hover: hover_bg.blend(text_faint.adjust_brightness(0.18).to_array()),
        panel_header_text: text_secondary,
        status_pill_fill: text_faint.adjust_brightness(0.14),
        status_pill_text: text_secondary,
        status_language_badge_fill: accent_soft_bg.adjust_brightness(2.2),
        status_language_badge_text: accent,
        bottom_scrollbar_track: divider_subtle.adjust_brightness(0.6),
        bottom_scrollbar_thumb: text_faint.adjust_brightness(0.3),
    }
}

/// Create a light-flavored `StyleTokens` for testing and development.
pub fn test_tokens_light() -> StyleTokens {
    let accent = ThemeColor::from_hex("#426EDB");
    let text_faint = ThemeColor::from_hex("#8A919D");
    let text_secondary = ThemeColor::from_hex("#3D434A");
    let text_primary = ThemeColor::from_hex("#22262B");
    let text_muted = ThemeColor::from_hex("#616975");
    let text_disabled = ThemeColor::from_hex("#B0B6C0");
    let text_on_accent = ThemeColor::from_hex("#FFFFFF");
    let divider_default = ThemeColor::from_hex("#D7D1C7");
    let divider_subtle = ThemeColor::new(0.84, 0.82, 0.78, 0.4);
    let hover_bg = ThemeColor::new(0.0, 0.0, 0.0, 0.04);
    let active_bg = ThemeColor::new(0.0, 0.0, 0.0, 0.08);
    let selected_bg = ThemeColor::new(0.26, 0.43, 0.86, 0.08);
    let focus_ring = ThemeColor::new(0.26, 0.43, 0.86, 0.25);
    let accent_soft_bg = ThemeColor::new(0.26, 0.43, 0.86, 0.05);

    let app_bg = ThemeColor::from_hex("#F4F3EF");
    let titlebar_bg = ThemeColor::from_hex("#F8F6F2");
    let rail_bg = ThemeColor::from_hex("#E7E4DD");
    let sidebar_bg = ThemeColor::from_hex("#F0EEE8");
    let editor_bg = ThemeColor::from_hex("#FBFAF7");
    let asst_bg = ThemeColor::from_hex("#F2F0EA");
    let bottom_bg = ThemeColor::from_hex("#ECE9E3");
    let status_bg = ThemeColor::from_hex("#ECE9E3");
    let panel_hdr_bg = ThemeColor::from_hex("#E8E5DE");
    let tab_strip_bg = ThemeColor::from_hex("#E7E4DD");
    let tab_active_bg = ThemeColor::from_hex("#FBFAF7");
    let tab_inactive_bg = ThemeColor::from_hex("#F1EEE8");
    let sidebar_input = ThemeColor::from_hex("#FFFFFF");
    let editor_gutter_bg = ThemeColor::from_hex("#FBFAF7");

    let editor_breadcrumb_bg = editor_bg.adjust_brightness(0.97);
    let sidebar_border = divider_default.adjust_brightness(0.85);
    let sidebar_search_divider = divider_subtle.adjust_brightness(0.8);
    let status_divider = divider_default.adjust_brightness(0.9);

    StyleTokens {
        app_background: app_bg,
        titlebar_background: titlebar_bg,
        rail_background: rail_bg,
        sidebar_background: sidebar_bg,
        sidebar_input,
        editor_breadcrumb_background: editor_breadcrumb_bg,
        editor_content_background: editor_bg,
        assistant_panel_background: asst_bg,
        bottom_panel_background: bottom_bg,
        bottom_panel_header_background: bottom_bg,
        assistant_panel_header_background: asst_bg,
        status_bar_background: status_bg,
        panel_header_background: panel_hdr_bg,
        panel_background: sidebar_bg,
        tab_strip_background: tab_strip_bg,
        tab_active_background: tab_active_bg,
        tab_inactive_background: tab_inactive_bg,
        text_primary,
        text_secondary,
        text_muted,
        text_faint,
        text_disabled,
        text_on_accent,
        divider_default,
        divider_subtle,
        sidebar_border,
        sidebar_search_divider,
        status_divider,
        accent,
        accent_soft_bg,
        hover_bg,
        active_bg,
        selected_bg,
        focus_ring,
        status_success: ThemeColor::from_hex("#2E7D32"),
        status_warning: ThemeColor::from_hex("#E65100"),
        status_error: ThemeColor::from_hex("#C62828"),
        status_info: accent,
        editor_gutter_bg,
        editor_line_highlight: ThemeColor::new(0.26, 0.43, 0.86, 0.03),
        editor_cursor: text_primary,
        editor_selection: ThemeColor::new(0.26, 0.43, 0.86, 0.14),
        editor_find_highlight: ThemeColor::new(0.90, 0.40, 0.0, 0.18),
        toolbar_brand_accent: accent.adjust_brightness(0.82),
        toolbar_close_button: accent.adjust_brightness(0.9),
        toolbar_button_default: text_faint.adjust_brightness(0.15),
        rail_item_active: selected_bg.adjust_brightness(1.6),
        rail_item_active_accent: accent,
        rail_item_inactive: text_faint.adjust_brightness(0.18),
        rail_item_bottom: text_faint.adjust_brightness(0.16),
        sidebar_file_item: text_faint.adjust_brightness(0.20),
        sidebar_scrollbar_track: divider_subtle.adjust_brightness(0.55),
        sidebar_scrollbar_thumb: text_faint.adjust_brightness(0.22),
        editor_scrollbar_track: divider_subtle.adjust_brightness(0.5),
        editor_scrollbar_thumb: text_faint.adjust_brightness(0.25),
        panel_action_fill: text_faint.adjust_brightness(0.18),
        panel_action_hover: hover_bg.blend(text_faint.adjust_brightness(0.18).to_array()),
        panel_header_text: text_secondary,
        status_pill_fill: text_faint.adjust_brightness(0.14),
        status_pill_text: text_secondary,
        status_language_badge_fill: accent_soft_bg.adjust_brightness(2.2),
        status_language_badge_text: accent,
        bottom_scrollbar_track: divider_subtle.adjust_brightness(0.6),
        bottom_scrollbar_thumb: text_faint.adjust_brightness(0.3),
    }
}
