mod core;
mod debug;
mod geometry;
mod surface;
mod pipelines;
mod text;
mod backend;
mod shapes;
mod ui;

/// Public facade for the renderer module.
///
/// Internal implementation modules are kept private; only the stable, intended
/// public API is re-exported here.
pub use core::Renderer;
pub use core::{RenderLayout, Rect};
pub use ui::UiBlock;
