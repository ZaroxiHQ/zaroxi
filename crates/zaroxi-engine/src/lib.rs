#![warn(missing_docs)]
//! High-level engine facade.
//!
//! Exposes a small, stable surface for running the engine.
//! Keeps implementation details in sub-crates.

use anyhow::Result;

pub use zaroxi_engine_input::event::Event as InputEvent;

/// Minimal configuration for launching the engine window.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Window title shown in the desktop app.
    pub title: String,
    /// Initial window width in physical pixels.
    pub width: u32,
    /// Initial window height in physical pixels.
    pub height: u32,
    /// Clear color as RGBA in 0.0..=1.0
    pub clear_color: [f64; 4],
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Zaroxi Studio".to_string(),
            width: 1280,
            height: 800,
            clear_color: [0.08, 0.09, 0.12, 1.0],
        }
    }
}

/// Run the engine with the provided configuration.
/// This is the public entrypoint used by apps/desktop.
pub fn run(config: EngineConfig) -> Result<()> {
    // Forward primitive fields to the runtime to avoid a cyclic dependency.
    zaroxi_engine_runtime::run(
        config.title,
        config.width,
        config.height,
        config.clear_color,
    )
}
