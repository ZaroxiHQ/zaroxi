/*!
 Minimal in-memory infrastructure adapters for Phase 0:
 - InMemoryWorkspaceRepo implements domain WorkspaceRepository
 - InMemoryBufferStore implements core BufferStore

 These adapters are intentionally tiny and synchronous where convenient to keep the slice simple.
*/

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

/// Simple boxed future helper for this tiny crate.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// NOTE:
// The InMemoryWorkspaceRepo adapter was removed from this infrastructure crate
// to avoid introducing forbidden dependencies on domain/core crates. This
// infrastructure crate now implements only application-facing durability/history
// adapters (HistoryRepository, DurabilityRepository) using the
// `zaroxi-application-workspace` port types. Workspace orchestration and any
// domain-level repository implementations belong in the application or domain
// crates and should be wired at composition time by the outer layer.

/// Maximum retained command and event records per store before oldest
/// entries are trimmed. Prevents unbounded growth for long-running sessions.
/// Override with `ZAROXI_HISTORY_CAP`.
const DEFAULT_HISTORY_CAP: usize = 10000;

fn history_cap() -> usize {
    std::env::var("ZAROXI_HISTORY_CAP")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_HISTORY_CAP)
}

/// In-memory history/event store.
pub struct InMemoryHistoryStore {
    cmds: std::sync::Arc<Mutex<Vec<zaroxi_application_workspace::ports::CommandRecord>>>,
    evs: std::sync::Arc<Mutex<Vec<zaroxi_application_workspace::ports::WorkspaceEvent>>>,
}

impl Default for InMemoryHistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryHistoryStore {
    pub fn new() -> Self {
        InMemoryHistoryStore {
            cmds: std::sync::Arc::new(Mutex::new(Vec::new())),
            evs: std::sync::Arc::new(Mutex::new(Vec::new())),
        }
    }
}

use zaroxi_application_workspace::ports::{
    CommandRecord, HistoryRepository, SessionId, WorkspaceEvent,
};

impl HistoryRepository for InMemoryHistoryStore {
    fn record_command(&self, rec: CommandRecord) -> BoxFuture<'static, Result<(), String>> {
        let inner = self.cmds.clone();
        Box::pin(async move {
            let mut v = inner.lock().unwrap();
            v.push(rec);
            let cap = history_cap();
            if v.len() > cap {
                let trim = v.len() - cap;
                v.drain(0..trim);
            }
            Ok(())
        })
    }

    fn record_event(&self, ev: WorkspaceEvent) -> BoxFuture<'static, Result<(), String>> {
        let inner = self.evs.clone();
        Box::pin(async move {
            let mut v = inner.lock().unwrap();
            v.push(ev);
            let cap = history_cap();
            if v.len() > cap {
                let trim = v.len() - cap;
                v.drain(0..trim);
            }
            Ok(())
        })
    }

    fn get_recent_commands(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> BoxFuture<'static, Result<Vec<CommandRecord>, String>> {
        let inner = self.cmds.clone();
        Box::pin(async move {
            let v = inner.lock().unwrap();
            // filter by session and return most recent up to limit
            let mut filtered: Vec<CommandRecord> = v
                .iter()
                .filter(|&c| c.session_id.as_ref().map(|s| s == &session_id.0).unwrap_or(false))
                .cloned()
                .collect();
            filtered.sort_by_key(|c| c.timestamp);
            if filtered.len() > limit {
                let start = filtered.len() - limit;
                filtered = filtered[start..].to_vec();
            }
            Ok(filtered)
        })
    }

    fn get_recent_events(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> BoxFuture<'static, Result<Vec<WorkspaceEvent>, String>> {
        let inner = self.evs.clone();
        Box::pin(async move {
            let v = inner.lock().unwrap();
            let mut filtered: Vec<WorkspaceEvent> =
                v.iter().filter(|&e| e.session_id == session_id).cloned().collect();
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
use std::sync::Mutex as StdMutex;
use uuid::Uuid;

/// Maximum retained checkpoints before oldest entries are removed.
const DEFAULT_CHECKPOINT_CAP: usize = 128;

/// In-memory checkpoint store.
pub struct InMemoryCheckpointStore {
    inner: std::sync::Arc<StdMutex<HashMap<String, Vec<u8>>>>,
    /// Ordered list of checkpoint ids (FIFO), used for eviction when the cap
    /// is exceeded.
    order: std::sync::Arc<StdMutex<Vec<String>>>,
}

impl Default for InMemoryCheckpointStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryCheckpointStore {
    pub fn new() -> Self {
        InMemoryCheckpointStore {
            inner: std::sync::Arc::new(StdMutex::new(HashMap::new())),
            order: std::sync::Arc::new(StdMutex::new(Vec::new())),
        }
    }

    /// Insert raw bytes under a location id (useful for tests to inject malformed data).
    pub fn insert_raw(&self, location: String, data: Vec<u8>) {
        let mut m = self.inner.lock().unwrap();
        m.insert(location, data);
    }
}

impl zaroxi_application_workspace::ports::DurabilityRepository for InMemoryCheckpointStore {
    fn save_checkpoint(
        &self,
        checkpoint: zaroxi_application_workspace::ports::Checkpoint,
    ) -> BoxFuture<'static, Result<String, zaroxi_application_workspace::ports::DurabilityError>>
    {
        let inner = self.inner.clone();
        let order = self.order.clone();
        Box::pin(async move {
            // Serialize checkpoint to JSON.
            let bytes = serde_json::to_vec(&checkpoint).map_err(|e| {
                zaroxi_application_workspace::ports::DurabilityError::Malformed(e.to_string())
            })?;
            let id = Uuid::new_v4().to_string();
            {
                let mut m = inner.lock().unwrap();
                m.insert(id.clone(), bytes);
            }
            {
                let mut ord = order.lock().unwrap();
                ord.push(id.clone());
                while ord.len() > DEFAULT_CHECKPOINT_CAP {
                    if let Some(old_id) = ord.first().cloned() {
                        ord.remove(0);
                        let mut m = inner.lock().unwrap();
                        m.remove(&old_id);
                    }
                }
            }
            Ok(id)
        })
    }

    fn load_checkpoint(
        &self,
        location: String,
    ) -> BoxFuture<
        'static,
        Result<
            zaroxi_application_workspace::ports::Checkpoint,
            zaroxi_application_workspace::ports::DurabilityError,
        >,
    > {
        let inner = self.inner.clone();
        Box::pin(async move {
            let m = inner.lock().unwrap();
            let bytes = m.get(&location).ok_or(
                zaroxi_application_workspace::ports::DurabilityError::NotFound(location.clone()),
            )?;
            // Attempt to deserialize; return explicit malformed error on failure.
            let ck: zaroxi_application_workspace::ports::Checkpoint = serde_json::from_slice(bytes)
                .map_err(|e| {
                    zaroxi_application_workspace::ports::DurabilityError::Malformed(e.to_string())
                })?;
            // Validate known version(s)
            if ck.version != 1 {
                return Err(zaroxi_application_workspace::ports::DurabilityError::UnknownVersion(
                    ck.version,
                ));
            }
            Ok(ck)
        })
    }
}

/// Export helper to get Arc<dyn DurabilityRepository> from the in-memory store.
pub fn into_checkpoint_store(
    store: InMemoryCheckpointStore,
) -> Arc<dyn zaroxi_application_workspace::ports::DurabilityRepository> {
    Arc::new(store)
}
