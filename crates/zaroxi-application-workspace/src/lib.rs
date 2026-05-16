/// Workspace service orchestration logic for Zaroxi Studio.
///
/// Application-level orchestrators (use-case services) live here. They depend on
/// domain contracts and core ports, but not on infrastructure or interface.
/// For Phase 0 keep implementations minimal and focused on the single slice.
pub mod service;
pub mod workspace_manager;
pub mod ports;

/// Prelude for convenient imports.
///
/// Be explicit about exported symbols to avoid ambiguous glob re-export warnings.
/// Ports own the public trait/type surface; re-export those intentionally.
/// Workspace manager helpers are re-exported as a group.
pub mod prelude {
    // Re-export the application-owned port/type surface explicitly.
    pub use crate::ports::{
        WorkspaceService,
        WorkspaceOpenCommand,
        WorkspaceSessionDTO,
        AppCommand,
        CommandResult,
        DynWorkspaceService,
    };

    // Re-export manager helpers (names here are intentionally grouped).
    pub use crate::workspace_manager::*;
}
