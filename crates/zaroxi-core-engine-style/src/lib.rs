// zaroxi-core-engine-style
// Engine-side style contracts: resolved color tokens, role enums, interaction states.
// The engine does NOT own theme policy. The host/app provides pre-resolved StyleTokens.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ThemeColor — canonical engine color (f32 RGBA)
// ---------------------------------------------------------------------------

/// RGBA color in linear float [0,1] components.
/// Uses an array representation for direct interop with wgpu/rendering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThemeColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ThemeColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn from_hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        Self { r, g, b, a: 1.0 }
    }

    pub fn with_alpha(&self, a: f32) -> Self {
        Self { r: self.r, g: self.g, b: self.b, a }
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn from_array(a: [f32; 4]) -> Self {
        Self { r: a[0], g: a[1], b: a[2], a: a[3] }
    }

    /// Adjust brightness by mixing with white/black.
    /// factor > 1.0 brightens, < 1.0 darkens.
    pub fn adjust_brightness(&self, factor: f32) -> Self {
        let r = (self.r * factor).clamp(0.0, 1.0);
        let g = (self.g * factor).clamp(0.0, 1.0);
        let b = (self.b * factor).clamp(0.0, 1.0);
        Self { r, g, b, a: self.a }
    }

    /// Blend with a translucent overlay.
    /// overlay is [r, g, b, a] where a controls blend strength.
    pub fn blend(&self, overlay: [f32; 4]) -> Self {
        let a = overlay[3];
        let r = self.r * (1.0 - a) + overlay[0] * a;
        let g = self.g * (1.0 - a) + overlay[1] * a;
        let b = self.b * (1.0 - a) + overlay[2] * a;
        Self { r, g, b, a: self.a }
    }
}

impl From<ThemeColor> for [f32; 4] {
    fn from(c: ThemeColor) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

impl From<[f32; 4]> for ThemeColor {
    fn from(a: [f32; 4]) -> Self {
        Self { r: a[0], g: a[1], b: a[2], a: a[3] }
    }
}

// ---------------------------------------------------------------------------
// StyleTokens — pre-resolved colors for all engine rendering slots
// ---------------------------------------------------------------------------
// The host/app creates this struct from its own theme system. The engine
// reads resolved colors without knowing how they were derived. No theme
// policy (dark/light variants, palette values, brightness modifiers) lives
// in engine crates.

/// Pre-resolved visual tokens provided by the host application.
///
/// Each field holds the final `ThemeColor` the engine should use for a
/// particular rendering slot. The host resolves all theme policy (variant,
/// palette, brightness modifiers) into this flat bag before handing it to
/// the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleTokens {
    // ── Panel backgrounds ──
    pub app_background: ThemeColor,
    pub titlebar_background: ThemeColor,
    pub rail_background: ThemeColor,
    pub sidebar_background: ThemeColor,
    pub sidebar_input: ThemeColor,
    pub editor_breadcrumb_background: ThemeColor,
    pub editor_content_background: ThemeColor,
    pub assistant_panel_background: ThemeColor,
    pub bottom_panel_background: ThemeColor,
    pub status_bar_background: ThemeColor,
    pub panel_header_background: ThemeColor,

    // ── Tab strip ──
    pub tab_strip_background: ThemeColor,
    pub tab_active_background: ThemeColor,
    pub tab_inactive_background: ThemeColor,

    // ── Text ──
    pub text_primary: ThemeColor,
    pub text_secondary: ThemeColor,
    pub text_muted: ThemeColor,
    pub text_faint: ThemeColor,
    pub text_disabled: ThemeColor,
    pub text_on_accent: ThemeColor,

    // ── Dividers ──
    pub divider_default: ThemeColor,
    pub divider_subtle: ThemeColor,
    pub sidebar_border: ThemeColor,
    pub sidebar_search_divider: ThemeColor,
    pub status_divider: ThemeColor,

    // ── Accent ──
    pub accent: ThemeColor,
    pub accent_soft_bg: ThemeColor,

    // ── Interaction state overlays ──
    pub hover_bg: ThemeColor,
    pub active_bg: ThemeColor,
    pub selected_bg: ThemeColor,
    pub focus_ring: ThemeColor,

