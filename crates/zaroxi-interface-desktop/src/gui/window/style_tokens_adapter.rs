//! Host-side adapter: resolves `StyleTokens` from the existing Zaroxi theme crate.
//!
//! This is where visual policy lives — brightness modifiers, role-to-color
//! mappings, and widget-specific pre-resolved fills. The engine crates consume
//! only the resolved `StyleTokens` struct and never reference `ZaroxiTheme` or
//! `SemanticColors`. A different app can replace this adapter entirely.

use zaroxi_core_engine_style::{StyleTokens, ThemeColor};
use zaroxi_interface_theme::theme::SemanticColors;

/// Convert a Zaroxi theme color to an engine ThemeColor.
fn to_engine(c: zaroxi_interface_theme::colors::Color) -> ThemeColor {
    ThemeColor::new(c.r, c.g, c.b, c.a)
}

/// Brightness/adjustment factors for deriving widget-specific colors from
/// base semantic tokens. These encode the app's visual policy.
pub struct AdapterModifiers {
    pub accent_dim: f32,
    pub brand_accent_dim: f32,
    pub titlebar_button_dim: f32,
    pub selected_brighten: f32,
    pub rail_inactive_fill: f32,
    pub rail_bottom_fill: f32,
    pub sidebar_file_fill: f32,
    pub sidebar_search_divider_dim: f32,
    pub sidebar_border_dim: f32,
    pub scrollbar_track_dim_editor: f32,
    pub scrollbar_thumb_dim_editor: f32,
    pub scrollbar_track_dim_sidebar: f32,
    pub scrollbar_thumb_dim_sidebar: f32,
    pub scrollbar_track_dim_bottom: f32,
    pub scrollbar_thumb_dim_bottom: f32,
    pub breadcrumb_dim: f32,
    pub status_pill_dim: f32,
    pub status_badge_brighten: f32,
    pub status_divider_dim: f32,
    pub panel_action_dim: f32,
}

impl Default for AdapterModifiers {
    fn default() -> Self {
        Self {
            accent_dim: 0.9,
            brand_accent_dim: 0.82,
            titlebar_button_dim: 0.15,
            selected_brighten: 1.6,
            rail_inactive_fill: 0.18,
            rail_bottom_fill: 0.16,
            sidebar_file_fill: 0.20,
            sidebar_search_divider_dim: 0.8,
            sidebar_border_dim: 0.85,
            scrollbar_track_dim_editor: 0.5,
            scrollbar_thumb_dim_editor: 0.25,
            scrollbar_track_dim_sidebar: 0.55,
            scrollbar_thumb_dim_sidebar: 0.22,
            scrollbar_track_dim_bottom: 0.6,
            scrollbar_thumb_dim_bottom: 0.3,
            breadcrumb_dim: 0.97,
            status_pill_dim: 0.14,
            status_badge_brighten: 2.2,
            status_divider_dim: 0.9,
            panel_action_dim: 0.18,
        }
    }
}

