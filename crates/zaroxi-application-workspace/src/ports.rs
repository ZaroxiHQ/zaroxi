// Application workspace orchestrator trait skeleton.
//
// This trait composes domain and core ports to implement use cases like "open workspace"
// and "open buffer" from the UI. Keep it minimal for the first slice.

use std::path::PathBuf;
use std::sync::Arc;
use crate as _; // placeholder for crate root
use serde::{Serialize, Deserialize};

use std::pin::Pin;
use std::future::Future;

/// Boxed future alias (replace with kernel::BoxFuture in real code)
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// DTO: workspace session created by the application
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSessionDTO {
    pub session_id: String,
    pub workspace_id: String,
}

/// Command used to request open workspace
#[derive(Clone, Debug)]
pub struct WorkspaceOpenCommand {
    pub path: PathBuf,
}

/// Very small service trait. Implementations are in application layer.
pub trait WorkspaceService: Send + Sync {
    /// Open a workspace and create a session for UI. Returns a session DTO.
    fn open_workspace(&self, cmd: WorkspaceOpenCommand) -> BoxFuture<'static, Result<WorkspaceSessionDTO, String>>;

    /// Open a buffer inside an active session (session_id is a string for the skeleton).
    fn open_buffer(&self, session_id: String, path: PathBuf) -> BoxFuture<'static, Result<String /* buffer id */, String>>;
}

pub type DynWorkspaceService = Arc<dyn WorkspaceService>;
