#![warn(missing_docs)]
//! High-level engine facade.
//!
//! Exposes a small, stable surface for running the engine.
//! Keeps implementation details in sub-crates.

use anyhow::Result;
use std::sync::Arc;

pub use zaroxi_engine_input::event::Event as InputEvent;

/// Minimal configuration for launching the engine window.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
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
    // Delegate to the runtime which owns the event loop.
    zaroxi_engine_runtime::run(config)
}
