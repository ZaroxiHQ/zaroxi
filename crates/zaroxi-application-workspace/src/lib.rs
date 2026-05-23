/// Workspace service orchestration logic for Zaroxi Studio.
///
/// Application-level orchestrators (use-case services) live here. They depend on
/// domain contracts and core ports, but not on infrastructure or interface.
/// For Phase 1 keep implementations minimal and focused on the single slice.
pub mod service;
pub mod editor_service;
pub use editor_service::EditorService;
pub mod workspace_manager;
pub mod ports;
pub mod in_memory_adapters;
pub mod usecases;
pub mod view; // small, read-only view seam (Phase 2)

/// Prelude for convenient imports.
///
/// Be explicit about exported symbols to avoid ambiguous glob re-export warnings.
/// Re-export application-owned types and the orchestrator.
pub mod prelude {
    // Re-export the application-owned port/type surface explicitly.
    pub use crate::ports::{
        WorkspaceService,
        WorkspaceBootRequest,
        WorkspaceBootResponse,
        OpenBufferRequest,
        OpenBufferResponse,
        DispatchCommandRequest,
        DispatchCommandResponse,
        AppCommand,
        CommandResult,
        WorkspaceSessionDTO,
        DynWorkspaceService,
    };

    // Re-export the concrete orchestrator type for convenience.
    pub use crate::usecases::WorkspaceOrchestrator;

    // Re-export manager helpers.
    pub use crate::workspace_manager::*;

    // Re-export thin view helpers (Phase 2)
    pub use crate::view::*;
}
