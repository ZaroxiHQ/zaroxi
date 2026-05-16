/*!
 Minimal in-memory infrastructure adapters for Phase 0:
 - InMemoryWorkspaceRepo implements domain WorkspaceRepository
 - InMemoryBufferStore implements core BufferStore

 These adapters are intentionally tiny and synchronous where convenient to keep the slice simple.
*/

use std::sync::Arc;
use std::path::PathBuf;
use std::pin::Pin;
use std::future::Future;

use zaroxi_domain_workspace::ports::{
    WorkspaceRepository, WorkspaceOpenCommand, WorkspaceDTO, DomainError,
};
use zaroxi_core_editor_buffer::ports::{
    BufferStore, BufferId, BufferError,
};

/// Simple boxed future helper for this tiny crate.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// In-memory workspace repository (infrastructure adapter).
pub struct InMemoryWorkspaceRepo;

impl InMemoryWorkspaceRepo {
    pub fn new() -> Self {
        InMemoryWorkspaceRepo
    }
}

impl WorkspaceRepository for InMemoryWorkspaceRepo {
    fn open_workspace(&self, cmd: WorkspaceOpenCommand) -> BoxFuture<'static, Result<WorkspaceDTO, DomainError>> {
        Box::pin(async move {
            // Minimal behavior: echo the provided path into DTO
            let dto = WorkspaceDTO {
                id: "workspace-1".to_string(),
                root_path: cmd.path.clone(),
                name: "Sample Workspace".to_string(),
            };
            Ok(dto)
        })
    }
}

/// In-memory buffer store (infrastructure adapter).
pub struct InMemoryBufferStore;

impl InMemoryBufferStore {
    pub fn new() -> Self {
        InMemoryBufferStore
    }
}

impl BufferStore for InMemoryBufferStore {
    fn open_buffer(&self, path: PathBuf) -> BoxFuture<'static, Result<BufferId, BufferError>> {
        Box::pin(async move {
            // Minimal behavior: return a deterministic buffer id.
            let id = BufferId(format!("buf:{}", path.to_string_lossy()));
            Ok(id)
        })
    }

    fn get_text(&self, id: &BufferId) -> Option<String> {
        // Return a small canned file content for the slice.
        Some("// sample file\nfn main() { println!(\"Hello Phase0\"); }\n".to_string())
    }
}

/// Export helpers to turn into Arc'd dyn trait objects.
pub fn into_workspace_repo(repo: InMemoryWorkspaceRepo) -> Arc<dyn WorkspaceRepository> {
    Arc::new(repo)
}

pub fn into_buffer_store(store: InMemoryBufferStore) -> Arc<dyn BufferStore> {
    Arc::new(store)
}
