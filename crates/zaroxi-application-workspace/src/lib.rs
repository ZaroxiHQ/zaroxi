/// Workspace service orchestration logic for Zaroxi Studio.
///
/// Application-level orchestrators (use-case services) live here. They depend on
/// domain contracts and core ports, but not on infrastructure or interface.
/// For Phase 0 keep implementations minimal and focused on the single slice.
pub mod service;
pub mod workspace_manager;
pub mod ports;

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::service::*;
    pub use super::workspace_manager::*;
    pub use super::ports::*;
}
