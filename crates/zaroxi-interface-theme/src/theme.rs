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
    /// Dark theme semantic colors — OpenCode-faithful.
    ///
    /// Matches the OpenCode default theme: a pure-neutral ramp from near-black
    /// `#0a0a0a` up to `#eeeeee`, with deliberately light-gray borders
    /// (`#3c3c3c`–`#606060`) so surfaces separate crisply instead of going flat.
    /// Signature accent is OpenCode's peach `#fab283`. Syntax is vivid and
    /// distinct — purple keywords, peach functions, red variables, green strings,
    /// orange numbers, yellow types, cyan operators, near-white punctuation.
    /// Editor sits on the darkest surface; chrome panels lift one neutral step.
    pub fn dark() -> Self {
        Self {
            // Background surfaces — OpenCode neutral ramp; editor on the deepest step.
            app_background: Color::from_hex("#0a0a0a"), // darkStep1 — deepest
            shell_background: Color::from_hex("#141414"), // darkStep2
            panel_background: Color::from_hex("#141414"), // darkStep2 — side panels
            elevated_panel_background: Color::from_hex("#1e1e1e"), // darkStep3 — modals/dropdowns
            editor_background: Color::from_hex("#0a0a0a"), // Editor = background (focused content)
            input_background: Color::from_hex("#1e1e1e"), // darkStep3 — inputs/search box
            status_bar_background: Color::from_hex("#141414"), // darkStep2
            title_bar_background: Color::from_hex("#141414"), // darkStep2
            activity_rail_background: Color::from_hex("#141414"), // darkStep2
            sidebar_background: Color::from_hex("#141414"), // darkStep2
            tab_background: Color::from_hex("#141414"), // Inactive tabs recede into the strip
            tab_active_background: Color::from_hex("#0a0a0a"), // Active tab = editor → connected
            assistant_panel_background: Color::from_hex("#141414"), // darkStep2
            bottom_panel_background: Color::from_hex("#141414"), // darkStep2
            bottom_panel_header_background: Color::from_hex("#1e1e1e"), // darkStep3 — header lift
            assistant_panel_header_background: Color::from_hex("#1e1e1e"), // darkStep3 — header lift

            // Text colors — neutral ramp, strong primary contrast.
            text_primary: Color::from_hex("#eeeeee"), // darkStep12
            text_secondary: Color::from_hex("#b4b4b4"), // Between text and muted
            text_muted: Color::from_hex("#808080"),   // darkStep11
            text_faint: Color::from_hex("#606060"),   // darkStep8 — line numbers, labels
            text_on_accent: Color::from_hex("#0a0a0a"), // Dark text on light peach accent
            text_on_surface: Color::from_hex("#eeeeee"),
            text_disabled: Color::from_hex("#484848"), // darkStep7
            text_link: Color::from_hex("#fab283"),     // Links = primary peach

            // UI elements — light-gray borders for crisp, non-flat separation.
            border: Color::from_hex("#484848"), // darkStep7 — visible separation
            border_subtle: Color::from_hex("#3c3c3c"), // darkStep6
            divider: Color::from_hex("#3c3c3c"), // darkStep6
            divider_subtle: Color::from_hex("#282828"), // darkStep4 — soft
            panel_header_background: Color::from_hex("#1e1e1e"), // darkStep3
            nested_surface_background: Color::from_hex("#1e1e1e"), // darkStep3
            app_chrome_background: Color::from_hex("#0a0a0a"), // darkStep1
            tab_strip_background: Color::from_hex("#141414"), // darkStep2
            accent: Color::from_hex("#fab283"), // OpenCode primary — peach
            accent_hover: Color::from_hex("#ffc09f"), // darkStep10
            accent_soft: Color::from_rgba(0.98, 0.70, 0.51, 0.16), // Soft peach background
            accent_soft_background: Color::from_rgba(0.98, 0.70, 0.51, 0.08), // Very soft peach

            // States — neutral hover/active; selection is a secondary-blue wash that preserves syntax.
            hover_background: Color::from_rgba(1.0, 1.0, 1.0, 0.05), // Hover
            active_background: Color::from_rgba(1.0, 1.0, 1.0, 0.09), // Active
            selected_background: Color::from_rgba(0.98, 0.70, 0.51, 0.16), // Selected list rows — peach
            selected_text_background: Color::from_rgba(0.36, 0.61, 0.96, 0.24), // Text selection — blue
            selected_editor_background: Color::from_rgba(0.36, 0.61, 0.96, 0.20), // Editor selection — blue

            // Status colors — OpenCode semantic set.
            success: Color::from_hex("#7fd88f"), // darkGreen
            warning: Color::from_hex("#f5a742"), // darkOrange
            error: Color::from_hex("#e06c75"),   // darkRed
            info: Color::from_hex("#56b6c2"),    // darkCyan

            // Focus
            focus_ring: Color::from_rgba(0.98, 0.70, 0.51, 0.45), // Peach focus ring

            // Editor specific
            // Gutter matches editor; a subtle border supplies the only separation.
            editor_gutter_background: Color::from_hex("#0a0a0a"), // same as editor_background
            editor_line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.04), // Current line — subtle neutral
            editor_cursor: Color::from_hex("#fab283"), // Cursor = primary peach (high visibility)
            editor_selection: Color::from_rgba(0.36, 0.61, 0.96, 0.22), // Blue selection — keeps syntax legible
            editor_find_highlight: Color::from_rgba(0.96, 0.65, 0.26, 0.30), // Orange find — distinct from selection

            // Syntax colors — OpenCode mapping: vivid and clearly separable.
            syntax_keyword: Color::from_hex("#9d7cd8"), // Keywords — accent purple
            syntax_function: Color::from_hex("#fab283"), // Functions — primary peach
            syntax_method: Color::from_hex("#fab283"),  // Methods — same as functions
            syntax_string: Color::from_hex("#7fd88f"),  // Strings — green
            syntax_comment: Color::from_hex("#808080"), // Comments — textMuted (secondary but readable)
            syntax_type: Color::from_hex("#e5c07b"),    // Types — yellow
            syntax_variable: Color::from_hex("#e06c75"), // Variables — red
            syntax_constant: Color::from_hex("#f5a742"), // Constants — orange (with numbers)
            syntax_number: Color::from_hex("#f5a742"),  // Numbers — orange
            syntax_operator: Color::from_hex("#56b6c2"), // Operators — cyan
            syntax_punctuation: Color::from_hex("#eeeeee"), // Punctuation — neutral text (quieter than hues)
            syntax_attribute: Color::from_hex("#9d7cd8"),   // Attributes/decorators — purple
            syntax_tag: Color::from_hex("#e06c75"),         // Tags — red
            syntax_namespace: Color::from_hex("#e5c07b"),   // Namespaces — yellow (type family)
            syntax_macro: Color::from_hex("#fab283"),       // Macros — peach (function family)
            syntax_property: Color::from_hex("#5c9cf5"), // Properties/fields — secondary blue (≠ variable)
            syntax_parameter: Color::from_hex("#e06c75"), // Parameters — red (variable family)
            syntax_builtin: Color::from_hex("#fab283"),  // Builtins — peach
            syntax_escape: Color::from_hex("#56b6c2"),   // Escape sequences — cyan
            syntax_embedded: Color::from_hex("#7fd88f"), // Embedded languages — green
            syntax_regex: Color::from_hex("#7fd88f"),    // Regex — green
            syntax_markup_heading: Color::from_hex("#9d7cd8"), // Markdown headings — purple
            syntax_markup_list: Color::from_hex("#fab283"), // Markdown lists — peach
            syntax_markup_quote: Color::from_hex("#e5c07b"), // Markdown quotes — yellow
            syntax_markup_link: Color::from_hex("#56b6c2"), // Markdown link text — cyan
            syntax_markup_code: Color::from_hex("#7fd88f"), // Markdown code — green
            syntax_markup_bold: Color::from_hex("#f5a742"), // Markdown strong — orange
            syntax_markup_italic: Color::from_hex("#e5c07b"), // Markdown emphasis — yellow
            syntax_markup_strikethrough: Color::from_hex("#808080"), // Markdown strikethrough — muted
            syntax_lifetime: Color::from_hex("#e5c07b"), // Lifetimes — yellow (type family)
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

    /// Light theme semantic colors — OpenCode-faithful.
    ///
    /// OpenCode's canonical light counterpart: a pure-neutral ramp
    /// (`#fafafa`–`#1a1a1a`) with light-gray borders (`#b8b8b8`) for crisp
    /// separation. To honor the "no harsh white" rule the brightest surface is
    /// the soft `#fafafa` (OpenCode's own step, one notch off pure white) rather
    /// than `#ffffff`. Light primary is blue `#3b7dd8`, accent amber `#d68c27`;
    /// syntax keeps the same semantic intent as dark, deepened for daylight.
    pub fn light() -> Self {
        Self {
            // Background surfaces — OpenCode neutral ramp; editor on the brightest soft-white step.
            app_background: Color::from_hex("#fafafa"), // lightStep2 — soft white (not harsh)
            shell_background: Color::from_hex("#f5f5f5"), // lightStep3
            panel_background: Color::from_hex("#f5f5f5"), // lightStep3 — side panels
            elevated_panel_background: Color::from_hex("#ebebeb"), // lightStep4 — modals/dropdowns
            editor_background: Color::from_hex("#fafafa"), // Editor = brightest content surface
            input_background: Color::from_hex("#ebebeb"), // lightStep4 — inputs/search box
            status_bar_background: Color::from_hex("#f5f5f5"), // lightStep3
            title_bar_background: Color::from_hex("#f5f5f5"), // lightStep3
            activity_rail_background: Color::from_hex("#f5f5f5"), // lightStep3
            sidebar_background: Color::from_hex("#f5f5f5"), // lightStep3
            tab_background: Color::from_hex("#f5f5f5"), // Inactive tabs recede
            tab_active_background: Color::from_hex("#fafafa"), // Active tab = editor → connected
            assistant_panel_background: Color::from_hex("#f5f5f5"), // lightStep3
            bottom_panel_background: Color::from_hex("#f5f5f5"), // lightStep3
            bottom_panel_header_background: Color::from_hex("#ebebeb"), // lightStep4 — header lift
            assistant_panel_header_background: Color::from_hex("#ebebeb"), // lightStep4 — header lift

            // Text colors — neutral ink ramp, strong contrast without glare.
            text_primary: Color::from_hex("#1a1a1a"), // lightStep12
            text_secondary: Color::from_hex("#4a4a4a"), // Between text and muted
            text_muted: Color::from_hex("#8a8a8a"),   // lightStep11
            text_faint: Color::from_hex("#a0a0a0"),   // lightStep8 — line numbers, labels
            text_on_accent: Color::from_hex("#1a1a1a"), // Dark text on amber accent
            text_on_surface: Color::from_hex("#1a1a1a"),
            text_disabled: Color::from_hex("#b8b8b8"), // lightStep7
            text_link: Color::from_hex("#3b7dd8"),     // Links = primary blue

            // UI elements — light-gray borders for crisp, non-flat separation.
            border: Color::from_hex("#b8b8b8"), // lightStep7 — visible separation
            border_subtle: Color::from_hex("#d4d4d4"), // lightStep6
            divider: Color::from_hex("#d4d4d4"), // lightStep6
            divider_subtle: Color::from_hex("#e1e1e1"), // lightStep5 — soft
            panel_header_background: Color::from_hex("#ebebeb"), // lightStep4
            nested_surface_background: Color::from_hex("#ebebeb"), // lightStep4
            app_chrome_background: Color::from_hex("#fafafa"), // lightStep2
            tab_strip_background: Color::from_hex("#f5f5f5"), // lightStep3
            accent: Color::from_hex("#d68c27"), // OpenCode light accent — amber
            accent_hover: Color::from_hex("#c07e22"), // Hover — deeper amber
            accent_soft: Color::from_rgba(0.84, 0.55, 0.15, 0.12), // Soft amber background
            accent_soft_background: Color::from_rgba(0.84, 0.55, 0.15, 0.06), // Very soft amber

            // States — neutral hover/active; selection is a primary-blue wash that preserves syntax.
            hover_background: Color::from_rgba(0.0, 0.0, 0.0, 0.05), // Hover
            active_background: Color::from_rgba(0.0, 0.0, 0.0, 0.09), // Active
            selected_background: Color::from_rgba(0.84, 0.55, 0.15, 0.14), // Selected list rows — amber
            selected_text_background: Color::from_rgba(0.23, 0.49, 0.85, 0.20), // Text selection — blue
            selected_editor_background: Color::from_rgba(0.23, 0.49, 0.85, 0.16), // Editor selection — blue

            // Status colors — OpenCode light semantic set.
            success: Color::from_hex("#3d9a57"), // lightGreen
            warning: Color::from_hex("#d68c27"), // lightOrange
            error: Color::from_hex("#d1383d"),   // lightRed
            info: Color::from_hex("#318795"),    // lightCyan

            // Focus
            focus_ring: Color::from_rgba(0.84, 0.55, 0.15, 0.40), // Amber focus ring

            // Editor specific
            editor_gutter_background: Color::from_hex("#fafafa"), // same as editor_background
            editor_line_highlight: Color::from_rgba(0.0, 0.0, 0.0, 0.04), // Current line — subtle neutral
            editor_cursor: Color::from_hex("#3b7dd8"), // Cursor = primary blue (high visibility)
            editor_selection: Color::from_rgba(0.23, 0.49, 0.85, 0.18), // Blue selection — keeps syntax legible
            editor_find_highlight: Color::from_rgba(0.84, 0.55, 0.15, 0.28), // Amber find — distinct from selection

            // Syntax colors — OpenCode light mapping: deepened for daylight legibility.
            syntax_keyword: Color::from_hex("#d68c27"), // Keywords — accent amber
            syntax_function: Color::from_hex("#3b7dd8"), // Functions — primary blue
            syntax_method: Color::from_hex("#3b7dd8"),  // Methods — same as functions
            syntax_string: Color::from_hex("#3d9a57"),  // Strings — green
            syntax_comment: Color::from_hex("#8a8a8a"), // Comments — textMuted (secondary but readable)
            syntax_type: Color::from_hex("#b0851f"),    // Types — yellow
            syntax_variable: Color::from_hex("#d1383d"), // Variables — red
            syntax_constant: Color::from_hex("#d68c27"), // Constants — orange (with numbers)
            syntax_number: Color::from_hex("#d68c27"),  // Numbers — orange
            syntax_operator: Color::from_hex("#318795"), // Operators — cyan
            syntax_punctuation: Color::from_hex("#1a1a1a"), // Punctuation — neutral text (quieter than hues)
            syntax_attribute: Color::from_hex("#d68c27"),   // Attributes/decorators — amber
            syntax_tag: Color::from_hex("#d1383d"),         // Tags — red
            syntax_namespace: Color::from_hex("#b0851f"),   // Namespaces — yellow (type family)
            syntax_macro: Color::from_hex("#3b7dd8"),       // Macros — blue (function family)
            syntax_property: Color::from_hex("#7b5bb6"), // Properties/fields — secondary purple (≠ variable)
            syntax_parameter: Color::from_hex("#d1383d"), // Parameters — red (variable family)
            syntax_builtin: Color::from_hex("#3b7dd8"),  // Builtins — blue
            syntax_escape: Color::from_hex("#318795"),   // Escape sequences — cyan
            syntax_embedded: Color::from_hex("#3d9a57"), // Embedded languages — green
            syntax_regex: Color::from_hex("#3d9a57"),    // Regex — green
            syntax_markup_heading: Color::from_hex("#d68c27"), // Markdown headings — amber
            syntax_markup_list: Color::from_hex("#3b7dd8"), // Markdown lists — blue
            syntax_markup_quote: Color::from_hex("#b0851f"), // Markdown quotes — yellow
            syntax_markup_link: Color::from_hex("#318795"), // Markdown link text — cyan
            syntax_markup_code: Color::from_hex("#3d9a57"), // Markdown code — green
            syntax_markup_bold: Color::from_hex("#d68c27"), // Markdown strong — amber
            syntax_markup_italic: Color::from_hex("#b0851f"), // Markdown emphasis — yellow
            syntax_markup_strikethrough: Color::from_hex("#8a8a8a"), // Markdown strikethrough — muted
            syntax_lifetime: Color::from_hex("#b0851f"), // Lifetimes — yellow (type family)
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
