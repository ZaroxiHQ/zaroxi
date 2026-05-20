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
// NOTE: We intentionally DO NOT re-export the GlyphonBackend concrete type here.
// Keeping Glyphon types fully private to this crate preserves the architecture
// invariant that implementation details (Glyphon) never leak into higher layers.
