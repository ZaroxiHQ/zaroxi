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
            // Minimal behavior: create a kernel Id and echo the provided path into DTO
            let dto = WorkspaceDTO {
                id: zaroxi_kernel_types::Id::new(),
                root_path: cmd.path.clone(),
                name: "Sample Workspace".to_string(),
            };
            Ok(dto)
        })
    }
}

use std::collections::HashMap;
use std::sync::Mutex;

/// In-memory buffer store (infrastructure adapter).
pub struct InMemoryBufferStore {
    inner: std::sync::Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryBufferStore {
    pub fn new() -> Self {
        InMemoryBufferStore { inner: std::sync::Arc::new(Mutex::new(HashMap::new())) }
    }
}

impl BufferStore for InMemoryBufferStore {
    fn open_buffer(&self, path: PathBuf) -> BoxFuture<'static, Result<BufferId, BufferError>> {
        let key = format!("buf:{}", path.to_string_lossy());
        let k_clone = key.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            // Ensure an entry exists (empty content) for the opened buffer.
            let mut m = inner.lock().unwrap();
            m.entry(k_clone.clone()).or_insert_with(|| "// sample file\nfn main() { println!(\"Hello Phase0\"); }\n".to_string());
            Ok(BufferId(key))
        })
    }

    fn get_text(&self, id: &BufferId) -> Option<String> {
        let m = self.inner.lock().unwrap();
        m.get(&id.0).cloned()
    }

    fn set_text(&self, id: &BufferId, content: String) -> BoxFuture<'static, Result<(), BufferError>> {
        let key = id.0.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut m = inner.lock().unwrap();
            if m.contains_key(&key) {
                m.insert(key, content);
                Ok(())
            } else {
                Err(BufferError("buffer not found".to_string()))
            }
        })
    }
}

/// Export helpers to turn into Arc'd dyn trait objects.
pub fn into_workspace_repo(repo: InMemoryWorkspaceRepo) -> Arc<dyn WorkspaceRepository> {
    Arc::new(repo)
}

pub fn into_buffer_store(store: InMemoryBufferStore) -> Arc<dyn BufferStore> {
    Arc::new(store)
}
