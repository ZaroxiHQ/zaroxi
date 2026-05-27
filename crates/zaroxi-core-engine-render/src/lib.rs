// Core engine render crate exports.
//
// This file exports the existing renderer surface/error modules and the
// new tiny semantic render-intent module.
//
// To keep Phase 52 tiny and compileable in CI without heavy GPU/windowing
// dependencies, the concrete renderer/surface/error modules are gated behind
// the "full_renderer" feature. The semantic `intent` module is always
// available and exported so layout -> render conversions can be tested
// without pulling in wgpu/winit/etc.

#[cfg(feature = "full_renderer")]
pub mod error;
#[cfg(feature = "full_renderer")]
pub mod renderer;
#[cfg(feature = "full_renderer")]
pub mod surface;

pub mod consistency;
pub mod intent;
pub mod plan;
pub mod text_seam;
pub mod transcript;

#[cfg(feature = "full_renderer")]
pub use error::RenderError;
#[cfg(feature = "full_renderer")]
pub use renderer::Rect;
#[cfg(feature = "full_renderer")]
pub use renderer::RenderLayout;
#[cfg(feature = "full_renderer")]
pub use renderer::Renderer;
#[cfg(feature = "full_renderer")]
pub use renderer::UiBlock;

// Export the tiny semantic render intent and the Phase 53 draw-plan adapter.
// Intent remains always available; the draw-plan is a semantic, non-rendering
// adapter built from ShellRenderIntent.
pub use intent::{RenderSection, ShellRenderIntent};
pub use plan::{DrawSection, ShellDrawPlan};
pub use transcript::ShellRenderTranscript;

// Export the tiny deterministic text renderer and the consistency seam.
pub mod shell_text_renderer;
pub use shell_text_renderer::ShellTextRenderer;

pub use consistency::{RenderConsistencyReport, analyze};
