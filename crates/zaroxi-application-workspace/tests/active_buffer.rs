use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use zaroxi_application_ai::ports as ai_ports;
use zaroxi_application_workspace::ports;
use zaroxi_application_workspace::ports::WorkspaceService;
use zaroxi_application_workspace::ports::{
    GetActiveBufferRequest, ListBuffersRequest, OpenBufferRequest, SetActiveBufferRequest,
    WorkspaceBootRequest,
};
use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_core_editor_buffer::ports as buffer_ports;
use zaroxi_domain_workspace::ports as domain_ports;
use zaroxi_kernel_types::Id;

/// Fake domain repo
struct FakeRepo;
impl domain_ports::WorkspaceRepository for FakeRepo {
    fn open_workspace(
        &self,
        cmd: domain_ports::WorkspaceOpenCommand,
    ) -> ports::BoxFuture<'static, Result<domain_ports::WorkspaceDTO, domain_ports::DomainError>>
    {
        Box::pin(async move {
            Ok(domain_ports::WorkspaceDTO {
                id: Id::new(),
                root_path: cmd.path.clone(),
                name: "Test".to_string(),
            })
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
    fn open_buffer(
        &self,
        path: PathBuf,
    ) -> ports::BoxFuture<'static, Result<buffer_ports::BufferId, buffer_ports::BufferError>> {
        // Use the canonical helper from core to build BufferId from a PathBuf.
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

    fn set_text(
        &self,
        id: &buffer_ports::BufferId,
        content: String,
    ) -> ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
        let key = id.0.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut m = inner.lock().unwrap();
            if let std::collections::hash_map::Entry::Occupied(mut e) = m.entry(key) {
                e.insert(content);
                Ok(())
            } else {
                Err(buffer_ports::BufferError("buffer not found".to_string()))
            }
        })
    }

    fn apply_transaction(
        &self,
        id: &buffer_ports::BufferId,
        txn: buffer_ports::TextEdit,
    ) -> ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
        let key = id.0.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut m = inner.lock().unwrap();
            let s =
                m.get_mut(&key).ok_or(buffer_ports::BufferError("buffer not found".to_string()))?;
            let char_to_byte = |st: &str, idx: usize| -> usize {
                st.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(st.len())
            };
            match txn {
                buffer_ports::TextEdit::Insert { index, text } => {
                    let bpos = char_to_byte(s, index);
                    s.insert_str(bpos, &text);
                    Ok(())
                }
                buffer_ports::TextEdit::Delete { start, end } => {
                    let bstart = char_to_byte(s, start);
                    let bend = char_to_byte(s, end);
                    if bstart <= bend && bend <= s.len() {
                        s.replace_range(bstart..bend, "");
                        Ok(())
                    } else {
                        Err(buffer_ports::BufferError("invalid delete range".to_string()))
                    }
                }
                buffer_ports::TextEdit::Replace { start, end, text } => {
                    let bstart = char_to_byte(s, start);
                    let bend = char_to_byte(s, end);
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

/// Fake AI that echoes the content snapshot.
struct FakeAi;
impl ai_ports::AiClient for FakeAi {
    fn request(
        &self,
        req: ai_ports::AiRequest,
    ) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
        Box::pin(
            async move { Ok(ai_ports::AiResponseDTO { text: format!("echo: {}", req.buffer_id) }) },
        )
    }
}

#[tokio::test]
async fn open_multiple_and_switch_active() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    // Open two buffers
    let open1 = OpenBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        path: PathBuf::from("a.rs"),
    };
    let a = orchestrator.open_buffer(open1).await.expect("open a");
    let open2 = OpenBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        path: PathBuf::from("b.rs"),
    };
    let b = orchestrator.open_buffer(open2).await.expect("open b");

    // List buffers - active should be the first opened (a.rs)
    let list = ListBuffersRequest { session_id: boot_res.session.session_id.clone() };
    let list_res = orchestrator.list_open_buffers(list).await.expect("list ok");
    assert_eq!(list_res.buffer_ids.len(), 2);
    assert_eq!(list_res.active_buffer.unwrap(), a.buffer_id);

    // Switch active to b
    let set_active = SetActiveBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        buffer_id: b.buffer_id.clone(),
    };
    let set_res = orchestrator.set_active_buffer(set_active).await.expect("set active");
    assert!(set_res.ok);

    // Confirm active
    let get_active = GetActiveBufferRequest { session_id: boot_res.session.session_id.clone() };
    let active_res = orchestrator.get_active_buffer(get_active).await.expect("get active");
    assert_eq!(active_res.buffer_id, b.buffer_id);

    // Explain active buffer: should use b
    let explain_req = GetActiveBufferRequest { session_id: boot_res.session.session_id.clone() };
    let explain_res = orchestrator.explain_active_buffer(explain_req).await.expect("explain ok");
    assert!(explain_res.result.message.contains(b.buffer_id.as_str()));
}

#[tokio::test]
async fn explain_fails_when_no_active() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    // Do not open any buffer. Explaining should return NoActiveBuffer.
    let explain_req = GetActiveBufferRequest { session_id: boot_res.session.session_id.clone() };
    let err = orchestrator.explain_active_buffer(explain_req).await.expect_err("should fail");
    assert!(err.to_string().contains("no active buffer"));
}
