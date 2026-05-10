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
}

impl SemanticColors {
    /// Dark theme semantic colors - Professional IDE with cool-neutral dark tones
    /// Designed for long coding sessions with clear surface hierarchy and restrained blue accent
    pub fn dark() -> Self {
        Self {
            // Background surfaces - clear hierarchy from deepest shell to brightest editor
            app_background: Color::from_hex("#1B1D22"), // Deepest shell
            shell_background: Color::from_hex("#1E2025"), // Slightly lighter shell
            panel_background: Color::from_hex("#252931"), // Side panels
            elevated_panel_background: Color::from_hex("#2A2E37"), // Elevated panels (modals, dropdowns)
            editor_background: Color::from_hex("#1E1F24"), // Editor - slightly lighter than shell
            input_background: Color::from_hex("#2A2E37"),  // Input fields
            status_bar_background: Color::from_hex("#23262D"), // Status bar - distinct from panels
            title_bar_background: Color::from_hex("#23262D"), // Title bar matches status bar
            activity_rail_background: Color::from_hex("#20232A"), // Activity rail - own distinct role
            sidebar_background: Color::from_hex("#252931"),       // Sidebar matches panels
            tab_background: Color::from_hex("#252830"),           // Inactive tabs
            tab_active_background: Color::from_hex("#1E1F24"),    // Active tab matches editor
            assistant_panel_background: Color::from_hex("#262A32"), // Right utility panel

            // Text colors - hierarchy from most prominent to subtle
            text_primary: Color::from_hex("#E6EAF2"), // Primary text - bright but not harsh
            text_secondary: Color::from_hex("#C8CDD6"), // Secondary text
            text_muted: Color::from_hex("#AAB2BF"),   // Muted text - still readable
            text_faint: Color::from_hex("#7E8794"),   // Faint text - labels, line numbers
            text_on_accent: Color::from_hex("#FFFFFF"), // Text on accent backgrounds
            text_on_surface: Color::from_hex("#E6EAF2"), // Text on surfaces
            text_disabled: Color::from_hex("#5A6270"), // Disabled text
            text_link: Color::from_hex("#5B8CFF"),    // Link blue matches accent

            // UI elements - restrained borders and dividers
            border: Color::from_hex("#343944"), // Borders - visible but not harsh
            border_subtle: Color::from_rgba(0.20, 0.22, 0.27, 0.5), // Subtle borders
            divider: Color::from_hex("#343944"), // Dividers match borders
            divider_subtle: Color::from_rgba(0.20, 0.22, 0.27, 0.3), // Very soft divider, less visible
            panel_header_background: Color::from_hex("#282C35"), // Slightly lighter than panel background
            nested_surface_background: Color::from_hex("#2D313A"), // For cards, input bars
            app_chrome_background: Color::from_hex("#1B1D22"),   // Matches app_background (shell)
            tab_strip_background: Color::from_hex("#20232A"), // Same as activity-rail for consistency
            accent: Color::from_hex("#5B8CFF"),               // Restrained blue accent
            accent_hover: Color::from_hex("#6B9CFF"),         // Hover accent - slightly brighter
            accent_soft: Color::from_rgba(0.36, 0.55, 1.0, 0.15), // Soft accent background
            accent_soft_background: Color::from_rgba(0.36, 0.55, 1.0, 0.08), // Very soft accent

            // States - subtle but clear
            hover_background: Color::from_rgba(1.0, 1.0, 1.0, 0.06), // Hover states
            active_background: Color::from_rgba(1.0, 1.0, 1.0, 0.10), // Active states
            selected_background: Color::from_rgba(0.36, 0.55, 1.0, 0.18), // Selected with accent
            selected_text_background: Color::from_rgba(0.36, 0.55, 1.0, 0.22), // Text selection
            selected_editor_background: Color::from_rgba(0.36, 0.55, 1.0, 0.18), // Editor selection

            // Status colors - clear but not distracting
            success: Color::from_hex("#4CAF50"), // Success green
            warning: Color::from_hex("#FF9800"), // Warning orange
            error: Color::from_hex("#F44336"),   // Error red
            info: Color::from_hex("#5B8CFF"),    // Info blue matches accent

            // Focus
            focus_ring: Color::from_rgba(0.36, 0.55, 1.0, 0.30), // Focus ring

            // Editor specific
            // Make gutter background match the editor background so the gutter visually
            // blends with the code surface (requested UX refinement).
            editor_gutter_background: Color::from_hex("#1E1F24"), // Use editor_background
            editor_line_highlight: Color::from_rgba(1.0, 1.0, 1.0, 0.03), // Line highlight
            editor_cursor: Color::from_hex("#E6EAF2"),            // Cursor matches primary text
            editor_selection: Color::from_rgba(0.36, 0.55, 1.0, 0.22), // Editor selection
            editor_find_highlight: Color::from_rgba(1.0, 0.60, 0.0, 0.25), // Find highlight

            // Syntax colors - clear, readable, professional
            syntax_keyword: Color::from_hex("#FF6B6B"), // Keywords - soft red
            syntax_function: Color::from_hex("#4CAF50"), // Functions - green
            syntax_method: Color::from_hex("#4CAF50"),   // Methods - same as functions
            syntax_string: Color::from_hex("#FFB74D"),  // Strings - warm orange
            syntax_comment: Color::from_hex("#7E8794"), // Comments - faint gray
            syntax_type: Color::from_hex("#5B8CFF"),    // Types - accent blue
            syntax_variable: Color::from_hex("#E6EAF2"), // Variables - primary text
            syntax_constant: Color::from_hex("#FFB74D"), // Constants - warm orange
            syntax_number: Color::from_hex("#B39DDB"),  // Numbers - soft purple
            syntax_operator: Color::from_hex("#C792EA"), // Operators - soft violet
            syntax_punctuation: Color::from_hex("#AAB2BF"), // Punctuation - muted gray
            syntax_attribute: Color::from_hex("#F78C6C"), // Attributes - soft orange
            syntax_tag: Color::from_hex("#F07178"),     // Tags - soft coral
            syntax_namespace: Color::from_hex("#82AAFF"), // Namespaces - light blue
            syntax_macro: Color::from_hex("#C792EA"),   // Macros - soft violet
            syntax_property: Color::from_hex("#FFCB6B"), // Properties - warm yellow
            syntax_parameter: Color::from_hex("#F78C6C"), // Parameters - soft orange
            syntax_builtin: Color::from_hex("#FF5370"),  // Builtins - coral red
            syntax_escape: Color::from_hex("#89DDFF"),   // Escape sequences - cyan
            syntax_embedded: Color::from_hex("#C3E88D"), // Embedded languages - light green
            syntax_regex: Color::from_hex("#89DDFF"),    // Regex - cyan
            syntax_markup_heading: Color::from_hex("#FF5370"), // Markdown headings - coral
            syntax_markup_list: Color::from_hex("#89DDFF"), // Markdown lists - cyan
            syntax_markup_quote: Color::from_hex("#7E8794"), // Markdown quotes - faint gray
            syntax_markup_link: Color::from_hex("#82AAFF"), // Markdown links - light blue
            syntax_markup_code: Color::from_hex("#C3E88D"), // Markdown code - light green
            syntax_markup_bold: Color::from_hex("#E6EAF2"), // Markdown bold - primary text
            syntax_markup_italic: Color::from_hex("#E6EAF2"), // Markdown italic - primary text
            syntax_markup_strikethrough: Color::from_hex("#7E8794"), // Markdown strikethrough - faint
        }
    }

