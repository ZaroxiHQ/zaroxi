use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use zaroxi_application_ai::ports as ai_ports;
use zaroxi_application_workspace::ports;
use zaroxi_application_workspace::ports::WorkspaceService;
use zaroxi_application_workspace::ports::{
    AppCommand, DispatchCommandRequest, EditorCursor, OpenBufferRequest, Selection,
    SetEditorCursorRequest, SetSelectionRequest, WorkspaceBootRequest,
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

/// Simple in-test buffer store supporting set_text/get_text/open_buffer/apply_transaction
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

/// Fake AI (not used)
struct FakeAi;
impl ai_ports::AiClient for FakeAi {
    fn request(
        &self,
        _req: ai_ports::AiRequest,
    ) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
        Box::pin(async move { Ok(ai_ports::AiResponseDTO { text: "ok".to_string() }) })
    }
}

#[tokio::test]
async fn insert_text_at_cursor_advances_cursor_and_updates_content() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open = OpenBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        path: PathBuf::from("main.rs"),
    };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");

    // Set cursor at start
    let cursor = EditorCursor { line: 0, column: 0 };
    let _ = orchestrator
        .set_editor_cursor(SetEditorCursorRequest {
            session_id: boot_res.session.session_id.clone(),
            buffer_id: open_res.buffer_id.clone(),
            cursor: cursor.clone(),
        })
        .await
        .expect("set cursor ok");

    // Dispatch insert command
    let cmd = DispatchCommandRequest {
        session_id: boot_res.session.session_id.clone(),
        command: AppCommand::InsertText {
            buffer_id: open_res.buffer_id.clone(),
            text: "hello ".to_string(),
        },
    };
    let res = orchestrator.dispatch_command(cmd).await.expect("dispatch ok");
    assert!(res.result.message.contains("inserted"));

    // Ensure content updated
    let content = store.get_text(&open_res.buffer_id).unwrap();
    assert!(content.starts_with("hello "));

    // Editor cursor should have advanced by 6 characters
    let st = orchestrator
        .get_editor_state(crate::ports::GetEditorStateRequest {
            session_id: boot_res.session.session_id.clone(),
            buffer_id: open_res.buffer_id.clone(),
        })
        .await
        .expect("get state ok");
    let state = st.state.expect("state present");
    assert_eq!(state.cursor.column, 6);
}

#[tokio::test]
async fn replace_selection_replaces_selected_text() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open = OpenBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        path: PathBuf::from("main.rs"),
    };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");

    // Ensure buffer has known content
    store.set_text(&open_res.buffer_id, "abcd efgh".to_string()).await.expect("set ok");

    // Set a selection from char 5..9 ("efgh")
    let sel = Selection {
        anchor: EditorCursor { line: 0, column: 5 },
        active: EditorCursor { line: 0, column: 9 },
    };
    let _ = orchestrator
        .set_editor_selection(SetSelectionRequest {
            session_id: boot_res.session.session_id.clone(),
            buffer_id: open_res.buffer_id.clone(),
            selection: sel.clone(),
        })
        .await
        .expect("set selection ok");

    // Dispatch replace selection
    let cmd = DispatchCommandRequest {
        session_id: boot_res.session.session_id.clone(),
        command: AppCommand::ReplaceSelection {
            buffer_id: open_res.buffer_id.clone(),
            text: "Z".to_string(),
        },
    };
    let res = orchestrator.dispatch_command(cmd).await.expect("dispatch ok");
    assert!(res.result.message.contains("replaced") || res.result.message.contains("inserted"));

    let content = store.get_text(&open_res.buffer_id).unwrap();
    assert!(content.contains("abcd Z"));
}

#[tokio::test]
async fn delete_selection_removes_selected_text() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open = OpenBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        path: PathBuf::from("main.rs"),
    };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");

    // Set content and selection
    store.set_text(&open_res.buffer_id, "hello world".to_string()).await.expect("set ok");
    let sel = Selection {
        anchor: EditorCursor { line: 0, column: 6 },
        active: EditorCursor { line: 0, column: 11 },
    };
    let _ = orchestrator
        .set_editor_selection(SetSelectionRequest {
            session_id: boot_res.session.session_id.clone(),
            buffer_id: open_res.buffer_id.clone(),
            selection: sel.clone(),
        })
        .await
        .expect("set selection ok");

    // Dispatch delete selection
    let cmd = DispatchCommandRequest {
        session_id: boot_res.session.session_id.clone(),
        command: AppCommand::DeleteSelection { buffer_id: open_res.buffer_id.clone() },
    };
    let res = orchestrator.dispatch_command(cmd).await.expect("dispatch ok");
    assert!(res.result.message.contains("deleted"));

    let content = store.get_text(&open_res.buffer_id).unwrap();
    assert!(content.contains("hello "));
}

#[tokio::test]
async fn indent_line_inserts_spaces_at_line_start() {
    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore::new()) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store.clone(), ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");

    let open = OpenBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        path: PathBuf::from("main.rs"),
    };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");

    // Set content with two lines
    store.set_text(&open_res.buffer_id, "line1\nline2".to_string()).await.expect("set ok");

    // Set cursor on second line
    let _ = orchestrator
        .set_editor_cursor(SetEditorCursorRequest {
            session_id: boot_res.session.session_id.clone(),
            buffer_id: open_res.buffer_id.clone(),
            cursor: EditorCursor { line: 1, column: 0 },
        })
        .await
        .expect("set cursor ok");

    // Dispatch indent
    let cmd = DispatchCommandRequest {
        session_id: boot_res.session.session_id.clone(),
        command: AppCommand::IndentLine { buffer_id: open_res.buffer_id.clone() },
    };
    let res = orchestrator.dispatch_command(cmd).await.expect("dispatch ok");
    assert!(res.result.message.contains("indented"));

    let content = store.get_text(&open_res.buffer_id).unwrap();
    // second line should now start with four spaces
    assert!(content.contains("\n    line2"));
}
