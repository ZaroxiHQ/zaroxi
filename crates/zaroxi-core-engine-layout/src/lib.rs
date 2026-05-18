#![allow(dead_code)]
// Auto-generated stub for `zaroxi-core-engine-layout`.
// Extended to export a tiny, structural layout input type for Phase 51.

pub const CRATE_NAME: &str = "zaroxi-core-engine-layout";

pub fn info() -> &'static str {
    CRATE_NAME
}

// Public re-exports for the tiny layout-facing input model.
pub mod shell_layout_input;
pub use shell_layout_input::{
    LayoutBlock, SelectionBlock, ShellLayoutInput, StatusBlock, TextBlock, ViewportFacts,
};
