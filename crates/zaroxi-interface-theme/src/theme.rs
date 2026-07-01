//! Theme definitions for Zaroxi
//! This module provides zaroxi_theme variants, design tokens, and semantic colors

use crate::colors::Color;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Theme variants for Zaroxi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZaroxiTheme {
    /// Dark zaroxi_theme
    Dark,
    /// Light zaroxi_theme
    Light,
    /// Use system preference
    System,
}

impl std::fmt::Display for ZaroxiTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZaroxiTheme::Dark => write!(f, "Dark"),
            ZaroxiTheme::Light => write!(f, "Light"),
            ZaroxiTheme::System => write!(f, "System"),
        }
    }
}

impl Default for ZaroxiTheme {
    fn default() -> Self {
        ZaroxiTheme::System
    }
}

impl ZaroxiTheme {
    /// Get all available zaroxi_theme variants
    pub fn all() -> Vec<Self> {
        vec![ZaroxiTheme::System, ZaroxiTheme::Light, ZaroxiTheme::Dark]
    }

    /// Get display name for the zaroxi_theme
    pub fn display_name(&self) -> &'static str {
        match self {
            ZaroxiTheme::System => "System",
            ZaroxiTheme::Light => "Light",
            ZaroxiTheme::Dark => "Dark",
        }
    }

    /// Resolve to concrete theme (Dark or Light) based on system preference if needed
    pub fn resolve(&self, system_is_dark: bool) -> Self {
        match self {
            ZaroxiTheme::Dark => ZaroxiTheme::Dark,
            ZaroxiTheme::Light => ZaroxiTheme::Light,
            ZaroxiTheme::System => {
                if system_is_dark {
                    ZaroxiTheme::Dark
                } else {
                    ZaroxiTheme::Light
                }
            }
        }
    }

    /// Get the semantic colors for this zaroxi_theme
    pub fn colors(&self, system_is_dark: bool) -> SemanticColors {
        match self.resolve(system_is_dark) {
            ZaroxiTheme::Dark => SemanticColors::dark(),
            ZaroxiTheme::Light => SemanticColors::light(),
            ZaroxiTheme::System => unreachable!(), // Should be resolved above
        }
    }
}

/// Design system tokens for Zaroxi
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DesignTokens {
    // Spacing scale (in pixels)
    pub spacing_xxs: f32,
    pub spacing_xs: f32,
    pub spacing_sm: f32,
    pub spacing_md: f32,
    pub spacing_lg: f32,
    pub spacing_xl: f32,
    pub spacing_xxl: f32,

    // Border radius
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,

    // Border widths
    pub border_width: f32,
    pub border_width_thick: f32,

    // Typography
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub font_size_xxl: f32,
}

impl Default for DesignTokens {
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

            font_size_sm: 12.0,
            font_size_md: 14.0,
            font_size_lg: 16.0,
            font_size_xl: 20.0,
            font_size_xxl: 24.0,
        }
    }
}

