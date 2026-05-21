//! AI context collection, ranking, packing, and prompt construction.
///
/// This crate provides utilities for collecting contextual signals, packing
/// them into prompts, ranking candidates, and building final prompts for
/// downstream AI components.
pub mod context;
pub mod packing;
pub mod prompt;
pub mod ranking;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::context::*;
    pub use super::packing::*;
    pub use super::prompt::*;
    pub use super::ranking::*;
}