    // ── Status colors ──
    pub status_success: ThemeColor,
    pub status_warning: ThemeColor,
    pub status_error: ThemeColor,
    pub status_info: ThemeColor,

    // ── Editor-specific ──
    pub editor_gutter_bg: ThemeColor,
    pub editor_line_highlight: ThemeColor,
    pub editor_cursor: ThemeColor,
    pub editor_selection: ThemeColor,
    pub editor_find_highlight: ThemeColor,

    // ── Widget-specific pre-resolved fills ──
    pub toolbar_brand_accent: ThemeColor,
    pub toolbar_close_button: ThemeColor,
    pub toolbar_button_default: ThemeColor,
    pub rail_item_active: ThemeColor,
    pub rail_item_active_accent: ThemeColor,
    pub rail_item_inactive: ThemeColor,
    pub rail_item_bottom: ThemeColor,
    pub sidebar_file_item: ThemeColor,
    pub sidebar_scrollbar_track: ThemeColor,
    pub sidebar_scrollbar_thumb: ThemeColor,
    pub editor_scrollbar_track: ThemeColor,
    pub editor_scrollbar_thumb: ThemeColor,
    pub panel_action_fill: ThemeColor,
    pub panel_action_hover: ThemeColor,
    pub panel_header_text: ThemeColor,
    pub status_pill_fill: ThemeColor,
    pub status_pill_text: ThemeColor,
    pub status_language_badge_fill: ThemeColor,
    pub status_language_badge_text: ThemeColor,
    pub bottom_scrollbar_track: ThemeColor,
    pub bottom_scrollbar_thumb: ThemeColor,
}

// ---------------------------------------------------------------------------
// EngineDesignTokens — spacing, radii, typography
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EngineDesignTokens {
    pub spacing_xxs: f32,
    pub spacing_xs: f32,
    pub spacing_sm: f32,
    pub spacing_md: f32,
    pub spacing_lg: f32,
    pub spacing_xl: f32,
    pub spacing_xxl: f32,

    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,

    pub border_width: f32,
    pub border_width_thick: f32,

    pub font_size_xs: f32,
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub font_size_xxl: f32,
}

impl Default for EngineDesignTokens {
    fn default() -> Self {
        Self {
            spacing_xxs: 2.0,
            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 12.0,
            spacing_lg: 16.0,
            spacing_xl: 24.0,
            spacing_xxl: 32.0,

            radius_sm: 4.0,
            radius_md: 6.0,
            radius_lg: 8.0,

            border_width: 1.0,
            border_width_thick: 2.0,

            font_size_xs: 10.0,
            font_size_sm: 12.0,
            font_size_md: 14.0,
            font_size_lg: 16.0,
            font_size_xl: 20.0,
            font_size_xxl: 24.0,
        }
    }
}

// ---------------------------------------------------------------------------
// PanelRole — generic UI region roles
// ---------------------------------------------------------------------------

/// Resolves fill colors for a generic UI surface role without region-name
/// string matching. Interface layers map IDE-specific concepts onto these
/// generic roles; the engine stays app-neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelRole {
    TopBar,
    NavigationRail,
    SidePanel,
    ContentTabStrip,
    ContentBreadcrumb,
    ContentArea,
    AuxiliaryPanelHeader,
    AuxiliaryPanelContent,
    BottomPanel,
    StatusBar,
    MinimapLane,
    BottomDock,
}

impl PanelRole {
    /// Resolve the primary fill color for this panel role from pre-resolved tokens.
    pub fn fill(&self, tokens: &StyleTokens) -> ThemeColor {
        match self {
            Self::TopBar => tokens.titlebar_background,
            Self::NavigationRail => tokens.rail_background,
            Self::SidePanel => tokens.sidebar_background,
            Self::ContentTabStrip => tokens.tab_strip_background,
            Self::ContentBreadcrumb => tokens.editor_breadcrumb_background,
            Self::ContentArea => tokens.editor_content_background,
            Self::AuxiliaryPanelHeader => tokens.panel_header_background,
            Self::AuxiliaryPanelContent => tokens.assistant_panel_background,
            Self::BottomPanel => tokens.bottom_panel_background,
            Self::StatusBar => tokens.status_bar_background,
            Self::MinimapLane => tokens.editor_content_background,
            Self::BottomDock => tokens.app_background,
        }
    }
}