/// Semantic color roles for Zaroxi IDE
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SemanticColors {
    // Background surfaces - hierarchy from deepest to highest
    pub app_background: Color,
    pub shell_background: Color,
    pub panel_background: Color,
    pub elevated_panel_background: Color,
    pub editor_background: Color,
    pub input_background: Color,
    pub status_bar_background: Color,
    pub title_bar_background: Color,
    pub activity_rail_background: Color,
    pub sidebar_background: Color,
    pub tab_background: Color,
    pub tab_active_background: Color,
    pub assistant_panel_background: Color,
    pub bottom_panel_background: Color,
    pub bottom_panel_header_background: Color,
    pub assistant_panel_header_background: Color,

    // Text colors - hierarchy from most prominent to subtle
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_faint: Color,
    pub text_on_accent: Color,
    pub text_on_surface: Color,
    pub text_disabled: Color,
    pub text_link: Color,

    // UI elements
    pub border: Color,
    pub border_subtle: Color,
    pub divider: Color,
    pub divider_subtle: Color,
    pub panel_header_background: Color,
    pub nested_surface_background: Color,
    pub app_chrome_background: Color,
    pub tab_strip_background: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_soft: Color,
    pub accent_soft_background: Color,

    // States
    pub hover_background: Color,
    pub active_background: Color,
    pub selected_background: Color,
    pub selected_text_background: Color,
    pub selected_editor_background: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Git diff gutter cues — dedicated tokens, intentionally distinct from
    // `error`/`success` and the syntax hues, so changed-line markers read as
    // their own editor-chrome cue rather than colliding with `syntax_property`.
    pub diff_added: Color,
    pub diff_removed: Color,

    // Focus
    pub focus_ring: Color,

    // Editor specific
    pub editor_gutter_background: Color,
    pub editor_line_highlight: Color,
    pub editor_cursor: Color,
    pub editor_selection: Color,
    pub editor_find_highlight: Color,

    // Syntax colors (basic set for IDE)
    pub syntax_keyword: Color,
    pub syntax_function: Color,
    pub syntax_method: Color,
    pub syntax_string: Color,
    pub syntax_comment: Color,
    pub syntax_type: Color,
    pub syntax_variable: Color,
    pub syntax_constant: Color,
    pub syntax_number: Color,
    pub syntax_operator: Color,
    pub syntax_punctuation: Color,
    pub syntax_attribute: Color,
    pub syntax_tag: Color,
    pub syntax_namespace: Color,
    pub syntax_macro: Color,
    pub syntax_property: Color,
    pub syntax_parameter: Color,
    pub syntax_builtin: Color,
    pub syntax_escape: Color,
    pub syntax_embedded: Color,
    pub syntax_regex: Color,
    pub syntax_markup_heading: Color,
    pub syntax_markup_list: Color,
    pub syntax_markup_quote: Color,
    pub syntax_markup_link: Color,
    pub syntax_markup_code: Color,
    pub syntax_markup_bold: Color,
    pub syntax_markup_italic: Color,
    pub syntax_markup_strikethrough: Color,
    pub syntax_lifetime: Color,
}

