#![allow(dead_code)]
// Auto-generated stub for `zaroxi-core-ui`.

pub const CRATE_NAME: &str = "zaroxi-core-ui";

pub fn info() -> &'static str {
    CRATE_NAME
}

pub mod engine_shell_view_input;
pub use engine_shell_view_input::{EngineSelection, EngineShellViewInput};

pub mod document_viewport;
pub use document_viewport::{
    CaretModel, DocumentViewport, RenderedDocument, ScrollModel, SelectionModel,
};
