use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, UpdateBufferRequest, CreateCheckpointRequest, RestoreCheckpointRequest, GetSessionSnapshotRequest,
};
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
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl FakeStore {
    fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }
}

impl buffer_ports::BufferStore for FakeStore {
    fn open_buffer(&self, path: PathBuf) -> ports::BoxFuture<'static, Result<buffer_ports::BufferId, buffer_ports::BufferError>> {
        let id = buffer_ports::BufferId::from_path(&path);
        let key = id.0.clone();
        let id_clone = id.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut m = inner.lock().unwrap();
            m.entry(key.clone()).or_insert_with(|| "fn main() {}".to_string());
            Ok(id_clone)
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
async fn checkpoint_create_and_restore() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    // History used by first orchestrator so checkpoint captures some records.
    struct TestHistory {
        cmds: Arc<Mutex<Vec<ports::CommandRecord>>>,
        evs: Arc<Mutex<Vec<ports::WorkspaceEvent>>>,
    }

    impl TestHistory {
        fn new() -> Self {
            Self { cmds: Arc::new(Mutex::new(Vec::new())), evs: Arc::new(Mutex::new(Vec::new())) }
        }
    }

    impl ports::HistoryRepository for TestHistory {
        fn record_command(&self, rec: ports::CommandRecord) -> ports::BoxFuture<'static, Result<(), String>> {
            let c = self.cmds.clone();
            Box::pin(async move {
                c.lock().unwrap().push(rec);
                Ok(())
            })
        }

        fn record_event(&self, ev: ports::WorkspaceEvent) -> ports::BoxFuture<'static, Result<(), String>> {
            let e = self.evs.clone();
            Box::pin(async move {
                e.lock().unwrap().push(ev);
                Ok(())
            })
        }

        fn get_recent_commands(&self, session_id: ports::SessionId, limit: usize) -> ports::BoxFuture<'static, Result<Vec<ports::CommandRecord>, String>> {
            let c = self.cmds.clone();
            Box::pin(async move {
                let v = c.lock().unwrap().clone();
                Ok(v.into_iter().filter(|r| r.session_id.as_ref().map(|s| s == &session_id.0).unwrap_or(false)).take(limit).collect())
            })
        }

        fn get_recent_events(&self, session_id: ports::SessionId, limit: usize) -> ports::BoxFuture<'static, Result<Vec<ports::WorkspaceEvent>, String>> {
            let e = self.evs.clone();
            Box::pin(async move {
                let v = e.lock().unwrap().clone();
                Ok(v.into_iter().filter(|ev| ev.session_id == session_id).take(limit).collect())
            })
        }
    }

    let hist = Arc::new(TestHistory::new()) as Arc<dyn ports::HistoryRepository>;
    let orchestrator = WorkspaceOrchestrator::new_with_history(repo, store.clone(), ai, hist.clone());

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");

    let new_content = "fn main() { println!(\"checkpointed\"); }".to_string();
    let update = UpdateBufferRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone(), new_content: new_content.clone() };
    let _ = orchestrator.update_buffer(update).await.expect("update ok");

    // Create checkpoint
    let cp_req = CreateCheckpointRequest { session_id: boot_res.session.session_id.clone() };
    let cp_res = orchestrator.create_checkpoint(cp_req).await.expect("checkpoint created");
    let checkpoint = cp_res.checkpoint.clone();

    // Restore into a fresh orchestrator with fresh infra instances.
    let repo2 = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store2 = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai2 = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;
    let hist2 = Arc::new(TestHistory::new()) as Arc<dyn ports::HistoryRepository>;

    let orchestrator2 = WorkspaceOrchestrator::new_with_history(repo2, store2.clone(), ai2, hist2);

    let restore_req = RestoreCheckpointRequest { checkpoint: checkpoint.clone() };
    let restore_res = orchestrator2.restore_checkpoint(restore_req).await.expect("restore ok");
    // Ensure restored session id matches checkpoint
    assert_eq!(restore_res.session.session_id, checkpoint.session_id);

    // Snapshot the restored session and ensure buffers/content match.
    let snap = orchestrator2.get_session_snapshot(GetSessionSnapshotRequest { session_id: checkpoint.session_id.clone(), recent_limit: 10 }).await.expect("snapshot ok").snapshot;
    assert_eq!(snap.opened_buffers.len(), checkpoint.opened_buffers.len());
    assert_eq!(snap.active_buffer, checkpoint.active_buffer);
    let main_snap = snap.buffers.iter().find(|b| b.buffer_id == open_res.buffer_id).expect("main present");
    assert!(main_snap.content.as_ref().unwrap().contains("checkpointed"));
}

#[tokio::test]
async fn restore_rejects_invalid_checkpoint() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store, ai);

    // Build a malformed checkpoint (invalid buffer id)
    let bad_cp = ports::Checkpoint {
        version: 1,
        session_id: ports::SessionId(Id::new()),
        workspace_id: Id::new(),
        opened_buffers: vec![zaroxi_core_editor_buffer::ports::BufferId::from("badid")],
        active_buffer: None,
        buffers: vec![],
        recent_commands: vec![],
        recent_events: vec![],
        created_at: chrono::Utc::now(),
    };

    let restore_req = RestoreCheckpointRequest { checkpoint: bad_cp };
    let err = orchestrator.restore_checkpoint(restore_req).await.expect_err("should fail invalid checkpoint");
    assert!(err.to_string().contains("invalid checkpoint"));
}
