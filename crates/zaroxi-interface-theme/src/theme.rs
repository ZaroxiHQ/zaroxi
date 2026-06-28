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
    /// Dark theme semantic colors — Zed "One Dark"–faithful.
    ///
    /// Ported from Zed's `One Dark` (`assets/themes/one/one.json`): cool slate
    /// surfaces with a crisp hierarchy — deep editor `#282c33`, panels/sidebars
    /// lifted to `#2f343e`, window chrome (title/status) lightest at `#3b414d`,
    /// subtle-but-effective borders `#464b57`/`#363c46`, restrained blue accent
    /// `#74ade8`. Syntax is the classic One Dark reading: purple keywords, blue
    /// functions, cyan-teal types, green strings, orange numbers, red properties,
    /// foreground variables/punctuation, dim-but-readable comments.
    pub fn dark() -> Self {
        Self {
            // Background surfaces — Zed One Dark slate hierarchy (editor deepest → chrome lightest).
            app_background: Color::from_hex("#282c33"), // Editor-deep backdrop
            shell_background: Color::from_hex("#2f343e"), // Lifted surface (panels)
            panel_background: Color::from_hex("#2f343e"), // surface.background
            elevated_panel_background: Color::from_hex("#363c46"), // Modals/dropdowns — lifted above panels
            editor_background: Color::from_hex("#282c33"), // editor.background — deep, focused
            input_background: Color::from_hex("#2e343e"),  // element.background — inset fields
            status_bar_background: Color::from_hex("#3b414d"), // status_bar.background — chrome frame
            title_bar_background: Color::from_hex("#3b414d"), // title_bar.background — chrome frame
            activity_rail_background: Color::from_hex("#2f343e"), // panel.background
            sidebar_background: Color::from_hex("#2f343e"), // panel.background — lifted from editor
            tab_background: Color::from_hex("#2f343e"),     // tab.inactive_background — recedes
            tab_active_background: Color::from_hex("#282c33"), // tab.active_background = editor → connected
            assistant_panel_background: Color::from_hex("#2f343e"), // panel.background
            bottom_panel_background: Color::from_hex("#2f343e"), // panel.background
            bottom_panel_header_background: Color::from_hex("#2f343e"), // editor.subheader.background
            assistant_panel_header_background: Color::from_hex("#2f343e"), // flattened — match body

            // Text colors — Zed One Dark ramp.
            text_primary: Color::from_hex("#dce0e5"), // text
            text_secondary: Color::from_hex("#c4c9d2"), // Between text and muted
            text_muted: Color::from_hex("#a9afbc"),   // text.muted
            text_faint: Color::from_hex("#6b7888"),   // Dim slate — line numbers, labels
            text_on_accent: Color::from_hex("#1b1f27"), // Dark text on light blue accent
            text_on_surface: Color::from_hex("#dce0e5"),
            text_disabled: Color::from_hex("#565d69"), // Disabled
            text_link: Color::from_hex("#74ade8"),     // text.accent — blue

            // UI elements — subtle but effective slate borders, blue accent.
            border: Color::from_hex("#464b57"),         // border
            border_subtle: Color::from_hex("#363c46"),  // border.variant
            divider: Color::from_hex("#363c46"),        // border.variant
            divider_subtle: Color::from_hex("#2e333c"), // scrollbar.track.border — soft
            panel_header_background: Color::from_hex("#2f343e"), // surface
            nested_surface_background: Color::from_hex("#2e343e"), // element.background
            app_chrome_background: Color::from_hex("#282c33"), // editor-deep
            tab_strip_background: Color::from_hex("#2f343e"), // tab_bar.background
            accent: Color::from_hex("#74ade8"),         // Zed accent — restrained blue
            accent_hover: Color::from_hex("#85c1ff"),   // Brighter blue hover
            accent_soft: Color::from_rgba(0.455, 0.678, 0.910, 0.16), // Soft blue background
            accent_soft_background: Color::from_rgba(0.455, 0.678, 0.910, 0.08), // Very soft blue

            // States — neutral hover/active; selection is the Zed blue wash (preserves syntax).
            hover_background: Color::from_rgba(1.0, 1.0, 1.0, 0.05), // Hover
            active_background: Color::from_rgba(1.0, 1.0, 1.0, 0.09), // Active
            selected_background: Color::from_rgba(0.455, 0.678, 0.910, 0.18), // Selected list rows — blue
            selected_text_background: Color::from_rgba(0.455, 0.678, 0.910, 0.24), // Text selection — blue
            selected_editor_background: Color::from_rgba(0.455, 0.678, 0.910, 0.22), // Editor selection — blue

            // Status colors — Zed One Dark semantic set.
            success: Color::from_hex("#a1c181"), // created / success green
            warning: Color::from_hex("#dec184"), // warning yellow
            error: Color::from_hex("#d07277"),   // error red
            info: Color::from_hex("#74ade8"),    // info blue

            // Focus
            focus_ring: Color::from_rgba(0.455, 0.678, 0.910, 0.45), // Blue focus ring

            // Editor specific
            // Gutter matches editor; a subtle border supplies the only separation.
            editor_gutter_background: Color::from_hex("#282c33"), // editor.gutter.background
            editor_line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.045), // Current line — visible but subtle
            editor_cursor: Color::from_hex("#74ade8"), // Cursor = blue accent (high visibility)
            editor_selection: Color::from_rgba(0.455, 0.678, 0.910, 0.24), // Blue selection — keeps syntax legible
            editor_find_highlight: Color::from_rgba(0.910, 0.686, 0.455, 0.38), // Amber find — distinct from selection

            // Syntax colors — Zed One Dark mapping.
            syntax_keyword: Color::from_hex("#b477cf"), // Keywords — purple
            syntax_function: Color::from_hex("#73ade9"), // Functions — blue
            syntax_method: Color::from_hex("#73ade9"),  // Methods — blue
            syntax_string: Color::from_hex("#a1c181"),  // Strings — green
            syntax_comment: Color::from_hex("#5d636f"), // Comments — dim slate (One Dark), readable
            syntax_type: Color::from_hex("#6eb4bf"),    // Types — cyan-teal
            syntax_variable: Color::from_hex("#acb2be"), // Variables — foreground (One Dark)
            syntax_constant: Color::from_hex("#dfc184"), // Constants — yellow
            syntax_number: Color::from_hex("#bf956a"),  // Numbers — orange
            syntax_operator: Color::from_hex("#6eb4bf"), // Operators — cyan-teal (visible, not loud)
            syntax_punctuation: Color::from_hex("#acb2be"), // Punctuation — foreground (quieter than hues)
            syntax_attribute: Color::from_hex("#74ade8"),   // Attributes — accent blue
            syntax_tag: Color::from_hex("#74ade8"),         // Tags — accent blue
            syntax_namespace: Color::from_hex("#acb2be"),   // Namespaces — foreground (One Dark)
            syntax_macro: Color::from_hex("#73ade9"),       // Macros — blue (function family)
            syntax_property: Color::from_hex("#d07277"),    // Properties/fields — red (≠ variable)
            syntax_parameter: Color::from_hex("#acb2be"),   // Parameters — foreground
            syntax_builtin: Color::from_hex("#bf956a"),     // Builtins — orange
            syntax_escape: Color::from_hex("#bf956a"),      // Escape sequences — orange
            syntax_embedded: Color::from_hex("#dce0e5"),    // Embedded — bright fg
            syntax_regex: Color::from_hex("#bf956a"),       // Regex — orange
            syntax_markup_heading: Color::from_hex("#d07277"), // Markdown headings — red (title)
            syntax_markup_list: Color::from_hex("#d07277"), // Markdown list markers — red
            syntax_markup_quote: Color::from_hex("#5d636f"), // Markdown quotes — dim slate
            syntax_markup_link: Color::from_hex("#73ade9"), // Markdown link text — blue
            syntax_markup_code: Color::from_hex("#a1c181"), // Markdown code — green
            syntax_markup_bold: Color::from_hex("#bf956a"), // Markdown strong — orange
            syntax_markup_italic: Color::from_hex("#74ade8"), // Markdown emphasis — blue
            syntax_markup_strikethrough: Color::from_hex("#878a98"), // Markdown strikethrough — muted
            syntax_lifetime: Color::from_hex("#6eb4bf"), // Lifetimes — cyan-teal (type family)
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

    /// Light theme semantic colors — Zed "One Light"–faithful.
    ///
    /// The readable light companion ported from Zed's `One Light`: editor
    /// brightest at `#fafafa`, panels/sidebars `#ebebec`, window chrome
    /// `#dcdcdd`, visible borders `#c9c9ca`, and near-black text `#242529` for
    /// high contrast (readable-first, never washed out / pale-on-pale). Blue
    /// accent `#5c78e2`; syntax keeps the One Light reading — magenta keywords,
    /// blue functions, teal types, green strings, amber numbers, red properties.
    pub fn light() -> Self {
        Self {
            // Background surfaces — Zed One Light hierarchy (editor brightest → chrome darkest).
            app_background: Color::from_hex("#fafafa"), // Editor-bright backdrop
            shell_background: Color::from_hex("#ebebec"), // Lifted-down surface (panels)
            panel_background: Color::from_hex("#ebebec"), // surface.background
            elevated_panel_background: Color::from_hex("#ffffff"), // Modals/dropdowns — brightest card
            editor_background: Color::from_hex("#fafafa"), // editor.background — focus surface
            input_background: Color::from_hex("#ffffff"),  // Crisp input/search field
            status_bar_background: Color::from_hex("#dcdcdd"), // status_bar.background — chrome frame
            title_bar_background: Color::from_hex("#dcdcdd"), // title_bar.background — chrome frame
            activity_rail_background: Color::from_hex("#ebebec"), // panel.background
            sidebar_background: Color::from_hex("#ebebec"),   // panel.background
            tab_background: Color::from_hex("#ebebec"),       // tab.inactive_background — recedes
            tab_active_background: Color::from_hex("#fafafa"), // tab.active_background = editor → connected
            assistant_panel_background: Color::from_hex("#ebebec"), // panel.background
            bottom_panel_background: Color::from_hex("#ebebec"), // panel.background
            bottom_panel_header_background: Color::from_hex("#ebebec"), // editor.subheader.background
            assistant_panel_header_background: Color::from_hex("#ebebec"), // flattened — match body

            // Text colors — near-black ink, high contrast (readable-first).
            text_primary: Color::from_hex("#242529"), // text
            text_secondary: Color::from_hex("#3f4045"), // Between text and muted
            text_muted: Color::from_hex("#58585a"),   // text.muted
            text_faint: Color::from_hex("#8a8b90"),   // Readable gray — line numbers, labels
            text_on_accent: Color::from_hex("#ffffff"), // White on blue accent
            text_on_surface: Color::from_hex("#242529"),
            text_disabled: Color::from_hex("#a8a9ad"), // Disabled
            text_link: Color::from_hex("#5c78e2"),     // text.accent — blue

            // UI elements — visible borders, blue accent.
            border: Color::from_hex("#c9c9ca"),         // border
            border_subtle: Color::from_hex("#dfdfe0"),  // border.variant
            divider: Color::from_hex("#dfdfe0"),        // border.variant
            divider_subtle: Color::from_hex("#e6e6e7"), // soft
            panel_header_background: Color::from_hex("#ebebec"), // surface
            nested_surface_background: Color::from_hex("#ebebec"), // element.background
            app_chrome_background: Color::from_hex("#fafafa"), // editor-bright
            tab_strip_background: Color::from_hex("#ebebec"), // tab_bar.background
            accent: Color::from_hex("#5c78e2"),         // Zed light accent — blue
            accent_hover: Color::from_hex("#4a66d0"),   // Deeper blue hover
            accent_soft: Color::from_rgba(0.36, 0.47, 0.89, 0.12), // Soft blue background
            accent_soft_background: Color::from_rgba(0.36, 0.47, 0.89, 0.06), // Very soft blue

            // States — neutral hover/active; selection is the Zed blue wash (preserves syntax).
            hover_background: Color::from_rgba(0.0, 0.0, 0.0, 0.05), // Hover
            active_background: Color::from_rgba(0.0, 0.0, 0.0, 0.09), // Active
            selected_background: Color::from_rgba(0.36, 0.47, 0.89, 0.14), // Selected list rows — blue
            selected_text_background: Color::from_rgba(0.36, 0.47, 0.89, 0.20), // Text selection — blue
            selected_editor_background: Color::from_rgba(0.36, 0.47, 0.89, 0.16), // Editor selection — blue

            // Status colors — Zed One Light semantic set.
            success: Color::from_hex("#669f59"), // created / success green
            warning: Color::from_hex("#a48819"), // warning gold
            error: Color::from_hex("#d36151"),   // error red
            info: Color::from_hex("#5c78e2"),    // info blue

            // Focus
            focus_ring: Color::from_rgba(0.36, 0.47, 0.89, 0.40), // Blue focus ring

            // Editor specific
            editor_gutter_background: Color::from_hex("#fafafa"), // editor.gutter.background
            editor_line_highlight: Color::from_rgba(0.0, 0.0, 0.0, 0.045), // Current line — visible but subtle
            editor_cursor: Color::from_hex("#5c78e2"), // Cursor = blue accent (high visibility)
            editor_selection: Color::from_rgba(0.36, 0.47, 0.89, 0.18), // Blue selection — keeps syntax legible
            editor_find_highlight: Color::from_rgba(0.82, 0.66, 0.14, 0.35), // Amber find — distinct from selection

            // Syntax colors — Zed One Light mapping.
            syntax_keyword: Color::from_hex("#a449ab"), // Keywords — magenta
            syntax_function: Color::from_hex("#5b79e3"), // Functions — blue
            syntax_method: Color::from_hex("#5b79e3"),  // Methods — blue
            syntax_string: Color::from_hex("#649f57"),  // Strings — green
            syntax_comment: Color::from_hex("#8c8d92"), // Comments — readable muted gray
            syntax_type: Color::from_hex("#3882b7"),    // Types — teal-blue
            syntax_variable: Color::from_hex("#242529"), // Variables — foreground (One Light)
            syntax_constant: Color::from_hex("#c18401"), // Constants — gold
            syntax_number: Color::from_hex("#ad6e25"),  // Numbers — amber-brown
            syntax_operator: Color::from_hex("#3882b7"), // Operators — teal-blue
            syntax_punctuation: Color::from_hex("#242529"), // Punctuation — foreground (quieter than hues)
            syntax_attribute: Color::from_hex("#5c78e2"),   // Attributes — blue
            syntax_tag: Color::from_hex("#5c78e2"),         // Tags — blue
            syntax_namespace: Color::from_hex("#242529"),   // Namespaces — foreground (One Light)
            syntax_macro: Color::from_hex("#5b79e3"),       // Macros — blue (function family)
            syntax_property: Color::from_hex("#d3604f"),    // Properties/fields — red (≠ variable)
            syntax_parameter: Color::from_hex("#242529"),   // Parameters — foreground
            syntax_builtin: Color::from_hex("#ad6e25"),     // Builtins — amber-brown
            syntax_escape: Color::from_hex("#7c7e86"),      // Escape sequences — muted
            syntax_embedded: Color::from_hex("#242529"),    // Embedded — foreground
            syntax_regex: Color::from_hex("#ad6e26"),       // Regex — amber-brown
            syntax_markup_heading: Color::from_hex("#d3604f"), // Markdown headings — red (title)
            syntax_markup_list: Color::from_hex("#d3604f"), // Markdown list markers — red
            syntax_markup_quote: Color::from_hex("#8c8d92"), // Markdown quotes — muted gray
            syntax_markup_link: Color::from_hex("#5b79e3"), // Markdown link text — blue
            syntax_markup_code: Color::from_hex("#649f57"), // Markdown code — green
            syntax_markup_bold: Color::from_hex("#ad6e25"), // Markdown strong — amber
            syntax_markup_italic: Color::from_hex("#5c78e2"), // Markdown emphasis — blue
            syntax_markup_strikethrough: Color::from_hex("#7e8086"), // Markdown strikethrough — muted
            syntax_lifetime: Color::from_hex("#3882b7"), // Lifetimes — teal-blue (type family)
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
