// Core engine render crate exports.
//
// This file exports the existing renderer surface/error modules and the
// new tiny semantic render-intent module.

pub mod error;
pub mod renderer;
pub mod surface;
pub mod intent;

pub use renderer::Renderer;
pub use renderer::RenderLayout;
pub use renderer::Rect;
pub use renderer::UiBlock;
pub use error::RenderError;

// Export the tiny semantic render intent for Phase 52.
pub use intent::{ShellRenderIntent, RenderSection};
