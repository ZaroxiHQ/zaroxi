mod backend;
pub mod core;
mod debug;
mod geometry;
mod header_layout;
mod pipelines;
mod shapes;
mod surface;
mod text;
#[cfg(feature = "full_renderer")]
pub mod vello_overlay;
pub use text::desktop_shim;
mod text_atlas;
mod text_pipeline;
mod ui;

/// Public facade for the renderer module.
///
/// Internal implementation modules are kept private; only the stable, intended
/// public API is re-exported here.
pub use core::{CockpitText, PanelColors, Rect, RenderCore, RenderLayout, Renderer};
pub use ui::UiBlock;
