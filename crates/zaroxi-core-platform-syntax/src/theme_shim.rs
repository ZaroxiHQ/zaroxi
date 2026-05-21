#![allow(dead_code)]
//! Minimal local theme types used by core-platform-syntax to avoid depending on the interface theme crate.
//!
//! This shim defines small, stable data types (Color, SemanticColors) that
//! mirror the fields read by theme_map.rs. Keeping these types local preserves
//! the architecture (core crates must not depend on interface crates) while
//! allowing the syntax crate to perform theme mapping without pulling the
//! full interface-theme crate into its dependency graph.

/// Simple RGBA color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Construct from components.
    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Convenience constructor when alpha is 1.0.
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

/// Semantic color roles required by theme_map.
///
/// This struct intentionally exposes only simple Color fields and is meant
/// to be a minimal, copyable DTO that mirrors the values read by the
/// syntax/theme mapping logic.
#[derive(Debug, Clone, Copy)]
pub struct SemanticColors {
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

    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_faint: Color,
    pub text_on_accent: Color,
    pub text_on_surface: Color,
    pub text_disabled: Color,
    pub text_link: Color,

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

    pub hover_background: Color,
    pub active_background: Color,
    pub selected_background: Color,
    pub selected_text_background: Color,
    pub selected_editor_background: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    pub focus_ring: Color,

    pub editor_gutter_background: Color,
    pub editor_line_highlight: Color,
    pub editor_cursor: Color,
    pub editor_selection: Color,
    pub editor_find_highlight: Color,

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
