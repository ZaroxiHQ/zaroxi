//! Workspace service orchestration logic for Zaroxi Studio.

pub mod service;
pub mod workspace_manager;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::service::*;
    pub use super::workspace_manager::*;
}
