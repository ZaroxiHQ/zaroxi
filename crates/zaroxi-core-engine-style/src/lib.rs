// zaroxi-core-engine-style
// Engine-side theme source of truth for Zaroxi Studio.
// Provides semantic color roles, design tokens, and interaction states.

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

    /// From sRGB hex string like "#1B1D22"
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
// EngineTheme — semantic color roles for the engine UI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ThemeVariant {
    Dark,
    Light,
}

/// Engine-owned semantic colors.
/// Organized by role so no per-panel hardcoded values are needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineTheme {
    pub variant: ThemeVariant,

    // ── Background surfaces (depth hierarchy) ──
    pub app_background: ThemeColor,
    pub shell_background: ThemeColor,
    pub surface_default: ThemeColor,
    pub surface_elevated: ThemeColor,
    pub editor_background: ThemeColor,
    pub input_background: ThemeColor,
    pub status_bar_background: ThemeColor,
    pub activity_rail_background: ThemeColor,
    pub sidebar_background: ThemeColor,
    pub tab_strip_background: ThemeColor,
    pub tab_active_background: ThemeColor,
    pub tab_inactive_background: ThemeColor,
    pub assistant_panel_background: ThemeColor,
    pub bottom_panel_background: ThemeColor,

    // ── Text roles ──
    pub text_primary: ThemeColor,
    pub text_secondary: ThemeColor,
    pub text_muted: ThemeColor,
    pub text_faint: ThemeColor,
    pub text_on_accent: ThemeColor,
    pub text_disabled: ThemeColor,
    pub text_link: ThemeColor,

    // ── Borders & dividers ──
    pub border_default: ThemeColor,
    pub border_subtle: ThemeColor,
    pub divider_default: ThemeColor,
    pub divider_subtle: ThemeColor,

    // ── Accent ──
    pub accent: ThemeColor,
    pub accent_hover: ThemeColor,
    pub accent_soft_bg: ThemeColor,

    // ── Interaction states ──
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
}

impl EngineTheme {
    pub fn dark() -> Self {
        Self {
            variant: ThemeVariant::Dark,
            app_background: ThemeColor::from_hex("#0D0E11"),
            shell_background: ThemeColor::from_hex("#121318"),
            surface_default: ThemeColor::from_hex("#1A1B21"),
            surface_elevated: ThemeColor::from_hex("#1E1F25"),
            editor_background: ThemeColor::from_hex("#15161A"),
            input_background: ThemeColor::from_hex("#1E1F25"),
            status_bar_background: ThemeColor::from_hex("#1A1B21"),
            activity_rail_background: ThemeColor::from_hex("#16171C"),
            sidebar_background: ThemeColor::from_hex("#1A1B21"),
            tab_strip_background: ThemeColor::from_hex("#121318"),
            tab_active_background: ThemeColor::from_hex("#15161A"),
            tab_inactive_background: ThemeColor::from_hex("#18191F"),
            assistant_panel_background: ThemeColor::from_hex("#1C1D23"),
            bottom_panel_background: ThemeColor::from_hex("#1A1B21"),

            text_primary: ThemeColor::from_hex("#E6EAF2"),
            text_secondary: ThemeColor::from_hex("#C8CDD6"),
            text_muted: ThemeColor::from_hex("#AAB2BF"),
            text_faint: ThemeColor::from_hex("#7E8794"),
            text_on_accent: ThemeColor::from_hex("#FFFFFF"),
            text_disabled: ThemeColor::from_hex("#5A6270"),
            text_link: ThemeColor::from_hex("#5B8CFF"),

            border_default: ThemeColor::from_hex("#343944"),
            border_subtle: ThemeColor::new(0.20, 0.22, 0.27, 0.5),
            divider_default: ThemeColor::from_hex("#343944"),
            divider_subtle: ThemeColor::new(0.20, 0.22, 0.27, 0.3),

            accent: ThemeColor::from_hex("#5B8CFF"),
            accent_hover: ThemeColor::from_hex("#6B9CFF"),
            accent_soft_bg: ThemeColor::new(0.36, 0.55, 1.0, 0.08),

            hover_bg: ThemeColor::new(1.0, 1.0, 1.0, 0.06),
            active_bg: ThemeColor::new(1.0, 1.0, 1.0, 0.10),
            selected_bg: ThemeColor::new(0.36, 0.55, 1.0, 0.18),
            focus_ring: ThemeColor::new(0.36, 0.55, 1.0, 0.30),

            status_success: ThemeColor::from_hex("#4CAF50"),
            status_warning: ThemeColor::from_hex("#FF9800"),
            status_error: ThemeColor::from_hex("#F44336"),
            status_info: ThemeColor::from_hex("#5B8CFF"),

            editor_gutter_bg: ThemeColor::from_hex("#1E1F24"),
            editor_line_highlight: ThemeColor::new(1.0, 1.0, 1.0, 0.03),
            editor_cursor: ThemeColor::from_hex("#E6EAF2"),
            editor_selection: ThemeColor::new(0.36, 0.55, 1.0, 0.22),
            editor_find_highlight: ThemeColor::new(1.0, 0.60, 0.0, 0.25),
        }
    }

