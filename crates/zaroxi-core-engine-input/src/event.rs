//! Minimal engine input event shim used by the runtime.
//!
//! This small module provides a placeholder `Event` type and a conversion
//! helper from winit's `WindowEvent`. It intentionally returns `None` for
//! unsupported events and is small to avoid pulling heavy input logic into
//! the runtime crate during this narrow compile-fix pass.

use winit::event::WindowEvent;

/// Minimal engine input event.
#[derive(Debug, Clone)]
pub struct Event {
    // Placeholder for future engine input fields.
}

impl Event {
    /// Convert a winit WindowEvent into an engine input Event.
    ///
    /// Returns `None` for events that are not currently translated.
    pub fn from_winit(_e: &WindowEvent) -> Option<Self> {
        None
    }
}
