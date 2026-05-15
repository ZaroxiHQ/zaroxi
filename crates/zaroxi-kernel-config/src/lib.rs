#![doc = "Application configuration and runtime defaults for Zaroxi Studio."]

use serde::{Deserialize, Serialize};

/// Minimal app configuration used by the desktop entrypoint and by tests.
///
/// Keep this crate small — more advanced configuration / persistence can be
/// added later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Human-facing title shown in the window.
    pub title: String,
    /// Initial window width (physical pixels).
    pub window_width: u32,
    /// Initial window height (physical pixels).
    pub window_height: u32,
    /// Clear color used by the renderer (RGBA 0.0..=1.0).
    pub clear_color: [f64; 4],
    /// Editor font size placeholder for future use.
    pub editor_font_size: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "Zaroxi Studio".to_string(),
            window_width: 1280,
            window_height: 800,
            clear_color: [0.08, 0.09, 0.12, 1.0],
            editor_font_size: 14.0,
        }
    }
}
