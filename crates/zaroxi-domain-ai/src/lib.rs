//! AI context collection, ranking, packing, and prompt construction.

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
