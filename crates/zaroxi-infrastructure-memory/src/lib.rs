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

/// In-memory history/event store.
pub struct InMemoryHistoryStore {
    cmds: std::sync::Arc<Mutex<Vec<zaroxi_application_workspace::ports::CommandRecord>>>,
    evs: std::sync::Arc<Mutex<Vec<zaroxi_application_workspace::ports::WorkspaceEvent>>>,
}

impl InMemoryHistoryStore {
    pub fn new() -> Self {
        InMemoryHistoryStore { cmds: std::sync::Arc::new(Mutex::new(Vec::new())), evs: std::sync::Arc::new(Mutex::new(Vec::new())) }
    }
}

use zaroxi_application_workspace::ports::{HistoryRepository, CommandRecord, WorkspaceEvent, SessionId};

impl HistoryRepository for InMemoryHistoryStore {
    fn record_command(&self, rec: CommandRecord) -> BoxFuture<'static, Result<(), String>> {
        let inner = self.cmds.clone();
        Box::pin(async move {
            let mut v = inner.lock().unwrap();
            v.push(rec);
            Ok(())
        })
    }

    fn record_event(&self, ev: WorkspaceEvent) -> BoxFuture<'static, Result<(), String>> {
        let inner = self.evs.clone();
        Box::pin(async move {
            let mut v = inner.lock().unwrap();
            v.push(ev);
            Ok(())
        })
    }

    fn get_recent_commands(&self, session_id: SessionId, limit: usize) -> BoxFuture<'static, Result<Vec<CommandRecord>, String>> {
        let inner = self.cmds.clone();
        Box::pin(async move {
            let v = inner.lock().unwrap();
            // filter by session and return most recent up to limit
            let mut filtered: Vec<CommandRecord> = v.iter().cloned().filter(|c| c.session_id.as_ref().map(|s| s == &session_id).unwrap_or(false)).collect();
            filtered.sort_by_key(|c| c.timestamp);
            if filtered.len() > limit {
                let start = filtered.len() - limit;
                filtered = filtered[start..].to_vec();
            }
            Ok(filtered)
        })
    }

    fn get_recent_events(&self, session_id: SessionId, limit: usize) -> BoxFuture<'static, Result<Vec<WorkspaceEvent>, String>> {
        let inner = self.evs.clone();
        Box::pin(async move {
            let v = inner.lock().unwrap();
            let mut filtered: Vec<WorkspaceEvent> = v.iter().cloned().filter(|e| e.session_id == session_id).collect();
            filtered.sort_by_key(|e| e.timestamp);
            if filtered.len() > limit {
                let start = filtered.len() - limit;
                filtered = filtered[start..].to_vec();
            }
            Ok(filtered)
        })
    }
}

/// Export helper to get Arc<dyn HistoryRepository>
pub fn into_history_store(store: InMemoryHistoryStore) -> Arc<dyn HistoryRepository> {
    Arc::new(store)
}
 
// --------- Checkpoint durability adapter (in-memory) ---------
//
// Lightweight in-memory checkpoint store used for Phase 9 durability tests and harness.
// It stores serialized checkpoint bytes under an opaque location id (UUID string).
//
use serde_json;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Mutex as StdMutex;

/// In-memory checkpoint store.
pub struct InMemoryCheckpointStore {
    inner: std::sync::Arc<StdMutex<HashMap<String, Vec<u8>>>>,
}

impl InMemoryCheckpointStore {
    pub fn new() -> Self {
        InMemoryCheckpointStore { inner: std::sync::Arc::new(StdMutex::new(HashMap::new())) }
    }
 
    /// Insert raw bytes under a location id (useful for tests to inject malformed data).
    pub fn insert_raw(&self, location: String, data: Vec<u8>) {
        let mut m = self.inner.lock().unwrap();
        m.insert(location, data);
    }
}
 
impl zaroxi_application_workspace::ports::DurabilityRepository for InMemoryCheckpointStore {
    fn save_checkpoint(&self, checkpoint: zaroxi_application_workspace::ports::Checkpoint) -> BoxFuture<'static, Result<String, zaroxi_application_workspace::ports::DurabilityError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            // Serialize checkpoint to JSON.
            let bytes = serde_json::to_vec(&checkpoint).map_err(|e| zaroxi_application_workspace::ports::DurabilityError::Malformed(e.to_string()))?;
            let id = Uuid::new_v4().to_string();
            let mut m = inner.lock().unwrap();
            m.insert(id.clone(), bytes);
            Ok(id)
        })
    }
 
    fn load_checkpoint(&self, location: String) -> BoxFuture<'static, Result<zaroxi_application_workspace::ports::Checkpoint, zaroxi_application_workspace::ports::DurabilityError>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let m = inner.lock().unwrap();
            let bytes = m.get(&location).ok_or(zaroxi_application_workspace::ports::DurabilityError::NotFound(location.clone()))?;
            // Attempt to deserialize; return explicit malformed error on failure.
            let ck: zaroxi_application_workspace::ports::Checkpoint = serde_json::from_slice(bytes).map_err(|e| zaroxi_application_workspace::ports::DurabilityError::Malformed(e.to_string()))?;
            // Validate known version(s)
            if ck.version != 1 {
                return Err(zaroxi_application_workspace::ports::DurabilityError::UnknownVersion(ck.version));
            }
            Ok(ck)
        })
    }
}
 
/// Export helper to get Arc<dyn DurabilityRepository> from the in-memory store.
pub fn into_checkpoint_store(store: InMemoryCheckpointStore) -> Arc<dyn zaroxi_application_workspace::ports::DurabilityRepository> {
    Arc::new(store)
}
