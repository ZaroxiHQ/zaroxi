use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, GetRecentCommandsRequest, GetRecentEventsRequest,
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

/// Fake AI that echoes the content snapshot.
struct FakeAi;
impl ai_ports::AiClient for FakeAi {
    fn request(&self, req: ai_ports::AiRequest) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
        Box::pin(async move {
            Ok(ai_ports::AiResponseDTO { text: format!("echo: {}", req.buffer_id) })
        })
    }
}

#[tokio::test]
async fn history_and_events_recorded() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    // Provide an in-test history implementation that records into memory
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
                Ok(v.into_iter().filter(|r| r.session_id.as_ref().map(|s| s == &session_id).unwrap_or(false)).take(limit).collect())
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

    // Query recent commands and events and assert
    let cmds = orchestrator.get_recent_commands(GetRecentCommandsRequest { session_id: boot_res.session.session_id.clone(), limit: 10 }).await.expect("query cmds");
    assert!(cmds.commands.iter().any(|c| matches!(c.kind, ports::CommandKind::BootWorkspace {..})));
    assert!(cmds.commands.iter().any(|c| matches!(c.kind, ports::CommandKind::OpenBuffer {..})));

    let evs = orchestrator.get_recent_events(GetRecentEventsRequest { session_id: boot_res.session.session_id.clone(), limit: 10 }).await.expect("query evs");
    assert!(evs.events.iter().any(|e| matches!(e.kind, ports::WorkspaceEventKind::SessionOpened {..})));
    assert!(evs.events.iter().any(|e| matches!(e.kind, ports::WorkspaceEventKind::BufferOpened {..})));
}
