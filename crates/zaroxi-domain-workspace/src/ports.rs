// Domain workspace ports and DTOs for Phase 0 / Phase 1.
//
// This file defines the WorkspaceRepository port that infrastructure implements
// and the small DTOs used across layers.

use crate as _; // keep referencing the crate root to avoid unused warnings in skeletons
use std::path::PathBuf;
use std::sync::Arc;
use std::fmt;

/// Minimal boxed future alias copied locally to avoid kernel coupling in skeletons.
/// In the real implementation, import kernel::BoxFuture.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Small error type for the slice.
#[derive(Debug, Clone)]
pub struct DomainError(pub String);

impl From<&str> for DomainError {
    fn from(s: &str) -> Self {
        DomainError(s.to_string())
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// DTO: workspace open command
#[derive(Clone, Debug)]
pub struct WorkspaceOpenCommand {
    pub path: PathBuf,
}

/// DTO: workspace metadata returned after open
#[derive(Clone, Debug)]
pub struct WorkspaceDTO {
    pub id: String,
    pub root_path: PathBuf,
    pub name: String,
}

/// Port: WorkspaceRepository (infrastructure implements this)
pub trait WorkspaceRepository: Send + Sync {
    /// Open a workspace from a filesystem path.
    fn open_workspace(&self, cmd: WorkspaceOpenCommand) -> BoxFuture<'static, Result<WorkspaceDTO, DomainError>>;
}

/// Helper Arc type for passing implementations around.
pub type DynWorkspaceRepository = Arc<dyn WorkspaceRepository>;
