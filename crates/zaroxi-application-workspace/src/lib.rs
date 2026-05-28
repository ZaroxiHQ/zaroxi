pub mod editor_service;
/// Workspace service orchestration logic for Zaroxi Studio.
///
/// Application-level orchestrators (use-case services) live here. They depend on
/// domain contracts and core ports, but not on infrastructure or interface.
/// For Phase 1 keep implementations minimal and focused on the single slice.
pub mod service;
pub use editor_service::EditorService;
pub mod in_memory_adapters;
pub mod ports;
pub mod usecases;
pub mod view;
pub mod workspace_manager; // small, read-only view seam (Phase 2)

use std::path::PathBuf;
use std::io;
use zaroxi_core_workspace_files::FileStorage;

/// Thin application-level helpers for Phase 9 disk-backed operations.
///
/// These convenience functions are small facades used by integration tests and
/// simple harnesses; richer application commands should live in the ports/usecases.
pub fn save_buffer_to_disk(path: &PathBuf, contents: &str) -> io::Result<()> {
    let storage = zaroxi_core_workspace_files::DiskFileStorage::new();
    storage.write_file(path, contents)
}

pub fn read_file_from_disk(path: &PathBuf) -> io::Result<String> {
    let storage = zaroxi_core_workspace_files::DiskFileStorage::new();
    storage.read_file(path)
}

/// Prelude for convenient imports.
///
/// Be explicit about exported symbols to avoid ambiguous glob re-export warnings.
/// Re-export application-owned types and the orchestrator.
pub mod prelude {
    // Re-export the application-owned port/type surface explicitly.
    pub use crate::ports::{
        AppCommand, CommandResult, DispatchCommandRequest, DispatchCommandResponse,
        DynWorkspaceService, OpenBufferRequest, OpenBufferResponse, WorkspaceBootRequest,
        WorkspaceBootResponse, WorkspaceService, WorkspaceSessionDTO,
    };

    // Re-export the concrete orchestrator type for convenience.
    pub use crate::usecases::WorkspaceOrchestrator;

    // Re-export manager helpers.
    pub use crate::workspace_manager::*;

    // Re-export thin view helpers (Phase 2)
    pub use crate::view::*;
}
