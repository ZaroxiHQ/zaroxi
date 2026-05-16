use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, GetEditorStateRequest, SetEditorCursorRequest, SetSelectionRequest,
    EditorCursor, Selection,
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

/// Simple in-test buffer store supporting open_buffer/get_text/set_text (minimal)
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

/// Fake AI (not used)
struct FakeAi;
impl ai_ports::AiClient for FakeAi {
    fn request(&self, _req: ai_ports::AiRequest) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
        Box::pin(async move {
            Ok(ai_ports::AiResponseDTO { text: "ok".to_string() })
        })
    }
}

#[tokio::test]
async fn set_and_get_cursor_and_selection() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");

    // Set cursor
    let cursor = EditorCursor { line: 3, column: 5 };
    let set_res = orchestrator.set_editor_cursor(SetEditorCursorRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone(), cursor: cursor.clone() }).await.expect("set cursor ok");
    assert!(set_res.ok);

    // Get editor state and verify cursor
    let get_res = orchestrator.get_editor_state(GetEditorStateRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone() }).await.expect("get state ok");
    let state = get_res.state.expect("state present");
    assert_eq!(state.cursor, cursor);

    // Set selection
    let sel = Selection { anchor: EditorCursor { line: 1, column: 0 }, active: EditorCursor { line: 1, column: 10 } };
    let sel_res = orchestrator.set_editor_selection(SetSelectionRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone(), selection: sel.clone() }).await.expect("set selection ok");
    assert!(sel_res.ok);

    let get_res2 = orchestrator.get_editor_state(GetEditorStateRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone() }).await.expect("get state ok");
    let state2 = get_res2.state.expect("state present");
    assert_eq!(state2.selection.unwrap(), sel);

    // Clear selection
    let clear_res = orchestrator.clear_editor_selection(crate::ports::ClearSelectionRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone() }).await.expect("clear ok");
    assert!(clear_res.ok);

    let get_res3 = orchestrator.get_editor_state(GetEditorStateRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open_res.buffer_id.clone() }).await.expect("get state ok");
    let state3 = get_res3.state.expect("state present");
    assert!(state3.selection.is_none());
}
