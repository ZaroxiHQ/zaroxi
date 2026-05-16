/// Workspace domain models for Zaroxi.
///
/// This crate contains the domain data types and pure policies for workspaces.
/// Keep the public surface minimal for Phase 0.
pub mod file_tree;
pub mod workspace;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::file_tree::*;
    pub use super::workspace::*;
}