    pub fn light() -> Self {
        Self {
            variant: ThemeVariant::Light,
            app_background: ThemeColor::from_hex("#F4F3EF"),
            shell_background: ThemeColor::from_hex("#F0EFEA"),
            surface_default: ThemeColor::from_hex("#F0EEE8"),
            surface_elevated: ThemeColor::from_hex("#F8F6F2"),
            editor_background: ThemeColor::from_hex("#FBFAF7"),
            input_background: ThemeColor::from_hex("#FFFFFF"),
            status_bar_background: ThemeColor::from_hex("#ECE9E3"),
            activity_rail_background: ThemeColor::from_hex("#E7E4DD"),
            sidebar_background: ThemeColor::from_hex("#F0EEE8"),
            tab_strip_background: ThemeColor::from_hex("#E7E4DD"),
            tab_active_background: ThemeColor::from_hex("#FBFAF7"),
            tab_inactive_background: ThemeColor::from_hex("#F1EEE8"),
            assistant_panel_background: ThemeColor::from_hex("#F2F0EA"),
            bottom_panel_background: ThemeColor::from_hex("#ECE9E3"),

            text_primary: ThemeColor::from_hex("#22262B"),
            text_secondary: ThemeColor::from_hex("#3D434A"),
            text_muted: ThemeColor::from_hex("#616975"),
            text_faint: ThemeColor::from_hex("#8A919D"),
            text_on_accent: ThemeColor::from_hex("#FFFFFF"),
            text_disabled: ThemeColor::from_hex("#B0B6C0"),
            text_link: ThemeColor::from_hex("#426EDB"),

            border_default: ThemeColor::from_hex("#D7D1C7"),
            border_subtle: ThemeColor::new(0.84, 0.82, 0.78, 0.5),
            divider_default: ThemeColor::from_hex("#D7D1C7"),
            divider_subtle: ThemeColor::new(0.84, 0.82, 0.78, 0.4),

            accent: ThemeColor::from_hex("#426EDB"),
            accent_hover: ThemeColor::from_hex("#3A62C8"),
            accent_soft_bg: ThemeColor::new(0.26, 0.43, 0.86, 0.05),

            hover_bg: ThemeColor::new(0.0, 0.0, 0.0, 0.04),
            active_bg: ThemeColor::new(0.0, 0.0, 0.0, 0.08),
            selected_bg: ThemeColor::new(0.26, 0.43, 0.86, 0.08),
            focus_ring: ThemeColor::new(0.26, 0.43, 0.86, 0.25),

            status_success: ThemeColor::from_hex("#2E7D32"),
            status_warning: ThemeColor::from_hex("#E65100"),
            status_error: ThemeColor::from_hex("#C62828"),
            status_info: ThemeColor::from_hex("#426EDB"),

            editor_gutter_bg: ThemeColor::from_hex("#FBFAF7"),
            editor_line_highlight: ThemeColor::new(0.26, 0.43, 0.86, 0.03),
            editor_cursor: ThemeColor::from_hex("#22262B"),
            editor_selection: ThemeColor::new(0.26, 0.43, 0.86, 0.14),
            editor_find_highlight: ThemeColor::new(0.90, 0.40, 0.0, 0.18),
        }
    }

    /// Resolve a panel header background: slightly elevated from the surface below.
    pub fn panel_header_bg(&self) -> ThemeColor {
        match self.variant {
            ThemeVariant::Dark => ThemeColor::from_hex("#1E1F25"),
            ThemeVariant::Light => ThemeColor::from_hex("#E8E5DE"),
        }
    }
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
// ThemeModifiers — brightness/alpha factors for derived theme colors
// ---------------------------------------------------------------------------

/// Named brightness and alpha factors used across the engine shell builder
/// and renderer to derive variant colors from theme tokens.
///
/// Phase 40: Consolidates all `adjust_brightness(N.M)` magic numbers spread
/// across shell_builder.rs, editor.rs, and app.rs into a single contract.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThemeModifiers {
    /// How much to dim the accent color for brand labels / pressed states.
    pub accent_dim: f32,

    /// Brightness for brand label accent fill (titlebar brand label).
    pub brand_accent_dim: f32,

    /// Dim factor for titlebar close/minimize/... button fills.
    pub titlebar_button_dim: f32,