// ---------------------------------------------------------------------------
// InteractionState — shared visual state model
// ---------------------------------------------------------------------------

/// Visual interaction states for engine UI primitives.
/// Primitives carry a state slot; renderers map state → visual adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InteractionState {
    #[default]
    Normal,
    Hover,
    Active,
    Focused,
    Selected,
    Disabled,
}

impl InteractionState {
    pub fn is_interactive(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    pub fn is_engaged(&self) -> bool {
        matches!(self, Self::Active | Self::Focused | Self::Selected)
    }

    /// Resolve the fill color for a widget background in this state,
    /// given the normal-state fill and overlay colors from StyleTokens.
    pub fn resolve_fill(&self, base_bg: &ThemeColor, tokens: &StyleTokens) -> ThemeColor {
        match self {
            Self::Normal => *base_bg,
            Self::Hover => base_bg.blend(tokens.hover_bg.to_array()),
            Self::Active => base_bg.blend(tokens.active_bg.to_array()),
            Self::Focused => base_bg.blend(tokens.focus_ring.to_array()),
            Self::Selected => tokens.selected_bg.blend(base_bg.to_array()),
            Self::Disabled => base_bg.adjust_brightness(0.6),
        }
    }

    /// Resolve the text color for a widget label in this state,
    /// given the normal-state text color and tokens for disabled/selected.
    pub fn resolve_text(&self, base_text: &ThemeColor, tokens: &StyleTokens) -> ThemeColor {
        match self {
            Self::Disabled => tokens.text_disabled,
            Self::Selected => tokens.text_primary,
            _ => *base_text,
        }
    }

    pub fn shows_accent(&self) -> bool {
        matches!(self, Self::Active | Self::Focused | Self::Selected)
    }
}

// ---------------------------------------------------------------------------
// WidgetId — lightweight widget identity for hit-testing and focus
// ---------------------------------------------------------------------------

/// Identifies a shell widget for hit-testing and state tracking purposes.
/// The engine uses these to resolve pointer-to-widget mappings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetId {
    Tab { index: usize },
    RailItem { index: usize },
    StatusSegment { index: usize },
    PanelHeader { id: &'static str },
    PanelAction { header_id: &'static str, action: &'static str },
    Scrollbar { index: usize },
    ToolbarButton { index: usize },
    Surface { role: SurfaceRole },
}

impl WidgetId {
    pub fn tab(idx: usize) -> Self {
        Self::Tab { index: idx }
    }
    pub fn rail_item(idx: usize) -> Self {
        Self::RailItem { index: idx }
    }
    pub fn status_segment(idx: usize) -> Self {
        Self::StatusSegment { index: idx }
    }
    pub fn panel_header(id: &'static str) -> Self {
        Self::PanelHeader { id }
    }
    pub fn panel_action(header_id: &'static str, action: &'static str) -> Self {
        Self::PanelAction { header_id, action }
    }
    pub fn scrollbar(idx: usize) -> Self {
        Self::Scrollbar { index: idx }
    }
    pub fn toolbar_button(idx: usize) -> Self {
        Self::ToolbarButton { index: idx }
    }
    pub fn surface(role: SurfaceRole) -> Self {
        Self::Surface { role }
    }
}

// ---------------------------------------------------------------------------
// SurfaceRole — structural role for a shell surface region
// ---------------------------------------------------------------------------

/// Declares what structural role a shell surface plays so the renderer can
/// apply role-appropriate theme colors without hardcoded identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceRole {
    AppBackground,
    Toolbar,
    ActivityRail,
    Sidebar,
    EditorContent,
    EditorTabStrip,
    EditorBreadcrumb,
    EditorBottomPanel,
    AIPanelHeader,
    AIPanelContent,
    StatusBar,
    BottomDock,
    MinimapLane,
}

// ---------------------------------------------------------------------------
// Test/development token factories
// ---------------------------------------------------------------------------
// These are convenience constructors for tests and quick iteration. They do
// NOT constitute engine-owned theme policy. Production code should source
// StyleTokens from the host application's theme system.

/// Create a dark-flavored `StyleTokens` for testing and development.
/// All widget-specific colors are pre-resolved so the engine can consume
/// them directly without brightness-modifier logic.
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
