use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{WorkspaceBootRequest, OpenBufferRequest, UpdateBufferRequest, DispatchCommandRequest, AppCommand};
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

/// Fake AI that echoes the content snapshot.
struct FakeAi;
impl ai_ports::AiClient for FakeAi {
    fn request(&self, req: ai_ports::AiRequest) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
        Box::pin(async move {
            Ok(ai_ports::AiResponseDTO { text: format!("echo: {}", req.content_snapshot) })
        })
    }
}

#[tokio::test]
async fn update_buffer_success_and_explain() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");

    let new_content = "fn main() { println!(\"mutated\"); }".to_string();
    let update = UpdateBufferRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone(), new_content: new_content.clone() };
    let update_res = orchestrator.update_buffer(update).await.expect("update ok");
    assert!(update_res.ok);

    // Ensure store has new content
    let content = store.get_text(&open_res.buffer_id).unwrap();
    assert!(content.contains("mutated"));

    // Dispatch AI explain and ensure it sees new content
    let dispatch = DispatchCommandRequest { session_id: boot_res.session.session_id.clone(), command: AppCommand::AiExplain { buffer_id: open_res.buffer_id.clone() } };
    let dispatch_res = orchestrator.dispatch_command(dispatch).await.expect("dispatch ok");
    assert!(dispatch_res.result.message.contains("mutated"));
}

#[tokio::test]
async fn update_buffer_invalid_rejected() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store, ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");

    // Invalid (empty) content
    let update = UpdateBufferRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone(), new_content: "   ".to_string() };
    let err = orchestrator.update_buffer(update).await.expect_err("should fail");
    assert!(err.to_string().contains("invalid mutation"));
}

#[tokio::test]
async fn update_buffer_unknown_session() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store, ai);

    let bogus_session = ports::SessionId(Id::new());
    let update = UpdateBufferRequest { session_id: bogus_session, buffer_id: buffer_ports::BufferId::from("buf:main.rs"), new_content: "fn main() {}".to_string() };
    let err = orchestrator.update_buffer(update).await.expect_err("should fail unknown session");
    assert!(err.to_string().contains("unknown session"));
}