/// Resolve a complete `StyleTokens` from the Zaroxi `SemanticColors` and optional
/// brightness modifiers.
///
/// All widget-specific pre-resolved fills are computed here so the engine never
/// applies its own brightness adjustments. This is the *host's* visual policy.
pub fn resolve_style_tokens(sem: &SemanticColors, mods: &AdapterModifiers) -> StyleTokens {
    log::debug!("ZAROXI_STYLE_ADAPTER: resolving StyleTokens from SemanticColors");
    let accent = to_engine(sem.accent);
    let text_faint = to_engine(sem.text_faint);
    let text_secondary = to_engine(sem.text_secondary);
    let text_primary = to_engine(sem.text_primary);
    let text_muted = to_engine(sem.text_muted);
    let text_disabled = to_engine(sem.text_disabled);
    let text_on_accent = to_engine(sem.text_on_accent);
    let divider = to_engine(sem.divider);
    let divider_subtle = to_engine(sem.divider_subtle);
    let hover_bg = to_engine(sem.hover_background);
    let active_bg = to_engine(sem.active_background);
    let selected_bg = to_engine(sem.selected_background);
    let focus_ring = to_engine(sem.focus_ring);
    let accent_soft_bg = to_engine(sem.accent_soft_background);

    let editor_bg = to_engine(sem.editor_background);
    let app_bg = to_engine(sem.app_background);
    let titlebar_bg = to_engine(sem.title_bar_background);
    let rail_bg = to_engine(sem.activity_rail_background);
    let sidebar_bg = to_engine(sem.sidebar_background);
    let asst_bg = to_engine(sem.assistant_panel_background);
    let status_bg = to_engine(sem.status_bar_background);
    let panel_hdr_bg = to_engine(sem.panel_header_background);
    let bottom_panel_bg = to_engine(sem.bottom_panel_background);
    let bottom_panel_hdr_bg = to_engine(sem.bottom_panel_header_background);
    let asst_hdr_bg = to_engine(sem.assistant_panel_header_background);
    let input_bg = to_engine(sem.input_background);
    let tab_strip_bg = to_engine(sem.tab_strip_background);
    let tab_active_bg = to_engine(sem.tab_active_background);
    let tab_inactive_bg = to_engine(sem.tab_background);

    // Pre-resolved widget fills
    let toolbar_brand_accent = accent.adjust_brightness(mods.brand_accent_dim);
    let toolbar_close_button = accent.adjust_brightness(mods.accent_dim);
    let toolbar_button_default = text_faint.adjust_brightness(mods.titlebar_button_dim);
    let rail_item_active = selected_bg.adjust_brightness(mods.selected_brighten);
    let rail_item_active_accent = accent;
    let rail_item_inactive = text_faint.adjust_brightness(mods.rail_inactive_fill);
    let rail_item_bottom = text_faint.adjust_brightness(mods.rail_bottom_fill);
    let sidebar_file_item = text_faint.adjust_brightness(mods.sidebar_file_fill);
    let editor_breadcrumb_bg = editor_bg.adjust_brightness(mods.breadcrumb_dim);

    let sidebar_border = divider.adjust_brightness(mods.sidebar_border_dim);
    let sidebar_search_divider = divider_subtle.adjust_brightness(mods.sidebar_search_divider_dim);
    let status_divider = divider.adjust_brightness(mods.status_divider_dim);

    let sidebar_scrollbar_track =
        divider_subtle.adjust_brightness(mods.scrollbar_track_dim_sidebar);
    let sidebar_scrollbar_thumb = text_faint.adjust_brightness(mods.scrollbar_thumb_dim_sidebar);
    let editor_scrollbar_track = divider_subtle.adjust_brightness(mods.scrollbar_track_dim_editor);
    let editor_scrollbar_thumb = text_faint.adjust_brightness(mods.scrollbar_thumb_dim_editor);
    let bottom_scrollbar_track = divider_subtle.adjust_brightness(mods.scrollbar_track_dim_bottom);
    let bottom_scrollbar_thumb = text_faint.adjust_brightness(mods.scrollbar_thumb_dim_bottom);

    let panel_action_fill = text_faint.adjust_brightness(mods.panel_action_dim);
    let panel_action_hover =
        hover_bg.blend(text_faint.adjust_brightness(mods.panel_action_dim).to_array());

    let status_pill_fill = text_faint.adjust_brightness(mods.status_pill_dim);
    let status_language_badge_fill = accent_soft_bg.adjust_brightness(mods.status_badge_brighten);

    StyleTokens {
        app_background: app_bg,
        titlebar_background: titlebar_bg,
        rail_background: rail_bg,
        sidebar_background: sidebar_bg,
        sidebar_input: input_bg,
        editor_breadcrumb_background: editor_breadcrumb_bg,
        editor_content_background: editor_bg,
        assistant_panel_background: asst_bg,
        bottom_panel_background: bottom_panel_bg,
        bottom_panel_header_background: bottom_panel_hdr_bg,
        assistant_panel_header_background: asst_hdr_bg,
        status_bar_background: status_bg,
        panel_header_background: panel_hdr_bg,
        tab_strip_background: tab_strip_bg,
        tab_active_background: tab_active_bg,
        tab_inactive_background: tab_inactive_bg,
        text_primary,
        text_secondary,
        text_muted,
        text_faint,
        text_disabled,
        text_on_accent,
        divider_default: divider,
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
        status_success: to_engine(sem.success),
        status_warning: to_engine(sem.warning),
        status_error: to_engine(sem.error),
        status_info: to_engine(sem.info),
        editor_gutter_bg: to_engine(sem.editor_gutter_background),
        editor_line_highlight: to_engine(sem.editor_line_highlight),
        editor_cursor: to_engine(sem.editor_cursor),
        editor_selection: to_engine(sem.editor_selection),
        editor_find_highlight: to_engine(sem.editor_find_highlight),
        toolbar_brand_accent,
        toolbar_close_button,
        toolbar_button_default,
        rail_item_active,
        rail_item_active_accent,
        rail_item_inactive,
        rail_item_bottom,
        sidebar_file_item,
        sidebar_scrollbar_track,
        sidebar_scrollbar_thumb,
        editor_scrollbar_track,
        editor_scrollbar_thumb,
        panel_action_fill,
        panel_action_hover,
        panel_header_text: text_secondary,
        status_pill_fill,
        status_pill_text: text_secondary,
        status_language_badge_fill,
        status_language_badge_text: accent,
        bottom_scrollbar_track,
        bottom_scrollbar_thumb,
    }
}
