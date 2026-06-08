use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use zaroxi_kernel_types::Id;

/// Boxed future alias used across the application slice.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Domain workspace repository trait/types (owned by domain crate).
use zaroxi_domain_workspace::ports::{
    DomainError, WorkspaceDTO, WorkspaceOpenCommand, WorkspaceRepository,
};

/// Core buffer store trait/types (owned by core editor buffer crate).
use zaroxi_core_editor_buffer::ports::{BufferError, BufferId, BufferStore, TextEdit};

/// In-memory workspace repository used for tests/harness/composition in the application layer.
///
/// NOTE:
/// - This adapter intentionally lives in the application crate so it may depend on
///   domain/core ports and composition helpers without violating infra-layer rules.
/// - Keep implementation minimal and deterministic for harness/tests.
pub struct InMemoryWorkspaceRepo {}

impl InMemoryWorkspaceRepo {
    pub fn new() -> Self {
        InMemoryWorkspaceRepo {}
    }
}

impl WorkspaceRepository for InMemoryWorkspaceRepo {
    fn open_workspace(
        &self,
        cmd: WorkspaceOpenCommand,
    ) -> BoxFuture<'static, Result<WorkspaceDTO, DomainError>> {
        Box::pin(async move {
            // Minimal deterministic DTO for harness usage.
            let dto = WorkspaceDTO {
                id: Id::new(),
                root_path: cmd.path.clone(),
                name: "Sample Workspace".to_string(),
            };
            Ok(dto)
        })
    }
}

/// In-memory buffer store (application-owned composition helper).
///
/// Implements the core BufferStore port so the orchestrator can compose with a
/// concrete buffer backend in tests and the harness.
pub struct InMemoryBufferStore {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryBufferStore {
    pub fn new() -> Self {
        InMemoryBufferStore { inner: Arc::new(Mutex::new(HashMap::new())) }
    }
}

impl BufferStore for InMemoryBufferStore {
    fn open_buffer(&self, path: PathBuf) -> BoxFuture<'static, Result<BufferId, BufferError>> {
        let id = BufferId::from_path(&path);
        let key = id.0.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let content = std::fs::read_to_string(&path).unwrap_or_else(|_| String::new());
            let mut m = inner.lock().unwrap();
            m.entry(key.clone()).or_insert(content);
            Ok(id)
        })
    }

    fn get_text(&self, id: &BufferId) -> Option<String> {
        let m = self.inner.lock().unwrap();
        m.get(&id.0).cloned()
    }

    fn set_text(
        &self,
        id: &BufferId,
        content: String,
    ) -> BoxFuture<'static, Result<(), BufferError>> {
        let key = id.0.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut m = inner.lock().unwrap();
            if m.contains_key(&key) {
                m.insert(key, content);
                Ok(())
            } else {
                Err(BufferError("buffer not found".to_string()))
            }
        })
    }

    fn apply_transaction(
        &self,
        id: &BufferId,
        txn: TextEdit,
    ) -> BoxFuture<'static, Result<(), BufferError>> {
        let key = id.0.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut m = inner.lock().unwrap();
            let s = m.get_mut(&key).ok_or(BufferError("buffer not found".to_string()))?;
            // Helper: map character index -> byte index in the current string.
            let char_to_byte = |st: &str, idx: usize| -> usize {
                st.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(st.len())
            };
            match txn {
                TextEdit::Insert { index, text } => {
                    let bpos = char_to_byte(&s, index);
                    s.insert_str(bpos, &text);
                    Ok(())
                }
                TextEdit::Delete { start, end } => {
                    let bstart = char_to_byte(&s, start);
                    let bend = char_to_byte(&s, end);
                    if bstart <= bend && bend <= s.len() {
                        s.replace_range(bstart..bend, "");
                        Ok(())
                    } else {
                        Err(BufferError("invalid delete range".to_string()))
                    }
                }
                TextEdit::Replace { start, end, text } => {
                    let bstart = char_to_byte(&s, start);
                    let bend = char_to_byte(&s, end);
                    if bstart <= bend && bend <= s.len() {
                        s.replace_range(bstart..bend, &text);
                        Ok(())
                    } else {
                        Err(BufferError("invalid replace range".to_string()))
                    }
                }
            }
        })
    }
}

/// Export helpers to turn into Arc'd dyn trait objects so composition sites can easily wire adapters.
pub fn into_workspace_repo(repo: InMemoryWorkspaceRepo) -> Arc<dyn WorkspaceRepository> {
    Arc::new(repo)
}

pub fn into_buffer_store(store: InMemoryBufferStore) -> Arc<dyn BufferStore> {
    Arc::new(store)
}