    /// Light theme semantic colors - Professional IDE with warm-neutral light tones
    /// Designed for long coding sessions with clear surface hierarchy and restrained blue accent
    pub fn light() -> Self {
        Self {
            // Background surfaces - clear hierarchy from deepest shell to brightest editor
            app_background: Color::from_hex("#F4F3EF"), // Warm shell background
            shell_background: Color::from_hex("#F0EFEA"), // Slightly deeper shell
            panel_background: Color::from_hex("#F0EEE8"), // Side panels - warm neutral
            elevated_panel_background: Color::from_hex("#F8F6F2"), // Elevated panels (modals, dropdowns)
            editor_background: Color::from_hex("#FBFAF7"),         // Editor - warm white
            input_background: Color::from_hex("#FFFFFF"),          // Input fields - pure white
            status_bar_background: Color::from_hex("#ECE9E3"), // Status bar - distinct from panels
            title_bar_background: Color::from_hex("#ECE9E3"),  // Title bar matches status bar
            activity_rail_background: Color::from_hex("#E7E4DD"), // Activity rail - own distinct role
            sidebar_background: Color::from_hex("#F0EEE8"),       // Sidebar matches panels
            tab_background: Color::from_hex("#F1EEE8"),           // Inactive tabs
            tab_active_background: Color::from_hex("#FBFAF7"),    // Active tab matches editor
            assistant_panel_background: Color::from_hex("#F2F0EA"), // Right utility panel

            // Text colors - hierarchy from most prominent to subtle
            text_primary: Color::from_hex("#22262B"), // Primary text - dark but not black
            text_secondary: Color::from_hex("#3D434A"), // Secondary text
            text_muted: Color::from_hex("#616975"),   // Muted text - still readable
            text_faint: Color::from_hex("#8A919D"),   // Faint text - labels, line numbers
            text_on_accent: Color::from_hex("#FFFFFF"), // Text on accent backgrounds
            text_on_surface: Color::from_hex("#22262B"), // Text on surfaces
            text_disabled: Color::from_hex("#B0B6C0"), // Disabled text
            text_link: Color::from_hex("#426EDB"),    // Link blue matches accent

            // UI elements - restrained borders and dividers
            border: Color::from_hex("#D7D1C7"), // Borders - warm neutral
            border_subtle: Color::from_rgba(0.84, 0.82, 0.78, 0.5), // Subtle borders
            divider: Color::from_hex("#D7D1C7"), // Dividers match borders
            divider_subtle: Color::from_rgba(0.84, 0.82, 0.78, 0.4), // Very soft divider
            panel_header_background: Color::from_hex("#E8E5DE"), // Slightly lighter than panel background
            nested_surface_background: Color::from_hex("#F0EEE8"), // For cards
            app_chrome_background: Color::from_hex("#F4F3EF"),   // Matches app_background (shell)
            tab_strip_background: Color::from_hex("#E7E4DD"),    // Matches activity-rail
            accent: Color::from_hex("#426EDB"),                  // Restrained blue accent
            accent_hover: Color::from_hex("#3A62C8"),            // Hover accent - slightly darker
            accent_soft: Color::from_rgba(0.26, 0.43, 0.86, 0.10), // Soft accent background
            accent_soft_background: Color::from_rgba(0.26, 0.43, 0.86, 0.05), // Very soft accent

            // States - subtle but clear
            hover_background: Color::from_rgba(0.0, 0.0, 0.0, 0.04), // Hover states
            active_background: Color::from_rgba(0.0, 0.0, 0.0, 0.08), // Active states
            selected_background: Color::from_rgba(0.26, 0.43, 0.86, 0.08), // Selected with accent
            selected_text_background: Color::from_rgba(0.26, 0.43, 0.86, 0.14), // Text selection
            selected_editor_background: Color::from_rgba(0.26, 0.43, 0.86, 0.08), // Editor selection

            // Status colors - clear but not distracting
            success: Color::from_hex("#2E7D32"), // Success green
            warning: Color::from_hex("#E65100"), // Warning orange
            error: Color::from_hex("#C62828"),   // Error red
            info: Color::from_hex("#426EDB"),    // Info blue matches accent

            // Focus
            focus_ring: Color::from_rgba(0.26, 0.43, 0.86, 0.25), // Focus ring

            // Editor specific
            // Make gutter background match the editor background for visual consistency.
            editor_gutter_background: Color::from_hex("#FBFAF7"), // Use editor_background
            editor_line_highlight: Color::from_rgba(0.26, 0.43, 0.86, 0.03), // Line highlight
            editor_cursor: Color::from_hex("#22262B"),            // Cursor matches primary text
            editor_selection: Color::from_rgba(0.26, 0.43, 0.86, 0.14), // Editor selection
            editor_find_highlight: Color::from_rgba(0.90, 0.40, 0.0, 0.18), // Find highlight

            // Syntax colors - clear, readable, professional
            syntax_keyword: Color::from_hex("#D32F2F"), // Keywords - red
            syntax_function: Color::from_hex("#2E7D32"), // Functions - green
            syntax_method: Color::from_hex("#2E7D32"),   // Methods - same as functions
            syntax_string: Color::from_hex("#E65100"),  // Strings - warm orange
            syntax_comment: Color::from_hex("#8A919D"), // Comments - faint gray
            syntax_type: Color::from_hex("#426EDB"),    // Types - accent blue
            syntax_variable: Color::from_hex("#22262B"), // Variables - primary text
            syntax_constant: Color::from_hex("#E65100"), // Constants - warm orange
            syntax_number: Color::from_hex("#7B1FA2"),  // Numbers - purple
            syntax_operator: Color::from_hex("#6A1B9A"), // Operators - violet
            syntax_punctuation: Color::from_hex("#616975"), // Punctuation - muted gray
            syntax_attribute: Color::from_hex("#E65100"), // Attributes - warm orange
            syntax_tag: Color::from_hex("#C62828"),     // Tags - red
            syntax_namespace: Color::from_hex("#1565C0"), // Namespaces - blue
            syntax_macro: Color::from_hex("#6A1B9A"),   // Macros - violet
            syntax_property: Color::from_hex("#F57F17"), // Properties - amber
            syntax_parameter: Color::from_hex("#E65100"), // Parameters - warm orange
            syntax_builtin: Color::from_hex("#C62828"),  // Builtins - red
            syntax_escape: Color::from_hex("#00838F"),   // Escape sequences - teal
            syntax_embedded: Color::from_hex("#2E7D32"), // Embedded languages - green
            syntax_regex: Color::from_hex("#00838F"),    // Regex - teal
            syntax_markup_heading: Color::from_hex("#C62828"), // Markdown headings - red
            syntax_markup_list: Color::from_hex("#00838F"), // Markdown lists - teal
            syntax_markup_quote: Color::from_hex("#8A919D"), // Markdown quotes - faint gray
            syntax_markup_link: Color::from_hex("#1565C0"), // Markdown links - blue
            syntax_markup_code: Color::from_hex("#2E7D32"), // Markdown code - green
            syntax_markup_bold: Color::from_hex("#22262B"), // Markdown bold - primary text
            syntax_markup_italic: Color::from_hex("#22262B"), // Markdown italic - primary text
            syntax_markup_strikethrough: Color::from_hex("#8A919D"), // Markdown strikethrough - faint
        }
    }

