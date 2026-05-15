//! Shared protocol definitions for Zaroxi.

pub mod commands;
pub mod events;
pub mod workspace;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::commands::*;
    pub use super::events::*;
    pub use super::workspace::*;
}
