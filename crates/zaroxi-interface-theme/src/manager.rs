//! Theme management and persistence
use crate::{SemanticColors, ZaroxiTheme};
use serde::{Deserialize, Serialize};

/// Theme settings that can be persisted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSettings {
    /// The user's preferred theme mode
    pub theme_mode: ZaroxiTheme,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        ThemeSettings { theme_mode: ZaroxiTheme::Light }
    }
}

/// Manages theme state and resolution
#[derive(Default)]
pub struct ThemeManager {
    settings: ThemeSettings,
}

impl ThemeManager {
    /// Create a new theme manager with default settings
    pub fn new() -> Self {
        Self { settings: ThemeSettings::default() }
    }

    /// Create a theme manager with specific settings
    pub fn with_settings(settings: ThemeSettings) -> Self {
        Self { settings }
    }

    /// Get the current theme mode
    pub fn theme_mode(&self) -> ZaroxiTheme {
        self.settings.theme_mode
    }

    /// Set the theme mode
    pub fn set_theme_mode(&mut self, theme_mode: ZaroxiTheme) {
        self.settings.theme_mode = theme_mode;
    }

    /// Get the resolved semantic colors based on system preference
    pub fn resolved_colors(&self, system_is_dark: bool) -> SemanticColors {
        self.settings.theme_mode.colors(system_is_dark)
    }

    /// Get the settings for persistence
    pub fn settings(&self) -> &ThemeSettings {
        &self.settings
    }

    /// Update settings
    pub fn update_settings(&mut self, settings: ThemeSettings) {
        self.settings = settings;
    }
}