    /// Brightness for selected background text for active items.
    pub selected_brighten: f32,

    /// Brightness for inactive rail/action fills derived from text_faint.
    pub rail_inactive_fill: f32,
    /// Brightness for bottom-rail fills.
    pub rail_bottom_fill: f32,

    /// Brightness for sidebar file item placeholder fills.
    pub sidebar_file_fill: f32,

    /// Brightness for scrollbar track fills.
    pub scrollbar_track_fill: f32,
    /// Brightness for scrollbar thumb fills.
    pub scrollbar_thumb_fill: f32,

    /// Brightness for breadcrumb background.
    pub breadcrumb_bg: f32,
    /// Alpha multiplier for subtle dividers.
    pub divider_subtle_alpha: f32,

    /// Brightness for status segment pill backgrounds.
    pub status_pill_fill: f32,
    /// Brightness for status language badge backgrounds.
    pub status_badge_brighten: f32,

    /// Brightness for panel action button fills.
    pub panel_action_fill: f32,

    /// Dim factor for tab accent strips.
    pub tab_accent_dim: f32,
    /// Active tab bottom separator factor.
    pub tab_separator_dim: f32,

    /// Minimap bar fill factors.
    pub minimap_function_bar: f32,
    pub minimap_type_bar: f32,
    pub minimap_other_bar: f32,
    pub minimap_viewport_fill: f32,
}

impl Default for ThemeModifiers {
    fn default() -> Self {
        Self {
            accent_dim: 0.9,
            brand_accent_dim: 0.82,
            titlebar_button_dim: 0.15,
            selected_brighten: 1.6,
            rail_inactive_fill: 0.18,
            rail_bottom_fill: 0.16,
            sidebar_file_fill: 0.20,
            scrollbar_track_fill: 0.55,
            scrollbar_thumb_fill: 0.25,
            breadcrumb_bg: 0.97,
            divider_subtle_alpha: 0.5,
            status_pill_fill: 0.14,
            status_badge_brighten: 2.2,
            panel_action_fill: 0.18,
            tab_accent_dim: 0.9,
            tab_separator_dim: 0.88,
            minimap_function_bar: 0.40,
            minimap_type_bar: 0.40,
            minimap_other_bar: 0.22,
            minimap_viewport_fill: 0.06,
        }
    }
}

// ---------------------------------------------------------------------------
// PanelStyleTable — role-to-color mapping for generic UI regions
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
    /// Resolve the primary fill color for this panel role from the theme.
    pub fn fill(&self, theme: &EngineTheme, mods: &ThemeModifiers) -> ThemeColor {
        match self {
            Self::TopBar => theme.surface_elevated,
            Self::NavigationRail => theme.activity_rail_background,
            Self::SidePanel => theme.sidebar_background,
            Self::ContentTabStrip => theme.tab_strip_background,
            Self::ContentBreadcrumb => {
                theme.editor_background.adjust_brightness(mods.breadcrumb_bg)
            }
            Self::ContentArea => theme.editor_background,
            Self::AuxiliaryPanelHeader => theme.panel_header_bg(),
            Self::AuxiliaryPanelContent => theme.assistant_panel_background,
            Self::BottomPanel => theme.bottom_panel_background,
            Self::StatusBar => theme.status_bar_background,
            Self::MinimapLane => theme.editor_background,
            Self::BottomDock => theme.surface_default,
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
    /// Whether this state implies the element is interactive (not disabled).
    pub fn is_interactive(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Whether this state indicates active engagement.
    pub fn is_engaged(&self) -> bool {
        matches!(self, Self::Active | Self::Focused | Self::Selected)
    }

    /// Resolve the fill color for a widget background in this state.
    /// `base_bg` is the normal-state fill, theme provides the state overlays.
    pub fn resolve_fill(&self, base_bg: &ThemeColor, theme: &EngineTheme) -> ThemeColor {
        match self {
            Self::Normal => *base_bg,
            Self::Hover => base_bg.blend(theme.hover_bg.to_array()),
            Self::Active => base_bg.blend(theme.active_bg.to_array()),
            Self::Focused => base_bg.blend(theme.focus_ring.to_array()),
            Self::Selected => theme.selected_bg.blend(base_bg.to_array()),
            Self::Disabled => base_bg.adjust_brightness(0.6),
        }
    }

    /// Resolve the text color for a widget label in this state.
    pub fn resolve_text(&self, base_text: &ThemeColor, theme: &EngineTheme) -> ThemeColor {
        match self {
            Self::Disabled => theme.text_disabled,
            Self::Selected => theme.text_primary,
            _ => *base_text,
        }
    }

    /// Whether the widget should show an accent indicator (left strip, ring).
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
