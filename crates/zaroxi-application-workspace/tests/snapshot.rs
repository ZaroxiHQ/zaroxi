use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, UpdateBufferRequest, SetActiveBufferRequest,
    GetSessionSnapshotRequest,
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
async fn snapshot_reflects_opened_buffers_and_active() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    // Open two buffers
    let open1 = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("a.rs") };
    let a = orchestrator.open_buffer(open1).await.expect("open a");
    let open2 = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("b.rs") };
    let b = orchestrator.open_buffer(open2).await.expect("open b");

    // Snapshot
    let snap_req = GetSessionSnapshotRequest { session_id: boot_res.session.session_id.clone(), recent_limit: 5 };
    let snap = orchestrator.get_session_snapshot(snap_req).await.expect("snapshot ok").snapshot;

    assert_eq!(snap.opened_buffers.len(), 2);
    assert_eq!(snap.active_buffer.unwrap(), a.buffer_id);
    // contents present for opened buffers
    assert!(snap.buffers.iter().any(|bs| bs.buffer_id == a.buffer_id && bs.content.is_some()));
    assert!(snap.buffers.iter().any(|bs| bs.buffer_id == b.buffer_id && bs.content.is_some()));
}

#[tokio::test]
async fn snapshot_reflects_active_change_and_mutation() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open1 = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open1).await.expect("open ok");

    // Update content
    let new_content = "fn main() { println!(\"mutated\"); }".to_string();
    let update = UpdateBufferRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone(), new_content: new_content.clone() };
    let _ = orchestrator.update_buffer(update).await.expect("update ok");

    // Open another buffer and set active to it, then snapshot
    let open2 = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("lib.rs") };
    let b = orchestrator.open_buffer(open2).await.expect("open b");
    let set_active = SetActiveBufferRequest { session_id: boot_res.session.session_id.clone(), buffer_id: b.buffer_id.clone() };
    let _ = orchestrator.set_active_buffer(set_active).await.expect("set active");

    let snap_req = GetSessionSnapshotRequest { session_id: boot_res.session.session_id.clone(), recent_limit: 5 };
    let snap = orchestrator.get_session_snapshot(snap_req).await.expect("snapshot ok").snapshot;

    // Active should be b
    assert_eq!(snap.active_buffer.unwrap(), b.buffer_id);
    // main.rs content should include mutated text
    let main_snapshot = snap.buffers.iter().find(|bs| bs.buffer_id == open_res.buffer_id).expect("main present");
    assert!(main_snapshot.content.as_ref().unwrap().contains("mutated"));
}

#[tokio::test]
async fn snapshot_unknown_session_fails() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store, ai);

    let bogus_session = ports::SessionId(Id::new());
    let req = GetSessionSnapshotRequest { session_id: bogus_session, recent_limit: 5 };
    let err = orchestrator.get_session_snapshot(req).await.expect_err("should fail");
    assert!(err.to_string().contains("unknown session"));
}
