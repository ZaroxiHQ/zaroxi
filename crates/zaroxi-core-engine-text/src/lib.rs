#![allow(dead_code)]
// Auto-generated stub for `zaroxi-core-engine-text`.

pub const CRATE_NAME: &str = "zaroxi-core-engine-text";

pub fn info() -> &'static str {
    CRATE_NAME
}

mod label;
pub use label::TextLabel;

mod backend;
pub use backend::{TextBackend, TextLayout, new_backend, DummyBackend};
#[cfg(feature = "glyphon_backend")]
pub use backend::glyphon_impl::GlyphonBackend;