impl SemanticColors {
    /// Dark theme semantic colors — Zaroxi Studio (definitive).
    ///
    /// The recognizable Zaroxi navy identity restored as the brand base — deep
    /// planes `bg.root #0B0F1A` → `bg.panel #10172A` → `bg.editor #0E1322` (hero)
    /// → `bg.deep #0A0D18`, separated by quiet architectural borders. Brand purple
    /// (`accent.primary #7C5CFF`) is the memory color; cyan (`accent.secondary
    /// #00C2E8`) is the restrained AI/computation signal. Syntax is a completely
    /// rebuilt, long-session-friendly hierarchy: most identifiers rest on a neutral
    /// text ramp; purple = structure, blue = callables, gold = types, sage = strings,
    /// warm sand = numbers, cyan = literals/links — each hue with one clear job.
    pub fn dark() -> Self {
        Self {
            // Background surfaces — deep navy planes (root → panels → editor hero → deep recess).
            app_background: Color::from_hex("#0B0F1A"), // bg.root — app frame
            shell_background: Color::from_hex("#10172A"), // bg.panel — panel shells
            panel_background: Color::from_hex("#10172A"), // bg.panel
            elevated_panel_background: Color::from_hex("#121A30"), // surface.overlay — popovers/dialogs
            editor_background: Color::from_hex("#0E1322"),         // bg.editor — the hero canvas
            input_background: Color::from_hex("#121A30"), // surface.default — search/form fields
            status_bar_background: Color::from_hex("#0A0D18"), // bg.deep — status bar
            title_bar_background: Color::from_hex("#10172A"), // bg.panel — title bar
            activity_rail_background: Color::from_hex("#10172A"), // bg.panel
            sidebar_background: Color::from_hex("#10172A"), // bg.panel — file explorer shell
            tab_background: Color::from_hex("#121A30"),   // surface.default — inactive tab
            tab_active_background: Color::from_hex("#1A2545"), // surface.active — active tab
            assistant_panel_background: Color::from_hex("#10172A"), // bg.panel — AI shell
            bottom_panel_background: Color::from_hex("#0A0D18"), // bg.deep — terminal/problems
            bottom_panel_header_background: Color::from_hex("#10172A"), // bg.panel — bottom tab strip
            assistant_panel_header_background: Color::from_hex("#10172A"), // bg.panel — AI header

            // Text colors — soft off-white ramp (no pure white → no halation).
            text_primary: Color::from_hex("#E6E9F2"), // text.primary — body code
            text_secondary: Color::from_hex("#A9B1D6"), // text.secondary — labels, operators
            text_muted: Color::from_hex("#6B7394"),   // text.muted — comments, line numbers
            text_faint: Color::from_hex("#525A78"),   // text.faint — invisibles, hints
            text_on_accent: Color::from_hex("#0B0F1A"), // text.inverse — on purple accent
            text_on_surface: Color::from_hex("#E6E9F2"), // text.primary
            text_disabled: Color::from_hex("#474E68"), // text.disabled
            text_link: Color::from_hex("#00C2E8"), // accent.secondary — interactive/computational

            // UI elements — quiet architectural separators, purple brand accent.
            border: Color::from_hex("#223055"), // border.default
            border_subtle: Color::from_hex("#1C2746"), // border.subtle
            divider: Color::from_hex("#1C2746"), // border.subtle — major dividers
            divider_subtle: Color::from_hex("#1C2746"), // border.subtle
            panel_header_background: Color::from_hex("#10172A"), // bg.panel
            nested_surface_background: Color::from_hex("#121A30"), // surface.default
            app_chrome_background: Color::from_hex("#0B0F1A"), // bg.root — frame
            tab_strip_background: Color::from_hex("#10172A"), // bg.panel — tab strip
            accent: Color::from_hex("#7C5CFF"), // accent.primary — brand
            accent_hover: Color::from_hex("#9B7CFF"), // accent.primaryHover
            accent_soft: Color::from_rgba(0.4863, 0.3608, 1.0, 0.22), // glow.primary
            accent_soft_background: Color::from_rgba(0.4863, 0.3608, 1.0, 0.12), // glow.primary (soft)

            // States — flat surface tokens for hover/active/selected; selection uses glow.primary.
            hover_background: Color::from_hex("#16203A"), // surface.hover
            active_background: Color::from_hex("#1A2545"), // surface.active
            selected_background: Color::from_hex("#1A2545"), // surface.active — selected rows
            selected_text_background: Color::from_rgba(0.4863, 0.3608, 1.0, 0.22), // glow.primary
            selected_editor_background: Color::from_rgba(0.4863, 0.3608, 1.0, 0.22), // glow.primary

            // Status colors — refined (calm, not neon).
            success: Color::from_hex("#7FBF7F"), // status.success — calm green
            warning: Color::from_hex("#E0B35A"), // status.warning — soft amber
            error: Color::from_hex("#F0718C"),   // status.error — refined rose
            info: Color::from_hex("#66AEEA"),    // status.info — soft blue
            diff_added: Color::from_hex("#7FBF7F"), // status.success — git added
            diff_removed: Color::from_hex("#F0718C"), // status.error — git removed

            // Focus — restrained purple ring only.
            focus_ring: Color::from_rgba(0.4863, 0.3608, 1.0, 0.22), // glow.primary

            // Editor specific.
            editor_gutter_background: Color::from_hex("#0E1322"), // bg.editor — gutter matches canvas
            editor_line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.035), // active line — subtle, no loud stripe
            editor_cursor: Color::from_hex("#7C5CFF"),                     // accent.primary
            editor_selection: Color::from_rgba(0.4863, 0.3608, 1.0, 0.22), // glow.primary
            editor_find_highlight: Color::from_rgba(0.8784, 0.702, 0.3529, 0.28), // status.warning — reduced surface

            // Syntax — distance-readable: fewer, clearer semantic groups on a neutral ramp.
            syntax_keyword: Color::from_hex("#B8A1FF"), // syntax.keyword — calm structural purple
            syntax_function: Color::from_hex("#7FB7FF"), // syntax.callable — distinct blue
            syntax_method: Color::from_hex("#7FB7FF"),  // syntax.callable
            syntax_string: Color::from_hex("#98C379"),  // syntax.string — softened green
            syntax_comment: Color::from_hex("#7581A1"), // syntax.comment — clearly secondary, still legible
            syntax_type: Color::from_hex("#F0C674"),    // syntax.type — warm gold
            syntax_variable: Color::from_hex("#E6E9F2"), // syntax.identifier — neutral (most-read)
            syntax_constant: Color::from_hex("#56C7D9"), // syntax.constant — cyan-teal literal
            syntax_number: Color::from_hex("#E5A96B"),  // syntax.number — warm orange-sand
            syntax_operator: Color::from_hex("#B2BCDA"), // syntax.secondary — quieter than keywords/callables
            syntax_punctuation: Color::from_hex("#5D6787"), // syntax.punctuation — pushed back strongly
            syntax_attribute: Color::from_hex("#56C7D9"),   // syntax.constant — attribute name
            syntax_tag: Color::from_hex("#B8A1FF"),         // syntax.keyword — markup structure
            syntax_namespace: Color::from_hex("#B2BCDA"),   // syntax.secondary
            syntax_macro: Color::from_hex("#7FB7FF"),       // syntax.callable
            syntax_property: Color::from_hex("#B2BCDA"),    // syntax.secondary
            syntax_parameter: Color::from_hex("#E6E9F2"),   // syntax.identifier
            syntax_builtin: Color::from_hex("#7FB7FF"),     // syntax.callable — builtin function
            syntax_escape: Color::from_hex("#E0B35A"),      // status.warning
            syntax_embedded: Color::from_hex("#E6E9F2"),    // syntax.identifier
            syntax_regex: Color::from_hex("#E0B35A"),       // status.warning (regexp)
            syntax_markup_heading: Color::from_hex("#B8A1FF"), // syntax.keyword
            syntax_markup_list: Color::from_hex("#B2BCDA"), // syntax.secondary
            syntax_markup_quote: Color::from_hex("#7581A1"), // syntax.comment
            syntax_markup_link: Color::from_hex("#56C7D9"), // syntax.constant — URL/link
            syntax_markup_code: Color::from_hex("#98C379"), // syntax.string — inline code
            syntax_markup_bold: Color::from_hex("#E6E9F2"), // syntax.identifier — strong
            syntax_markup_italic: Color::from_hex("#B2BCDA"), // syntax.secondary — emphasis
            syntax_markup_strikethrough: Color::from_hex("#7581A1"), // syntax.comment
            syntax_lifetime: Color::from_hex("#5D6787"), // syntax.punctuation — quiet special marker
        }
    }

    /// High-contrast debug theme for proving theme plumbing visually.
    /// Activated by env var `ZAROXI_DEBUG_THEME=1` on the host side.
    /// Uses unmistakably different surface colors so any visual change
    /// confirms the theme pipeline is live end-to-end.
    pub fn debug() -> Self {
        let mut sem = Self::dark();
        sem.app_background = Color::from_hex("#0D142D");
        sem.shell_background = Color::from_hex("#0F1A3C");
        sem.panel_background = Color::from_hex("#141F44");
        sem.elevated_panel_background = Color::from_hex("#1A254A");
        sem.editor_background = Color::from_hex("#0A142E");
        sem.input_background = Color::from_hex("#121C3D");
        sem.status_bar_background = Color::from_hex("#26408A");
        sem.title_bar_background = Color::from_hex("#0F1A42");
        sem.activity_rail_background = Color::from_hex("#264466");
        sem.sidebar_background = Color::from_hex("#2E3866");
        sem.tab_background = Color::from_hex("#141F4A");
        sem.tab_active_background = Color::from_hex("#0A142E");
        sem.assistant_panel_background = Color::from_hex("#382648");
        sem.bottom_panel_background = Color::from_hex("#264080");
        sem.bottom_panel_header_background = Color::from_hex("#33558C");
        sem.assistant_panel_header_background = Color::from_hex("#2E1E3D");
        sem
    }

    /// Light theme semantic colors — Zaroxi Studio (definitive).
    ///
    /// The same Zaroxi product identity in soft architectural cool-white — never a
    /// different app. Foundations `bg.root #F6F8FC`, `bg.editor #FCFDFF` (soft, no
    /// glare), `bg.panel #F0F3FA`, `bg.deep #E8ECF7` stay premium and calm. Brand
    /// purple (`#7C5CFF`) is preserved for cross-mode recognition; the secondary
    /// accent deepens to readable teal (`#0E93B0`). Syntax mirrors the dark theme's
    /// hierarchy with darkened hue families so nothing washes out on light surfaces.
    pub fn light() -> Self {
        Self {
            // Background surfaces — soft cool-white planes (root → panels → editor hero → deep recess).
            app_background: Color::from_hex("#F6F8FC"), // bg.root — app frame
            shell_background: Color::from_hex("#F0F3FA"), // bg.panel — panel shells
            panel_background: Color::from_hex("#F0F3FA"), // bg.panel
            elevated_panel_background: Color::from_hex("#FFFFFF"), // surface.overlay — popovers/dialogs
            editor_background: Color::from_hex("#FCFDFF"), // bg.editor — soft white, no glare
            input_background: Color::from_hex("#FFFFFF"),  // surface.default — search/form fields
            status_bar_background: Color::from_hex("#E8ECF7"), // bg.deep — status bar
            title_bar_background: Color::from_hex("#F0F3FA"), // bg.panel — title bar
            activity_rail_background: Color::from_hex("#F0F3FA"), // bg.panel
            sidebar_background: Color::from_hex("#F0F3FA"), // bg.panel — file explorer shell
            tab_background: Color::from_hex("#FFFFFF"),    // surface.default — inactive tab
            tab_active_background: Color::from_hex("#E9EEFF"), // surface.active — active tab
            assistant_panel_background: Color::from_hex("#F0F3FA"), // bg.panel — AI shell
            bottom_panel_background: Color::from_hex("#E8ECF7"), // bg.deep — terminal/problems
            bottom_panel_header_background: Color::from_hex("#F0F3FA"), // bg.panel — bottom tab strip
            assistant_panel_header_background: Color::from_hex("#F0F3FA"), // bg.panel — AI header

            // Text colors — near-ink ramp, high-contrast readable-first.
            text_primary: Color::from_hex("#1A2138"), // text.primary — body code
            text_secondary: Color::from_hex("#4A5578"), // text.secondary — labels, operators
            text_muted: Color::from_hex("#7B85A3"),   // text.muted — comments, line numbers
            text_faint: Color::from_hex("#96A0BB"),   // text.faint — invisibles, hints
            text_on_accent: Color::from_hex("#FFFFFF"), // text.inverse — on purple accent
            text_on_surface: Color::from_hex("#1A2138"), // text.primary
            text_disabled: Color::from_hex("#A1A9C0"), // text.disabled
            text_link: Color::from_hex("#0E93B0"), // accent.secondary — interactive/computational

            // UI elements — quiet architectural separators, purple brand accent.
            border: Color::from_hex("#C8D2EA"), // border.default
            border_subtle: Color::from_hex("#D6DDF0"), // border.subtle
            divider: Color::from_hex("#D6DDF0"), // border.subtle — major dividers
            divider_subtle: Color::from_hex("#D6DDF0"), // border.subtle
            panel_header_background: Color::from_hex("#F0F3FA"), // bg.panel
            nested_surface_background: Color::from_hex("#FFFFFF"), // surface.default
            app_chrome_background: Color::from_hex("#F6F8FC"), // bg.root — frame
            tab_strip_background: Color::from_hex("#F0F3FA"), // bg.panel — tab strip
            accent: Color::from_hex("#7C5CFF"), // accent.primary — brand
            accent_hover: Color::from_hex("#6A4DFF"), // accent.primaryHover
            accent_soft: Color::from_rgba(0.4863, 0.3608, 1.0, 0.16), // glow.primary
            accent_soft_background: Color::from_rgba(0.4863, 0.3608, 1.0, 0.08), // glow.primary (soft)

            // States — flat surface tokens for hover/active/selected; selection uses glow.primary.
            hover_background: Color::from_hex("#F4F7FF"), // surface.hover
            active_background: Color::from_hex("#E9EEFF"), // surface.active
            selected_background: Color::from_hex("#E9EEFF"), // surface.active — selected rows
            selected_text_background: Color::from_rgba(0.4863, 0.3608, 1.0, 0.16), // glow.primary
            selected_editor_background: Color::from_rgba(0.4863, 0.3608, 1.0, 0.16), // glow.primary

            // Status colors — refined (readable on white, not washed out).
            success: Color::from_hex("#3F8C56"), // status.success
            warning: Color::from_hex("#B7791F"), // status.warning
            error: Color::from_hex("#D85B72"),   // status.error
            info: Color::from_hex("#2F73D9"),    // status.info
            diff_added: Color::from_hex("#3F8C56"), // status.success — git added
            diff_removed: Color::from_hex("#D85B72"), // status.error — git removed

            // Focus — restrained purple ring only.
            focus_ring: Color::from_rgba(0.4863, 0.3608, 1.0, 0.16), // glow.primary

            // Editor specific.
            editor_gutter_background: Color::from_hex("#FCFDFF"), // bg.editor — gutter matches canvas
            editor_line_highlight: Color::from_rgba(0.102, 0.129, 0.2196, 0.045), // active line — subtle
            editor_cursor: Color::from_hex("#7C5CFF"),                            // accent.primary
            editor_selection: Color::from_rgba(0.4863, 0.3608, 1.0, 0.16),        // glow.primary
            editor_find_highlight: Color::from_rgba(0.7176, 0.4745, 0.1216, 0.24), // status.warning — reduced surface

            // Syntax — distance-readable: fewer, clearer semantic groups; darkened so nothing washes out.
            syntax_keyword: Color::from_hex("#6F42D6"), // syntax.keyword — structural purple (deep for white)
            syntax_function: Color::from_hex("#2F6FD6"), // syntax.callable — distinct blue
            syntax_method: Color::from_hex("#2F6FD6"),  // syntax.callable
            syntax_string: Color::from_hex("#3E8A50"),  // syntax.string — readable green
            syntax_comment: Color::from_hex("#6E7896"), // syntax.comment — clearly secondary, still legible
            syntax_type: Color::from_hex("#9A6A12"), // syntax.type — warm gold (readable on white)
            syntax_variable: Color::from_hex("#1A2138"), // syntax.identifier — neutral (most-read)
            syntax_constant: Color::from_hex("#0D8DAA"), // syntax.constant — teal literal
            syntax_number: Color::from_hex("#B86A22"), // syntax.number — warm orange-sand
            syntax_operator: Color::from_hex("#4A5578"), // syntax.secondary — quieter than keywords/callables
            syntax_punctuation: Color::from_hex("#8A93AE"), // syntax.punctuation — pushed back strongly
            syntax_attribute: Color::from_hex("#0D8DAA"),   // syntax.constant — attribute name
            syntax_tag: Color::from_hex("#6F42D6"),         // syntax.keyword — markup structure
            syntax_namespace: Color::from_hex("#4A5578"),   // syntax.secondary
            syntax_macro: Color::from_hex("#2F6FD6"),       // syntax.callable
            syntax_property: Color::from_hex("#4A5578"),    // syntax.secondary
            syntax_parameter: Color::from_hex("#1A2138"),   // syntax.identifier
            syntax_builtin: Color::from_hex("#2F6FD6"),     // syntax.callable — builtin function
            syntax_escape: Color::from_hex("#B7791F"),      // status.warning
            syntax_embedded: Color::from_hex("#1A2138"),    // syntax.identifier
            syntax_regex: Color::from_hex("#B7791F"),       // status.warning (regexp)
            syntax_markup_heading: Color::from_hex("#6F42D6"), // syntax.keyword
            syntax_markup_list: Color::from_hex("#4A5578"), // syntax.secondary
            syntax_markup_quote: Color::from_hex("#6E7896"), // syntax.comment
            syntax_markup_link: Color::from_hex("#0D8DAA"), // syntax.constant — URL/link
            syntax_markup_code: Color::from_hex("#3E8A50"), // syntax.string — inline code
            syntax_markup_bold: Color::from_hex("#1A2138"), // syntax.identifier — strong
            syntax_markup_italic: Color::from_hex("#4A5578"), // syntax.secondary — emphasis
            syntax_markup_strikethrough: Color::from_hex("#6E7896"), // syntax.comment
            syntax_lifetime: Color::from_hex("#8A93AE"), // syntax.punctuation — quiet special marker
        }
    }

    /// Serialize semantic colors into CSS variable map that the frontend consumes.
    ///
    /// Returns a JSON object mapping CSS custom property name -> string color value.
    /// Example key: "--color-editor-background" -> "rgba(30,31,36,1)"
    pub fn to_css_vars(&self) -> Value {
        let mut m: Map<String, Value> = Map::new();

        // Background surfaces
        m.insert(
            "--color-app-background".to_string(),
            Value::String(self.app_background.to_css_rgba()),
        );
        m.insert(
            "--color-shell-background".to_string(),
            Value::String(self.shell_background.to_css_rgba()),
        );
        m.insert(
            "--color-panel-background".to_string(),
            Value::String(self.panel_background.to_css_rgba()),
        );
        m.insert(
            "--color-elevated-panel-background".to_string(),
            Value::String(self.elevated_panel_background.to_css_rgba()),
        );
        m.insert(
            "--color-editor-background".to_string(),
            Value::String(self.editor_background.to_css_rgba()),
        );
        m.insert(
            "--color-input-background".to_string(),
            Value::String(self.input_background.to_css_rgba()),
        );
        m.insert(
            "--color-status-bar-background".to_string(),
            Value::String(self.status_bar_background.to_css_rgba()),
        );
        m.insert(
            "--color-title-bar-background".to_string(),
            Value::String(self.title_bar_background.to_css_rgba()),
        );
        m.insert(
            "--color-activity-rail-background".to_string(),
            Value::String(self.activity_rail_background.to_css_rgba()),
        );
        m.insert(
            "--color-sidebar-background".to_string(),
            Value::String(self.sidebar_background.to_css_rgba()),
        );
        m.insert(
            "--color-tab-background".to_string(),
            Value::String(self.tab_background.to_css_rgba()),
        );
        m.insert(
            "--color-tab-active-background".to_string(),
            Value::String(self.tab_active_background.to_css_rgba()),
        );
        m.insert(
            "--color-assistant-panel-background".to_string(),
            Value::String(self.assistant_panel_background.to_css_rgba()),
        );
        m.insert(
            "--color-bottom-panel-background".to_string(),
            Value::String(self.bottom_panel_background.to_css_rgba()),
        );
        m.insert(
            "--color-bottom-panel-header-background".to_string(),
            Value::String(self.bottom_panel_header_background.to_css_rgba()),
        );
        m.insert(
            "--color-assistant-panel-header-background".to_string(),
            Value::String(self.assistant_panel_header_background.to_css_rgba()),
        );

        // Text colors
        m.insert(
            "--color-text-primary".to_string(),
            Value::String(self.text_primary.to_css_rgba()),
        );
        m.insert(
            "--color-text-secondary".to_string(),
            Value::String(self.text_secondary.to_css_rgba()),
        );
        m.insert("--color-text-muted".to_string(), Value::String(self.text_muted.to_css_rgba()));
        m.insert("--color-text-faint".to_string(), Value::String(self.text_faint.to_css_rgba()));
        m.insert(
            "--color-text-on-accent".to_string(),
            Value::String(self.text_on_accent.to_css_rgba()),
        );
        m.insert(
            "--color-text-on-surface".to_string(),
            Value::String(self.text_on_surface.to_css_rgba()),
        );
        m.insert(
            "--color-text-disabled".to_string(),
            Value::String(self.text_disabled.to_css_rgba()),
        );
        m.insert("--color-text-link".to_string(), Value::String(self.text_link.to_css_rgba()));

        // UI elements
        m.insert("--color-border".to_string(), Value::String(self.border.to_css_rgba()));
        m.insert(
            "--color-border-subtle".to_string(),
            Value::String(self.border_subtle.to_css_rgba()),
        );
        m.insert("--color-divider".to_string(), Value::String(self.divider.to_css_rgba()));
        m.insert(
            "--color-divider-subtle".to_string(),
            Value::String(self.divider_subtle.to_css_rgba()),
        );
        m.insert(
            "--color-panel-header-background".to_string(),
            Value::String(self.panel_header_background.to_css_rgba()),
        );
        m.insert(
            "--color-nested-surface-background".to_string(),
            Value::String(self.nested_surface_background.to_css_rgba()),
        );
        m.insert(
            "--color-app-chrome-background".to_string(),
            Value::String(self.app_chrome_background.to_css_rgba()),
        );
        m.insert(
            "--color-tab-strip-background".to_string(),
            Value::String(self.tab_strip_background.to_css_rgba()),
        );
        m.insert("--color-accent".to_string(), Value::String(self.accent.to_css_rgba()));
        m.insert(
            "--color-accent-hover".to_string(),
            Value::String(self.accent_hover.to_css_rgba()),
        );
        m.insert("--color-accent-soft".to_string(), Value::String(self.accent_soft.to_css_rgba()));
        m.insert(
            "--color-accent-soft-background".to_string(),
            Value::String(self.accent_soft_background.to_css_rgba()),
        );

        // States
        m.insert(
            "--color-hover-background".to_string(),
            Value::String(self.hover_background.to_css_rgba()),
        );
        m.insert(
            "--color-active-background".to_string(),
            Value::String(self.active_background.to_css_rgba()),
        );
        m.insert(
            "--color-selected-background".to_string(),
            Value::String(self.selected_background.to_css_rgba()),
        );
        m.insert(
            "--color-selected-text-background".to_string(),
            Value::String(self.selected_text_background.to_css_rgba()),
        );
        m.insert(
            "--color-selected-editor-background".to_string(),
            Value::String(self.selected_editor_background.to_css_rgba()),
        );

        // Status colors
        m.insert("--color-success".to_string(), Value::String(self.success.to_css_rgba()));
        m.insert("--color-warning".to_string(), Value::String(self.warning.to_css_rgba()));
        m.insert("--color-error".to_string(), Value::String(self.error.to_css_rgba()));
        m.insert("--color-info".to_string(), Value::String(self.info.to_css_rgba()));

        // Focus
        m.insert("--color-focus-ring".to_string(), Value::String(self.focus_ring.to_css_rgba()));

        // Editor specific
        m.insert(
            "--color-editor-gutter-background".to_string(),
            Value::String(self.editor_gutter_background.to_css_rgba()),
        );
        m.insert(
            "--color-editor-line-highlight".to_string(),
            Value::String(self.editor_line_highlight.to_css_rgba()),
        );
        m.insert(
            "--color-editor-cursor".to_string(),
            Value::String(self.editor_cursor.to_css_rgba()),
        );
        m.insert(
            "--color-editor-selection".to_string(),
            Value::String(self.editor_selection.to_css_rgba()),
        );
        m.insert(
            "--color-editor-find-highlight".to_string(),
            Value::String(self.editor_find_highlight.to_css_rgba()),
        );

        // Syntax colors
        m.insert(
            "--color-syntax-keyword".to_string(),
            Value::String(self.syntax_keyword.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-function".to_string(),
            Value::String(self.syntax_function.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-method".to_string(),
            Value::String(self.syntax_method.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-string".to_string(),
            Value::String(self.syntax_string.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-comment".to_string(),
            Value::String(self.syntax_comment.to_css_rgba()),
        );
        m.insert("--color-syntax-type".to_string(), Value::String(self.syntax_type.to_css_rgba()));
        m.insert(
            "--color-syntax-variable".to_string(),
            Value::String(self.syntax_variable.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-constant".to_string(),
            Value::String(self.syntax_constant.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-number".to_string(),
            Value::String(self.syntax_number.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-operator".to_string(),
            Value::String(self.syntax_operator.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-punctuation".to_string(),
            Value::String(self.syntax_punctuation.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-attribute".to_string(),
            Value::String(self.syntax_attribute.to_css_rgba()),
        );
        m.insert("--color-syntax-tag".to_string(), Value::String(self.syntax_tag.to_css_rgba()));
        m.insert(
            "--color-syntax-namespace".to_string(),
            Value::String(self.syntax_namespace.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-macro".to_string(),
            Value::String(self.syntax_macro.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-property".to_string(),
            Value::String(self.syntax_property.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-parameter".to_string(),
            Value::String(self.syntax_parameter.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-builtin".to_string(),
            Value::String(self.syntax_builtin.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-escape".to_string(),
            Value::String(self.syntax_escape.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-embedded".to_string(),
            Value::String(self.syntax_embedded.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-regex".to_string(),
            Value::String(self.syntax_regex.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-markup-heading".to_string(),
            Value::String(self.syntax_markup_heading.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-markup-list".to_string(),
            Value::String(self.syntax_markup_list.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-markup-quote".to_string(),
            Value::String(self.syntax_markup_quote.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-markup-link".to_string(),
            Value::String(self.syntax_markup_link.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-markup-code".to_string(),
            Value::String(self.syntax_markup_code.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-markup-bold".to_string(),
            Value::String(self.syntax_markup_bold.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-markup-italic".to_string(),
            Value::String(self.syntax_markup_italic.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-markup-strikethrough".to_string(),
            Value::String(self.syntax_markup_strikethrough.to_css_rgba()),
        );
        m.insert(
            "--color-syntax-lifetime".to_string(),
            Value::String(self.syntax_lifetime.to_css_rgba()),
        );

        Value::Object(m)
    }
}
