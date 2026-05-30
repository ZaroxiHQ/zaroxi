mod backend;
pub mod core;
mod debug;
mod geometry;
mod pipelines;
mod shapes;
mod surface;
mod text;
pub use text::desktop_shim;
mod text_atlas;
mod text_pipeline;
mod ui;

/// Public facade for the renderer module.
///
/// Internal implementation modules are kept private; only the stable, intended
/// public API is re-exported here.
pub use core::Renderer;
pub use core::{Rect, RenderLayout};
pub use ui::UiBlock;