    /// Serialize semantic colors into CSS variable map that the frontend consumes.
    ///
    /// Returns a JSON object mapping CSS custom property name -> string color value.
    /// Example key: "--color-editor-background" -> "rgba(30,31,36,1)"
    pub fn to_css_vars(&self) -> Value {
        let mut m: Map<String, Value> = Map::new();

        // Background surfaces
        m.insert("--color-app-background".to_string(), Value::String(self.app_background.to_css_rgba()));
        m.insert("--color-shell-background".to_string(), Value::String(self.shell_background.to_css_rgba()));
        m.insert("--color-panel-background".to_string(), Value::String(self.panel_background.to_css_rgba()));
        m.insert("--color-elevated-panel-background".to_string(), Value::String(self.elevated_panel_background.to_css_rgba()));
        m.insert("--color-editor-background".to_string(), Value::String(self.editor_background.to_css_rgba()));
        m.insert("--color-input-background".to_string(), Value::String(self.input_background.to_css_rgba()));
        m.insert("--color-status-bar-background".to_string(), Value::String(self.status_bar_background.to_css_rgba()));
        m.insert("--color-title-bar-background".to_string(), Value::String(self.title_bar_background.to_css_rgba()));
        m.insert("--color-activity-rail-background".to_string(), Value::String(self.activity_rail_background.to_css_rgba()));
        m.insert("--color-sidebar-background".to_string(), Value::String(self.sidebar_background.to_css_rgba()));
        m.insert("--color-tab-background".to_string(), Value::String(self.tab_background.to_css_rgba()));
        m.insert("--color-tab-active-background".to_string(), Value::String(self.tab_active_background.to_css_rgba()));
        m.insert("--color-assistant-panel-background".to_string(), Value::String(self.assistant_panel_background.to_css_rgba()));

        // Text colors
        m.insert("--color-text-primary".to_string(), Value::String(self.text_primary.to_css_rgba()));
        m.insert("--color-text-secondary".to_string(), Value::String(self.text_secondary.to_css_rgba()));
        m.insert("--color-text-muted".to_string(), Value::String(self.text_muted.to_css_rgba()));
        m.insert("--color-text-faint".to_string(), Value::String(self.text_faint.to_css_rgba()));
        m.insert("--color-text-on-accent".to_string(), Value::String(self.text_on_accent.to_css_rgba()));
        m.insert("--color-text-on-surface".to_string(), Value::String(self.text_on_surface.to_css_rgba()));
        m.insert("--color-text-disabled".to_string(), Value::String(self.text_disabled.to_css_rgba()));
        m.insert("--color-text-link".to_string(), Value::String(self.text_link.to_css_rgba()));

        // UI elements
        m.insert("--color-border".to_string(), Value::String(self.border.to_css_rgba()));
        m.insert("--color-border-subtle".to_string(), Value::String(self.border_subtle.to_css_rgba()));
        m.insert("--color-divider".to_string(), Value::String(self.divider.to_css_rgba()));
        m.insert("--color-divider-subtle".to_string(), Value::String(self.divider_subtle.to_css_rgba()));
        m.insert("--color-panel-header-background".to_string(), Value::String(self.panel_header_background.to_css_rgba()));
        m.insert("--color-nested-surface-background".to_string(), Value::String(self.nested_surface_background.to_css_rgba()));
        m.insert("--color-app-chrome-background".to_string(), Value::String(self.app_chrome_background.to_css_rgba()));
        m.insert("--color-tab-strip-background".to_string(), Value::String(self.tab_strip_background.to_css_rgba()));
        m.insert("--color-accent".to_string(), Value::String(self.accent.to_css_rgba()));
        m.insert("--color-accent-hover".to_string(), Value::String(self.accent_hover.to_css_rgba()));
        m.insert("--color-accent-soft".to_string(), Value::String(self.accent_soft.to_css_rgba()));
        m.insert("--color-accent-soft-background".to_string(), Value::String(self.accent_soft_background.to_css_rgba()));

        // States
        m.insert("--color-hover-background".to_string(), Value::String(self.hover_background.to_css_rgba()));
        m.insert("--color-active-background".to_string(), Value::String(self.active_background.to_css_rgba()));
        m.insert("--color-selected-background".to_string(), Value::String(self.selected_background.to_css_rgba()));
        m.insert("--color-selected-text-background".to_string(), Value::String(self.selected_text_background.to_css_rgba()));
        m.insert("--color-selected-editor-background".to_string(), Value::String(self.selected_editor_background.to_css_rgba()));

        // Status colors
        m.insert("--color-success".to_string(), Value::String(self.success.to_css_rgba()));
        m.insert("--color-warning".to_string(), Value::String(self.warning.to_css_rgba()));
        m.insert("--color-error".to_string(), Value::String(self.error.to_css_rgba()));
        m.insert("--color-info".to_string(), Value::String(self.info.to_css_rgba()));

        // Focus
        m.insert("--color-focus-ring".to_string(), Value::String(self.focus_ring.to_css_rgba()));

        // Editor specific
        m.insert("--color-editor-gutter-background".to_string(), Value::String(self.editor_gutter_background.to_css_rgba()));
        m.insert("--color-editor-line-highlight".to_string(), Value::String(self.editor_line_highlight.to_css_rgba()));
        m.insert("--color-editor-cursor".to_string(), Value::String(self.editor_cursor.to_css_rgba()));
        m.insert("--color-editor-selection".to_string(), Value::String(self.editor_selection.to_css_rgba()));
        m.insert("--color-editor-find-highlight".to_string(), Value::String(self.editor_find_highlight.to_css_rgba()));

        // Syntax colors
        m.insert("--color-syntax-keyword".to_string(), Value::String(self.syntax_keyword.to_css_rgba()));
        m.insert("--color-syntax-function".to_string(), Value::String(self.syntax_function.to_css_rgba()));
        m.insert("--color-syntax-method".to_string(), Value::String(self.syntax_method.to_css_rgba()));
        m.insert("--color-syntax-string".to_string(), Value::String(self.syntax_string.to_css_rgba()));
        m.insert("--color-syntax-comment".to_string(), Value::String(self.syntax_comment.to_css_rgba()));
        m.insert("--color-syntax-type".to_string(), Value::String(self.syntax_type.to_css_rgba()));
        m.insert("--color-syntax-variable".to_string(), Value::String(self.syntax_variable.to_css_rgba()));
        m.insert("--color-syntax-constant".to_string(), Value::String(self.syntax_constant.to_css_rgba()));
        m.insert("--color-syntax-number".to_string(), Value::String(self.syntax_number.to_css_rgba()));
        m.insert("--color-syntax-operator".to_string(), Value::String(self.syntax_operator.to_css_rgba()));
        m.insert("--color-syntax-punctuation".to_string(), Value::String(self.syntax_punctuation.to_css_rgba()));
        m.insert("--color-syntax-attribute".to_string(), Value::String(self.syntax_attribute.to_css_rgba()));
        m.insert("--color-syntax-tag".to_string(), Value::String(self.syntax_tag.to_css_rgba()));
        m.insert("--color-syntax-namespace".to_string(), Value::String(self.syntax_namespace.to_css_rgba()));
        m.insert("--color-syntax-macro".to_string(), Value::String(self.syntax_macro.to_css_rgba()));
        m.insert("--color-syntax-property".to_string(), Value::String(self.syntax_property.to_css_rgba()));
        m.insert("--color-syntax-parameter".to_string(), Value::String(self.syntax_parameter.to_css_rgba()));
        m.insert("--color-syntax-builtin".to_string(), Value::String(self.syntax_builtin.to_css_rgba()));
        m.insert("--color-syntax-escape".to_string(), Value::String(self.syntax_escape.to_css_rgba()));
        m.insert("--color-syntax-embedded".to_string(), Value::String(self.syntax_embedded.to_css_rgba()));
        m.insert("--color-syntax-regex".to_string(), Value::String(self.syntax_regex.to_css_rgba()));
        m.insert("--color-syntax-markup-heading".to_string(), Value::String(self.syntax_markup_heading.to_css_rgba()));
        m.insert("--color-syntax-markup-list".to_string(), Value::String(self.syntax_markup_list.to_css_rgba()));
        m.insert("--color-syntax-markup-quote".to_string(), Value::String(self.syntax_markup_quote.to_css_rgba()));
        m.insert("--color-syntax-markup-link".to_string(), Value::String(self.syntax_markup_link.to_css_rgba()));
        m.insert("--color-syntax-markup-code".to_string(), Value::String(self.syntax_markup_code.to_css_rgba()));
        m.insert("--color-syntax-markup-bold".to_string(), Value::String(self.syntax_markup_bold.to_css_rgba()));
        m.insert("--color-syntax-markup-italic".to_string(), Value::String(self.syntax_markup_italic.to_css_rgba()));
        m.insert("--color-syntax-markup-strikethrough".to_string(), Value::String(self.syntax_markup_strikethrough.to_css_rgba()));

        Value::Object(m)
    }
}
