//! Workspace service implementation.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use tracing::info;
use zaroxi_kernel_types::Id;

/// Workspace service for handling workspace operations.
pub struct WorkspaceService {
    /// Internal state.
    state: Arc<Mutex<WorkspaceServiceState>>,
}

struct WorkspaceServiceState {
    /// Whether the service is running.
    running: bool,
}

impl WorkspaceService {
    /// Create a new workspace service.
    pub fn new() -> Self {
        Self { state: Arc::new(Mutex::new(WorkspaceServiceState { running: false })) }
    }

    /// Start the workspace service.
    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.running {
            return Err(anyhow::anyhow!("Workspace service is already running"));
        }
        state.running = true;
        info!("Workspace service started");
        Ok(())
    }

    /// Stop the workspace service.
    pub async fn stop(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if !state.running {
            return Err(anyhow::anyhow!("Workspace service is not running"));
        }
        state.running = false;
        info!("Workspace service stopped");
        Ok(())
    }

    /// Check if the service is running.
    pub async fn is_running(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.running
    }

    /// Open a workspace at the given path
    ///
    /// Note: this helper intentionally does minimal validation and returns a kernel Id
    /// representing the opened workspace. The authoritative workspace model and creation
    /// policies are owned by the domain and the WorkspaceRepository port; callers should
    /// prefer using the WorkspaceOrchestrator which delegates to the domain repository.
    pub async fn open_workspace(&self, path: std::path::PathBuf) -> Result<Id> {
        // Validate path exists
        if !path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", path));
        }
        if !path.is_dir() {
            return Err(anyhow::anyhow!("Path is not a directory: {:?}", path));
        }

        // Check if we can read the directory
        std::fs::read_dir(&path)
            .map_err(|e| anyhow::anyhow!("Cannot read directory: {:?}: {}", path, e))?;

        // Create a kernel-level Id for the opened workspace. The domain model remains authoritative.
        let id = Id::new();

        info!("Validated workspace path {:?}, assigned id {}", path, id);
        Ok(id)
    }

    /// Get workspace metadata (future enhancement)
    pub async fn get_workspace_metadata(&self, workspace_id: Id) -> Result<WorkspaceMetadata> {
        // TODO: Implement actual metadata retrieval
        Ok(WorkspaceMetadata { id: workspace_id, file_count: 0, total_size: 0, last_indexed: None })
    }
}

/// Workspace metadata
#[derive(Debug, Clone)]
pub struct WorkspaceMetadata {
    pub id: Id,
    pub file_count: usize,
    pub total_size: u64,
    pub last_indexed: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn open_workspace_returns_id_on_existing_dir() {
        let svc = WorkspaceService::new();
        let cur = std::env::current_dir().unwrap();
        let id = svc.open_workspace(cur).await.expect("open ok");
        assert!(id.as_uuid().to_string().len() > 0);
    }
}
