use std::sync::Arc;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{WorkspaceBootRequest, SaveCheckpointRequest, LoadCheckpointRequest};
use zaroxi_application_workspace::ports as ports;
use zaroxi_application_workspace::ports::WorkspaceService;
use zaroxi_domain_workspace::ports as domain_ports;
use zaroxi_core_editor_buffer::ports as buffer_ports;
use zaroxi_application_ai::ports as ai_ports;
use zaroxi_kernel_types::Id;

/// Fake domain repo
struct FakeRepo;
impl domain_ports::WorkspaceRepository for FakeRepo {
    fn open_workspace(&self, cmd: domain_ports::WorkspaceOpenCommand) -> ports::BoxFuture<'static, Result<domain_ports::WorkspaceDTO, domain_ports::DomainError>> {
        Box::pin(async move {
            Ok(domain_ports::WorkspaceDTO { id: Id::new(), root_path: cmd.path.clone(), name: "Test".to_string() })
        })
    }
}

/// Simple in-test buffer store supporting set_text/get_text/open_buffer
struct FakeStore {
    inner: Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
}

impl FakeStore {
    fn new() -> Self {
        Self { inner: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())) }
    }
}

impl buffer_ports::BufferStore for FakeStore {
    fn open_buffer(&self, path: PathBuf) -> ports::BoxFuture<'static, Result<buffer_ports::BufferId, buffer_ports::BufferError>> {
        let key = format!("buf:{}", path.to_string_lossy());
        let k_clone = key.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut m = inner.lock().unwrap();
            m.entry(k_clone.clone()).or_insert_with(|| "fn main() {}".to_string());
            Ok(buffer_ports::BufferId(key))
        })
    }

    fn get_text(&self, id: &buffer_ports::BufferId) -> Option<String> {
        let m = self.inner.lock().unwrap();
        m.get(&id.0).cloned()
    }

    fn set_text(&self, id: &buffer_ports::BufferId, content: String) -> ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
        let key = id.0.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut m = inner.lock().unwrap();
            if m.contains_key(&key) {
                m.insert(key, content);
                Ok(())
            } else {
                Err(buffer_ports::BufferError("buffer not found".to_string()))
            }
        })
    }
}

/// Fake AI (not used heavily in these tests)
struct FakeAi;
impl ai_ports::AiClient for FakeAi {
    fn request(&self, _req: ai_ports::AiRequest) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
        Box::pin(async move {
            Ok(ai_ports::AiResponseDTO { text: "ok".to_string() })
        })
    }
}

#[tokio::test]
async fn save_and_load_roundtrip() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let history = Arc::new(zaroxi_infrastructure_memory::InMemoryHistoryStore::new()) as Arc<dyn ports::HistoryRepository>;
    let checkpoint_store = zaroxi_infrastructure_memory::InMemoryCheckpointStore::new();
    let checkpoint_dyn = zaroxi_infrastructure_memory::into_checkpoint_store(checkpoint_store);

    let orchestrator = WorkspaceOrchestrator::new_with_history_and_durability(repo.clone(), store.clone(), ai.clone(), history.clone(), checkpoint_dyn.clone());

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    // Open a buffer so checkpoint contains some state
    let open = ports::OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let _open_res = orchestrator.open_buffer(open).await.expect("open ok");

    // Save checkpoint
    let save_res = orchestrator.save_checkpoint(SaveCheckpointRequest { session_id: boot_res.session.session_id.clone() }).await.expect("save ok");
    assert!(!save_res.location.is_empty());

    // Compose a fresh orchestrator that will load the checkpoint
    let repo2 = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store2 = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai2 = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;
    let history2 = Arc::new(zaroxi_infrastructure_memory::InMemoryHistoryStore::new()) as Arc<dyn ports::HistoryRepository>;
    let orchestrator2 = WorkspaceOrchestrator::new_with_history_and_durability(repo2, store2.clone(), ai2, history2, checkpoint_dyn.clone());

    let load_res = orchestrator2.load_checkpoint(LoadCheckpointRequest { location: save_res.location.clone() }).await.expect("load ok");
    assert!(load_res.session.session_id.0.to_string().len() > 0);

    // Snapshot the restored session
    let snap = orchestrator2.get_session_snapshot(ports::GetSessionSnapshotRequest { session_id: load_res.session.session_id.clone(), recent_limit: 10 }).await.expect("snapshot ok").snapshot;
    assert!(snap.opened_buffers.len() >= 1);
}

#[tokio::test]
async fn malformed_data_fails() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let history = Arc::new(zaroxi_infrastructure_memory::InMemoryHistoryStore::new()) as Arc<dyn ports::HistoryRepository>;
    let checkpoint_store = zaroxi_infrastructure_memory::InMemoryCheckpointStore::new();
    // Insert malformed bytes under an id
    let bad_id = "bad-checkpoint".to_string();
    checkpoint_store.insert_raw(bad_id.clone(), b"not-a-json".to_vec());
    let checkpoint_dyn = zaroxi_infrastructure_memory::into_checkpoint_store(checkpoint_store);

    let orchestrator = WorkspaceOrchestrator::new_with_history_and_durability(repo, store, ai, history, checkpoint_dyn.clone());

    let err = orchestrator.load_checkpoint(LoadCheckpointRequest { location: bad_id.clone() }).await.expect_err("should fail");
    assert!(err.to_string().contains("invalid checkpoint") || err.to_string().contains("malformed"));
}
