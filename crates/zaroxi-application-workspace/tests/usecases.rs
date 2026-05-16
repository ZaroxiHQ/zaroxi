use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, DispatchCommandRequest, AppCommand,
};
use zaroxi_application_workspace::ports as ports;
use zaroxi_application_workspace::ports::{WorkspaceService, WorkspaceView};
use zaroxi_domain_workspace::ports as domain_ports;
use zaroxi_core_editor_buffer::ports as buffer_ports;
use zaroxi_application_ai::ports as ai_ports;
use zaroxi_kernel_types::Id;

/// Simple integration-style unit test using in-crate test doubles.
#[tokio::test]
async fn orchestrator_flow_happy_path() {
    // Fake repo
    struct FakeRepo;
    impl domain_ports::WorkspaceRepository for FakeRepo {
        fn open_workspace(&self, cmd: domain_ports::WorkspaceOpenCommand) -> ports::BoxFuture<'static, Result<domain_ports::WorkspaceDTO, domain_ports::DomainError>> {
            Box::pin(async move {
                Ok(domain_ports::WorkspaceDTO { id: Id::new(), root_path: cmd.path.clone(), name: "Test".to_string() })
            })
        }
    }

    /// Simple in-test buffer store supporting open/get/set/apply_transaction
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

        fn apply_transaction(&self, id: &buffer_ports::BufferId, txn: buffer_ports::TextEdit) -> ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
            let key = id.0.clone();
            let inner = self.inner.clone();
            Box::pin(async move {
                let mut m = inner.lock().unwrap();
                let s = m.get_mut(&key).ok_or(buffer_ports::BufferError("buffer not found".to_string()))?;
                let char_to_byte = |st: &str, idx: usize| -> usize {
                    st.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(st.len())
                };
                match txn {
                    buffer_ports::TextEdit::Insert { index, text } => {
                        let bpos = char_to_byte(&s, index);
                        s.insert_str(bpos, &text);
                        Ok(())
                    }
                    buffer_ports::TextEdit::Delete { start, end } => {
                        let bstart = char_to_byte(&s, start);
                        let bend = char_to_byte(&s, end);
                        if bstart <= bend && bend <= s.len() {
                            s.replace_range(bstart..bend, "");
                            Ok(())
                        } else {
                            Err(buffer_ports::BufferError("invalid delete range".to_string()))
                        }
                    }
                    buffer_ports::TextEdit::Replace { start, end, text } => {
                        let bstart = char_to_byte(&s, start);
                        let bend = char_to_byte(&s, end);
                        if bstart <= bend && bend <= s.len() {
                            s.replace_range(bstart..bend, &text);
                            Ok(())
                        } else {
                            Err(buffer_ports::BufferError("invalid replace range".to_string()))
                        }
                    }
                }
            })
        }
    }

    // Fake AI
    struct FakeAi;
    impl ai_ports::AiClient for FakeAi {
        fn request(&self, req: ai_ports::AiRequest) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
            Box::pin(async move {
                Ok(ai_ports::AiResponseDTO { text: format!("fake: {}", req.buffer_id) })
            })
        }
    }

    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store, ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");
    // session id is now typed; ensure it's present.
    assert!(boot_res.session.session_id.0.to_string().len() > 0);

    let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");
    // Prefer typed assertion: the BufferId is expected to map to a filesystem path.
    assert!(open_res.buffer_id.path().is_some());

    // Phase 2: verify view seam can read buffer content
    let content = orchestrator.get_buffer_content(open_res.buffer_id.clone()).await.expect("read ok");
    assert!(content.is_some());
    assert!(content.unwrap().contains("fn main"));

    let dispatch = DispatchCommandRequest { session_id: boot_res.session.session_id.clone(), command: AppCommand::AiExplain { buffer_id: open_res.buffer_id.clone() } };
    let dispatch_res = orchestrator.dispatch_command(dispatch).await.expect("dispatch ok");
    assert!(dispatch_res.result.message.contains("fake:"));
}
