//! Settings domain model for Zaroxi.
//!
//! Defines typed preferences (theme, font, telemetry), the `Settings` aggregate,
//! resolution helpers (`EffectiveTheme`), and `SettingsAction` for dispatching
//! user intent. This crate sits in the `domain` layer: it depends on nothing
//! above `kernel` and carries no persistence or UI logic.

use serde::{Deserialize, Serialize};

/// The user's theme preference.
///
/// `System` means the app should follow the OS-level dark/light mode. `Dark`
/// and `Light` force the corresponding palette regardless of the OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemePreference {
    /// All available variants in display order.
    pub fn all() -> &'static [Self] {
        &[Self::System, Self::Dark, Self::Light]
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

/// The user's editor font preference.
///
/// Controls which typeface the renderer uses for editor text and UI text.
/// `JetBrainsMonoNerdFont` is the workspace-bundled monospace (preferred);
/// `JetBrainsMono` is the non-nerd variant (fallback). Additional variants
/// can be added in later phases without changing the Settings model shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FontPreference {
    /// Workspace-bundled JetBrains Mono Nerd Font (patched for icons).
    #[default]
    JetBrainsMonoNerdFont,
    /// Workspace-bundled JetBrains Mono (no icon patching).
    JetBrainsMono,
}

impl FontPreference {
    /// All available variants in display order.
    pub fn all() -> &'static [Self] {
        &[Self::JetBrainsMonoNerdFont, Self::JetBrainsMono]
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::JetBrainsMonoNerdFont => "JetBrains Mono NF",
            Self::JetBrainsMono => "JetBrains Mono",
        }
    }
}

/// Telemetry preference (privacy toggle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TelemetryPreference {
    /// Whether anonymous usage data is sent.
    pub enabled: bool,
}

/// The aggregate settings model — the single source of truth for all user-
/// facing preferences that affect app behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Settings {
    pub theme: ThemePreference,
    pub font: FontPreference,
    pub telemetry: TelemetryPreference,
}

impl Settings {
    /// Create with factory-default values.
    pub fn new() -> Self {
        Self::default()
    }
}

/// The resolved (effective) theme after evaluating `System` against the OS
/// preference. `EffectiveTheme` always reduces to a concrete `Dark` or `Light`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveTheme {
    Dark,
    Light,
}

impl EffectiveTheme {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

/// Resolve a `ThemePreference` to the concrete `EffectiveTheme` using the
/// OS-level dark-mode flag.
pub fn effective_theme(pref: ThemePreference, system_is_dark: bool) -> EffectiveTheme {
    match pref {
        ThemePreference::Dark => EffectiveTheme::Dark,
        ThemePreference::Light => EffectiveTheme::Light,
        ThemePreference::System => {
            if system_is_dark {
                EffectiveTheme::Dark
            } else {
                EffectiveTheme::Light
            }
        }
    }
}

/// Mutation intents dispatched from the UI to update settings state.
///
/// The UI (interface layer) constructs these actions; the application layer
/// applies them to the domain state. Each action carries the new value so
/// the handler is a pure state transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    SetTheme(ThemePreference),
    SetFont(FontPreference),
    SetTelemetry(bool),
}

/// Convenience: convert a [`ThemePreference`] into the interface-layer
/// [`ZaroxiTheme`]. This is intentionally kept as a free function rather
/// than a `From` impl so `zaroxi-domain-settings` never depends on
/// `zaroxi-interface-theme`.
impl ThemePreference {
    /// Map to the theme-crate enum (caller supplies the import).
    /// Returns `("System", true)` / `("Dark", false)` / `("Light", false)`
    /// for callers that need to construct `ZaroxiTheme` + `system_is_dark`.
    pub fn to_theme_tag(&self) -> (&'static str, bool) {
        match self {
            Self::System => ("System", true),
            Self::Dark => ("Dark", false),
            Self::Light => ("Light", false),
        }
    }
}
