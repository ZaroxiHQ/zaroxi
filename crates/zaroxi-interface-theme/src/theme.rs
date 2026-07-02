//! Theme definitions for Zaroxi
//! This module provides zaroxi_theme variants, design tokens, and semantic colors

use crate::colors::Color;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Theme variants for Zaroxi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ZaroxiTheme {
    /// Dark zaroxi_theme
    Dark,
    /// Light zaroxi_theme
    Light,
    /// Use system preference
    #[default]
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
    //
    // Border hierarchy (quietest → loudest):
    //   `divider_subtle` = hairline (internal dividers, scrollbar tracks),
    //   `divider`/`border_subtle` = subtle (major panel seams, tab separators),
    //   `border` = default component outline (cards, inputs, popovers),
    //   `border_strong` = emphasis seam (active divider, drag target, minimap
    //   viewport, find container), `border_focus` = accent focus edge.
    pub border: Color,
    pub border_subtle: Color,
    pub divider: Color,
    pub divider_subtle: Color,
    /// Emphasis seam — active dividers, selected panel edge, minimap viewport,
    /// drag targets. Still elegant, never loud.
    pub border_strong: Color,
    /// Focus edge — focused inputs / active prompt container; accent-aligned.
    pub border_focus: Color,
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
    /// Modified-line cue (amber) — reserved for git-modified gutter/badges.
    pub diff_modified: Color,
    /// Staged cue (indigo) — staged = intentional, uses the Zaroxi signature.
    pub git_staged: Color,

    // AI signal — teal, reserved EXCLUSIVELY for AI features (session pulse,
    // AI cards). Never reused for generic UI so a teal cue always means "AI".
    pub ai_active: Color,
    pub ai_idle: Color,
    pub ai_surface: Color,
    pub ai_border: Color,

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
            // Surface layers — near-black indigo-tinted planes (chrome → panels → editor → float → overlay).
            app_background: Color::from_hex("#07070B"), // bg_root — window chrome (near-black, not #000)
            shell_background: Color::from_hex("#0A0A0F"), // bg_panel — sidebar/tab bar/panels
            panel_background: Color::from_hex("#0A0A0F"), // bg_panel
            elevated_panel_background: Color::from_hex("#1A1A26"), // bg_overlay — tooltip/popover/palette
            editor_background: Color::from_hex("#0D0D13"),         // bg_editor — main editor area
            input_background: Color::from_hex("#13131B"), // bg_float — inputs/cards/float surfaces
            status_bar_background: Color::from_hex("#07070B"), // bg_root — status bar fill
            title_bar_background: Color::from_hex("#0A0A0F"), // bg_panel — title bar
            activity_rail_background: Color::from_hex("#0A0A0F"), // bg_panel
            sidebar_background: Color::from_hex("#0A0A0F"), // bg_panel — file explorer shell
            tab_background: Color::from_hex("#0A0A0F"),   // bg_panel — inactive tab strip
            tab_active_background: Color::from_hex("#0D0D13"), // bg_editor — active tab connects to editor
            assistant_panel_background: Color::from_hex("#0A0A0F"), // bg_panel — AI shell
            bottom_panel_background: Color::from_hex("#0A0A0F"), // bg_panel — terminal/problems
            bottom_panel_header_background: Color::from_hex("#0A0A0F"), // bg_panel — bottom tab strip
            assistant_panel_header_background: Color::from_hex("#0A0A0F"), // bg_panel — AI header

            // Text hierarchy — warm off-white (no pure #FFF → avoids halation at hour 4+).
            text_primary: Color::from_hex("#E8E6F0"), // text_primary — active file/tab text
            text_secondary: Color::from_hex("#B8B6C8"), // text_secondary — body, labels
            text_muted: Color::from_hex("#6B6980"),   // text_muted — line numbers, section headers
            text_faint: Color::from_hex("#45435A"),   // text_disabled — placeholder, invisibles
            text_on_accent: Color::from_hex("#FFFFFF"), // text_on_accent — on filled indigo
            text_on_surface: Color::from_hex("#E8E6F0"), // text_primary
            text_disabled: Color::from_hex("#45435A"), // text_disabled
            text_link: Color::from_hex("#9D7BF0"), // accent — interactive links carry the signature

            // Borders — white-tinted transparency scale so panels separate cleanly on near-black.
            border: Color::from_rgba(1.0, 1.0, 1.0, 0.11), // border.strong — editor↔AI / editor↔bottom seams, panel outlines
            border_subtle: Color::from_rgba(1.0, 1.0, 1.0, 0.07), // border.subtle — tab separators, cards, inputs
            divider: Color::from_rgba(1.0, 1.0, 1.0, 0.07), // border.subtle — explorer↔editor, status, region seams
            divider_subtle: Color::from_rgba(1.0, 1.0, 1.0, 0.04), // border.hairline — soft inner dividers
            border_strong: Color::from_rgba(1.0, 1.0, 1.0, 0.11),  // border.strong — emphasis seams
            border_focus: Color::from_hex("#9D7BF0"), // accent — focus edge (Electric Indigo)
            panel_header_background: Color::from_hex("#0A0A0F"), // bg_panel
            nested_surface_background: Color::from_hex("#13131B"), // bg_float
            app_chrome_background: Color::from_hex("#07070B"), // bg_root — frame
            tab_strip_background: Color::from_hex("#0A0A0F"), // bg_panel — tab strip
            accent: Color::from_hex("#9D7BF0"), // accent — Electric Indigo, the Zaroxi signature
            accent_hover: Color::from_hex("#B89CF7"), // accent_bright — hover on accent element
            accent_soft: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.08), // accent_surface — active item tint
            accent_soft_background: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.06), // accent_line — active line tint

            // States — accent is RARE: hover is a neutral lift; accent is reserved for
            // selection/active/focus so it stays powerful. Strength climbs hover→select→text.
            hover_background: Color::from_rgba(0.9098, 0.902, 0.9412, 0.04), // text_primary @4% — neutral lift
            active_background: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.16), // accent — active/pressed
            selected_background: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.16), // accent — selected row
            selected_text_background: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.22), // accent — text selection (strongest)
            selected_editor_background: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.18), // accent — selected block

            // Status / semantic — functional colors, used nowhere decorative.
            success: Color::from_hex("#3FB950"), // saved/ok/passing
            warning: Color::from_hex("#D29922"), // lint warning
            error: Color::from_hex("#F85149"),   // compile/LSP error
            info: Color::from_hex("#9ECBFF"),    // info/hint
            diff_added: Color::from_hex("#3FB950"), // git added — green
            diff_removed: Color::from_hex("#F85149"), // git deleted — red
            diff_modified: Color::from_hex("#D29922"), // git modified — amber
            git_staged: Color::from_hex("#9D7BF0"), // staged — indigo (intentional)

            // AI signal — teal, AI-only.
            ai_active: Color::from_hex("#3DDBD9"), // live session pulse
            ai_idle: Color::from_hex("#2A9E9C"),   // session idle
            ai_surface: Color::from_rgba(0.2392, 0.8588, 0.851, 0.08), // AI card bg
            ai_border: Color::from_rgba(0.2392, 0.8588, 0.851, 0.25), // AI card border

            // Focus — indigo keyboard-focus ring.
            focus_ring: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.40), // accent_ring

            // Editor specific.
            editor_gutter_background: Color::from_hex("#0A0A0F"), // bg_panel — gutter
            editor_line_highlight: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.06), // accent_line — 6% tracks position without noise
            editor_cursor: Color::from_hex("#9D7BF0"), // accent — indigo cursor
            editor_selection: Color::from_rgba(0.6157, 0.4824, 0.9412, 0.22), // accent — matches text selection
            editor_find_highlight: Color::from_rgba(0.8235, 0.6, 0.1333, 0.22), // git_modified_surface — search match

            // Syntax — teal keywords = Zaroxi signature; high contrast on key tokens, receding punctuation.
            syntax_keyword: Color::from_hex("#3DDBD9"), // syn_keyword — TEAL signature (no other IDE does this)
            syntax_function: Color::from_hex("#9ECBFF"), // syn_function — soft sky blue
            syntax_method: Color::from_hex("#9ECBFF"),  // syn_function
            syntax_string: Color::from_hex("#F0A882"),  // syn_string — warm peach/coral
            syntax_comment: Color::from_hex("#4E6A4E"), // syn_comment — low-contrast, recedes intentionally
            syntax_type: Color::from_hex("#F0C674"),    // syn_type — warm gold
            syntax_variable: Color::from_hex("#C8C6DA"), // syn_variable — warm light gray
            syntax_constant: Color::from_hex("#D7BA7D"), // syn_constant — warm yellow
            syntax_number: Color::from_hex("#B5CEA8"),  // syn_number — sage green
            syntax_operator: Color::from_hex("#8B9EC9"), // syn_operator — muted periwinkle
            syntax_punctuation: Color::from_hex("#5A586E"), // syn_punct — very dim, glyphs recede
            syntax_attribute: Color::from_hex("#C586C0"), // syn_attribute — soft magenta
            syntax_tag: Color::from_hex("#3DDBD9"),     // syn_keyword — markup structure teal
            syntax_namespace: Color::from_hex("#A8A6C0"), // syn_module — near-neutral cool
            syntax_macro: Color::from_hex("#C586C0"),   // syn_macro — soft magenta (special syntax)
            syntax_property: Color::from_hex("#C8C6DA"), // syn_property — named slots, like variables
            syntax_parameter: Color::from_hex("#C8C6DA"), // syn_parameter
            syntax_builtin: Color::from_hex("#9ECBFF"),  // syn_function — builtin function
            syntax_escape: Color::from_hex("#D7BA7D"), // syn_escape — special value, like constant
            syntax_embedded: Color::from_hex("#3DDBD9"), // syn_interpolation — teal "active" slots
            syntax_regex: Color::from_hex("#F0A882"),  // syn_regex — like string
            syntax_markup_heading: Color::from_hex("#3DDBD9"), // syn_keyword — teal headings
            syntax_markup_list: Color::from_hex("#8B9EC9"), // syn_operator — dim list markers
            syntax_markup_quote: Color::from_hex("#4E6A4E"), // syn_comment — dim blockquote
            syntax_markup_link: Color::from_hex("#9ECBFF"), // syn_function — blue links
            syntax_markup_code: Color::from_hex("#F0A882"), // syn_string — peach inline code
            syntax_markup_bold: Color::from_hex("#D7BA7D"), // syn_constant — warm emphasis
            syntax_markup_italic: Color::from_hex("#C8C6DA"), // syn_variable — neutral italic
            syntax_markup_strikethrough: Color::from_hex("#6B6980"), // text_muted
            syntax_lifetime: Color::from_hex("#D4A0A0"), // syn_lifetime — dusty rose, spotable
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
            // Surface layers — warm off-white (reduces blue-light fatigue), editor warmest for reading.
            app_background: Color::from_hex("#F5F4F8"), // bg_root — window chrome
            shell_background: Color::from_hex("#EEEDF3"), // bg_panel — sidebar/panels
            panel_background: Color::from_hex("#EEEDF3"), // bg_panel
            elevated_panel_background: Color::from_hex("#DDDBE8"), // bg_overlay — tooltip/popover
            editor_background: Color::from_hex("#FAFAFA"), // bg_editor — warmest, easiest to read
            input_background: Color::from_hex("#E8E6F0"), // bg_float — inputs/cards
            status_bar_background: Color::from_hex("#F5F4F8"), // bg_root — status bar fill
            title_bar_background: Color::from_hex("#EEEDF3"), // bg_panel — title bar
            activity_rail_background: Color::from_hex("#EEEDF3"), // bg_panel
            sidebar_background: Color::from_hex("#EEEDF3"), // bg_panel — file explorer shell
            tab_background: Color::from_hex("#EEEDF3"), // bg_panel — inactive tab strip
            tab_active_background: Color::from_hex("#FAFAFA"), // bg_editor — active tab connects to editor
            assistant_panel_background: Color::from_hex("#EEEDF3"), // bg_panel — AI shell
            bottom_panel_background: Color::from_hex("#EEEDF3"), // bg_panel — terminal/problems
            bottom_panel_header_background: Color::from_hex("#EEEDF3"), // bg_panel — bottom tab strip
            assistant_panel_header_background: Color::from_hex("#EEEDF3"), // bg_panel — AI header

            // Text hierarchy — near-black with purple tint, high-contrast readable-first.
            text_primary: Color::from_hex("#1A1830"), // text_primary — active file/tab text
            text_secondary: Color::from_hex("#3D3A54"), // text_secondary — body, labels
            text_muted: Color::from_hex("#6B6880"),   // text_muted — line numbers, section headers
            text_faint: Color::from_hex("#A8A6BE"),   // text_disabled — placeholder, invisibles
            text_on_accent: Color::from_hex("#FFFFFF"), // text_on_accent — on filled indigo
            text_on_surface: Color::from_hex("#1A1830"), // text_primary
            text_disabled: Color::from_hex("#A8A6BE"), // text_disabled
            text_link: Color::from_hex("#6E40C9"), // accent — interactive links carry the signature

            // Borders — ink-tinted transparency scale, soft and architectural on light surfaces.
            border: Color::from_rgba(0.0667, 0.0941, 0.1529, 0.13), // border.strong — editor↔AI / editor↔bottom seams
            border_subtle: Color::from_rgba(0.0667, 0.0941, 0.1529, 0.08), // border.subtle — tab separators, cards, inputs
            divider: Color::from_rgba(0.0667, 0.0941, 0.1529, 0.08), // border.subtle — explorer↔editor, status, region seams
            divider_subtle: Color::from_rgba(0.0667, 0.0941, 0.1529, 0.05), // border.hairline — soft inner dividers
            border_strong: Color::from_rgba(0.0667, 0.0941, 0.1529, 0.13), // border.strong — emphasis seams
            border_focus: Color::from_hex("#6E40C9"), // accent — focus edge (brand purple)
            panel_header_background: Color::from_hex("#EEEDF3"), // bg_panel
            nested_surface_background: Color::from_hex("#E8E6F0"), // bg_float
            app_chrome_background: Color::from_hex("#F5F4F8"), // bg_root — frame
            tab_strip_background: Color::from_hex("#EEEDF3"), // bg_panel — tab strip
            accent: Color::from_hex("#6E40C9"), // accent — deeper Electric Indigo for light-bg contrast
            accent_hover: Color::from_hex("#9D7BF0"), // accent_bright — hover
            accent_soft: Color::from_rgba(0.4314, 0.251, 0.7882, 0.08), // accent_surface
            accent_soft_background: Color::from_rgba(0.4314, 0.251, 0.7882, 0.05), // accent_line

            // States — accent is RARE: hover is a neutral lift; accent is reserved for
            // selection/active/focus so it stays powerful. Strength climbs hover→select→text.
            hover_background: Color::from_rgba(0.102, 0.0941, 0.1882, 0.05), // text_primary @5% — neutral lift
            active_background: Color::from_rgba(0.4314, 0.251, 0.7882, 0.13), // accent — active/pressed
            selected_background: Color::from_rgba(0.4314, 0.251, 0.7882, 0.13), // accent — selected row
            selected_text_background: Color::from_rgba(0.4314, 0.251, 0.7882, 0.18), // accent — text selection (strongest)
            selected_editor_background: Color::from_rgba(0.4314, 0.251, 0.7882, 0.16), // accent — selected block

            // Status / semantic — deeper for readability on light.
            success: Color::from_hex("#1A7F37"), // saved/ok/passing
            warning: Color::from_hex("#9A6700"), // lint warning
            error: Color::from_hex("#CF222E"),   // compile/LSP error
            info: Color::from_hex("#0550AE"),    // info/hint
            diff_added: Color::from_hex("#1A7F37"), // git added — green
            diff_removed: Color::from_hex("#CF222E"), // git deleted — red
            diff_modified: Color::from_hex("#9A6700"), // git modified — amber
            git_staged: Color::from_hex("#6E40C9"), // staged — indigo (intentional)

            // AI signal — teal, AI-only.
            ai_active: Color::from_hex("#0E8B89"), // live session pulse
            ai_idle: Color::from_hex("#1A6B69"),   // session idle
            ai_surface: Color::from_rgba(0.0549, 0.5451, 0.5373, 0.08), // AI card bg
            ai_border: Color::from_rgba(0.0549, 0.5451, 0.5373, 0.25), // AI card border

            // Focus — indigo keyboard-focus ring.
            focus_ring: Color::from_rgba(0.4314, 0.251, 0.7882, 0.35), // accent_ring

            // Editor specific.
            editor_gutter_background: Color::from_hex("#EEEDF3"), // bg_panel — gutter
            editor_line_highlight: Color::from_rgba(0.4314, 0.251, 0.7882, 0.05), // accent_line
            editor_cursor: Color::from_hex("#6E40C9"),            // accent — indigo cursor
            editor_selection: Color::from_rgba(0.4314, 0.251, 0.7882, 0.18), // accent — matches text selection
            editor_find_highlight: Color::from_rgba(0.6039, 0.4039, 0.0, 0.22), // git_modified_surface — search match

            // Syntax — deeper teal keywords keep the Zaroxi signature; darkened so nothing washes out.
            syntax_keyword: Color::from_hex("#0E8B89"), // syn_keyword — deep teal signature
            syntax_function: Color::from_hex("#0550AE"), // syn_function — deep blue
            syntax_method: Color::from_hex("#0550AE"),  // syn_function
            syntax_string: Color::from_hex("#953800"),  // syn_string — terracotta/coral
            syntax_comment: Color::from_hex("#8B9EB0"), // syn_comment — blue-gray, recedes
            syntax_type: Color::from_hex("#8B5000"),    // syn_type — deep amber/brown
            syntax_variable: Color::from_hex("#1A1830"), // syn_variable — near-ink
            syntax_constant: Color::from_hex("#8B5000"), // syn_constant — deep amber
            syntax_number: Color::from_hex("#116329"),  // syn_number — deep green
            syntax_operator: Color::from_hex("#4A4580"), // syn_operator — muted indigo-gray
            syntax_punctuation: Color::from_hex("#8B88A8"), // syn_punct — dim, recedes
            syntax_attribute: Color::from_hex("#8250DF"), // syn_attribute — purple metadata
            syntax_tag: Color::from_hex("#0E8B89"),     // syn_keyword — markup structure teal
            syntax_namespace: Color::from_hex("#3D3A54"), // syn_module — near-neutral
            syntax_macro: Color::from_hex("#8250DF"),   // syn_macro — purple (special syntax)
            syntax_property: Color::from_hex("#1A1830"), // syn_property — like variable
            syntax_parameter: Color::from_hex("#1A1830"), // syn_parameter
            syntax_builtin: Color::from_hex("#0550AE"), // syn_function — builtin function
            syntax_escape: Color::from_hex("#8B5000"),  // syn_escape — special value, like constant
            syntax_embedded: Color::from_hex("#0E8B89"), // syn_interpolation — teal "active" slots
            syntax_regex: Color::from_hex("#953800"),   // syn_regex — like string
            syntax_markup_heading: Color::from_hex("#0E8B89"), // syn_keyword — teal headings
            syntax_markup_list: Color::from_hex("#4A4580"), // syn_operator — dim list markers
            syntax_markup_quote: Color::from_hex("#8B9EB0"), // syn_comment — dim blockquote
            syntax_markup_link: Color::from_hex("#0550AE"), // syn_function — blue links
            syntax_markup_code: Color::from_hex("#953800"), // syn_string — inline code
            syntax_markup_bold: Color::from_hex("#8B5000"), // syn_constant — warm emphasis
            syntax_markup_italic: Color::from_hex("#1A1830"), // syn_variable — neutral italic
            syntax_markup_strikethrough: Color::from_hex("#6B6880"), // text_muted
            syntax_lifetime: Color::from_hex("#953800"), // syn_lifetime — terracotta, spotable
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
            "--color-border-strong".to_string(),
            Value::String(self.border_strong.to_css_rgba()),
        );
        m.insert(
            "--color-border-focus".to_string(),
            Value::String(self.border_focus.to_css_rgba()),
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

        // Git / diff cues
        m.insert("--color-diff-added".to_string(), Value::String(self.diff_added.to_css_rgba()));
        m.insert(
            "--color-diff-removed".to_string(),
            Value::String(self.diff_removed.to_css_rgba()),
        );
        m.insert(
            "--color-diff-modified".to_string(),
            Value::String(self.diff_modified.to_css_rgba()),
        );
        m.insert("--color-git-staged".to_string(), Value::String(self.git_staged.to_css_rgba()));

        // AI signal (teal — AI features only)
        m.insert("--color-ai-active".to_string(), Value::String(self.ai_active.to_css_rgba()));
        m.insert("--color-ai-idle".to_string(), Value::String(self.ai_idle.to_css_rgba()));
        m.insert("--color-ai-surface".to_string(), Value::String(self.ai_surface.to_css_rgba()));
        m.insert("--color-ai-border".to_string(), Value::String(self.ai_border.to_css_rgba()));

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
