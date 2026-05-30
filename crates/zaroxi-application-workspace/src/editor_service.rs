use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use zaroxi_core_editor_buffer::buffer::{Buffer, Selection};
use zaroxi_core_workspace_files::FileStorage;

/// Public snapshot type that the interface can consume to build presenter editor layout.
///
/// Lines are 0-based internally; the presenter EditorLayoutSpec uses 1-based
/// document lines, so the interface layer will map these fields accordingly.
#[derive(Debug, Clone)]
pub struct EditorSnapshot {
    pub lines: Vec<String>,
    /// top visible document line (1-based for presenter convenience). For our
    /// snapshot 1-based means 1 = first line.
    pub top_line: u32,
    pub cursor_line: Option<u32>,
    pub cursor_column: Option<u32>,
    /// Optional selection as (start_line, start_col, end_line, end_col) all 1-based
    pub selection: Option<(u32, u32, u32, u32)>,
    /// Whether the buffer contains unsaved edits.
    pub dirty: bool,
}

/// Result of attempting to close a buffer.
#[derive(Debug, PartialEq, Eq)]
pub enum CloseResult {
    Closed,
    BlockedByDirty,
    BufferNotFound,
}

/// Result of attempting to resolve a blocked dirty-close.
///
/// - ClosedAfterSave: buffer was saved to disk (saved state updated) and then closed.
/// - ClosedAfterDiscard: in-memory edits were discarded (replaced by on-disk content or cleared)
///   and the buffer was closed.
/// - SaveFailed(e): attempted save failed with an IO error; buffer remains open.
/// - IoError(e): generic IO error (e.g. failed to read on discard).
/// - BufferNotFound: no opened buffer matched the provided path.
/// - NotDirty: the target buffer was not dirty (no resolution needed).
#[derive(Debug)]
pub enum ResolveDirtyCloseResult {
    ClosedAfterSave,
    ClosedAfterDiscard,
    SaveFailed(std::io::Error),
    IoError(std::io::Error),
    BufferNotFound,
    NotDirty,
}

/// Result of attempting to reload a buffer from disk.
///
/// - Reloaded: buffer was reloaded from disk (and marked clean).
/// - BlockedByDirty: buffer had unsaved edits and reload was refused.
/// - IoError: underlying IO/read/write failure occurred.
/// - BufferNotFound: no opened buffer matched the provided path.
#[derive(Debug)]
pub enum ReloadResult {
    Reloaded,
    BlockedByDirty,
    IoError(std::io::Error),
    BufferNotFound,
}

/// Result of attempting to close an entire workspace/session.
///
/// - Closed: session had no dirty buffers and was closed (buffers removed).
/// - BlockedByDirty: one or more buffers are dirty; caller may resolve via
///   resolve_close_session_save_all / resolve_close_session_discard_all.
///   Each tuple is (buffer_index, Option<PathBuf>) where None indicates an unnamed buffer.
/// - SessionNotFound: (reserved) no session/workspace found (keeps parity with single-buffer APIs).
#[derive(Debug, PartialEq, Eq)]
pub enum AttemptCloseSessionResult {
    Closed,
    BlockedByDirty { dirty_buffers: Vec<(usize, Option<PathBuf>)> },
    SessionNotFound,
}

/// Result of attempting to resolve a previously-blocked session close.
#[derive(Debug)]
pub enum ResolveCloseSessionResult {
    ClosedAfterSaveAll,
    ClosedAfterDiscardAll,
    /// Save-all failed; failed_buffers contains tuples (buffer_index, Option<PathBuf>)
    /// with None indicating unnamed buffers that could not be saved.
    SaveAllFailed {
        failed_buffers: Vec<(usize, Option<PathBuf>)>,
    },
    /// IO error while attempting to discard (reload) buffers from disk.
    IoError(std::io::Error),
    SessionNotFound,
    /// Nothing to resolve (no dirty buffers).
    NothingToResolve,
}

/// Internal state holding opened buffers and their optional file paths.
/// All indices correspond between paths and buffers vecs.
struct BuffersState {
    paths: Vec<Option<PathBuf>>,
    buffers: Vec<Arc<Mutex<Buffer>>>,
    active: Option<usize>,
}

/// EditorService now manages multiple opened buffers in a small, deterministic
/// workspace model while preserving the previous single-buffer convenience API.
pub struct EditorService {
    /// For backward compatibility this field still exists and refers to the
    /// initially-created buffer Arc. Consumers should prefer the service
    /// methods which reflect the actual active buffer state.
    pub buffer: Arc<Mutex<Buffer>>,

    /// Internal multi-buffer state protected by a mutex.
    inner: Mutex<BuffersState>,

    /// Optional storage adapter for filesystem operations. Injected by composition (harness).
    pub storage: Option<Arc<dyn FileStorage>>,
}

impl EditorService {
    /// Construct service with a single unnamed buffer from text.
    pub fn new_with_text(text: &str) -> Self {
        let buf_arc = Arc::new(Mutex::new(Buffer::from_text(text)));
        let state =
            BuffersState { paths: vec![None], buffers: vec![buf_arc.clone()], active: Some(0) };
        Self { buffer: buf_arc, inner: Mutex::new(state), storage: None }
    }

    /// Create an EditorService by loading file contents from path.
    pub fn new_from_file(path: &Path) -> io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let buf_arc = Arc::new(Mutex::new(Buffer::from_text(&content)));
        let state = BuffersState {
            paths: vec![Some(path.to_path_buf())],
            buffers: vec![buf_arc.clone()],
            active: Some(0),
        };
        Ok(Self { buffer: buf_arc, inner: Mutex::new(state), storage: None })
    }

    /// Attach a storage adapter for subsequent save/load operations.
    pub fn set_storage(&mut self, storage: Option<Arc<dyn FileStorage>>) {
        self.storage = storage;
    }

    /// Save a specific buffer (by index) to its associated filesystem path using the configured storage.
    /// Returns Ok(()) on success or an io::Error describing the failure.
    pub fn save_buffer(&self, index: usize) -> io::Result<()> {
        let storage: Arc<dyn zaroxi_core_workspace_files::FileStorage> = match &self.storage {
            Some(s) => s.clone(),
            // Fallback to a disk-backed implementation when no adapter was injected.
            // This keeps tests and simple harnesses convenient while allowing harnesses
            // to inject mocks in more advanced scenarios.
            None => Arc::new(zaroxi_core_workspace_files::DiskFileStorage::new()),
        };

        // Snapshot required state (path + buffer arc) under the inner mutex.
        let (path_opt, buf_arc) = {
            let state = self.inner.lock().unwrap();
            if index >= state.buffers.len() {
                return Err(io::Error::new(io::ErrorKind::NotFound, "buffer index out of range"));
            }
            (state.paths[index].clone(), state.buffers[index].clone())
        };

        let path = match path_opt {
            Some(p) => p,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "target buffer has no associated filesystem path",
                ));
            }
        };

        // Read buffer text while holding the buffer lock, then perform IO.
        let text = {
            let buf = buf_arc.lock().unwrap();
            buf.to_text()
        };

        storage.write_file(&path, &text)?;

        // On success update saved_text and clear dirty flag.
        {
            let mut buf = buf_arc.lock().unwrap();
            buf.saved_text = Some(text);
            buf.dirty = false;
        }

        Ok(())
    }

    /// Save the currently active buffer (if any).
    pub fn save_active_buffer(&self) -> io::Result<()> {
        let index = {
            let state = self.inner.lock().unwrap();
            match state.active {
                Some(i) => i,
                None => {
                    return Err(io::Error::new(io::ErrorKind::Other, "no active buffer"));
                }
            }
        };
        self.save_buffer(index)
    }

    /// Save all dirty buffers that have associated filesystem paths using the configured storage.
    /// Returns the number of buffers successfully saved.
    pub fn save_all_buffers(&self) -> io::Result<usize> {
        let storage: Arc<dyn zaroxi_core_workspace_files::FileStorage> = match &self.storage {
            Some(s) => s.clone(),
            // Default to the disk-backed storage when no adapter was provided.
            None => Arc::new(zaroxi_core_workspace_files::DiskFileStorage::new()),
        };

        // Collect indices to save to avoid holding the inner mutex during IO.
        let to_save: Vec<(usize, PathBuf, Arc<Mutex<Buffer>>)> = {
            let state = self.inner.lock().unwrap();
            state
                .paths
                .iter()
                .enumerate()
                .filter_map(|(idx, path_opt)| {
                    if let Some(path) = path_opt {
                        let buf_arc = state.buffers[idx].clone();
                        let dirty = {
                            let buf = buf_arc.lock().unwrap();
                            buf.dirty
                        };
                        if dirty {
                            return Some((idx, path.clone(), buf_arc));
                        }
                    }
                    None
                })
                .collect()
        };

        let mut saved_count = 0usize;
        for (_idx, path, buf_arc) in to_save {
            let text = {
                let buf = buf_arc.lock().unwrap();
                buf.to_text()
            };
            if let Err(e) = storage.write_file(&path, &text) {
                // Stop on first failure and return the error.
                return Err(e);
            }
            // update buffer saved state
            {
                let mut buf = buf_arc.lock().unwrap();
                buf.saved_text = Some(text);
                buf.dirty = false;
            }
            saved_count += 1;
        }

        Ok(saved_count)
    }

    /// Attempt to reload buffer from disk. If the buffer is dirty, the reload is blocked and returns BlockedByDirty.
    pub fn reload_buffer(&self, path: &Path) -> ReloadResult {
        // find index
        let idx_opt = {
            let st = self.inner.lock().unwrap();
            st.paths.iter().rposition(|p| match p {
                Some(pp) => pp == path,
                None => false,
            })
        };
        let idx = match idx_opt {
            Some(i) => i,
            None => return ReloadResult::BufferNotFound,
        };

        let buf_arc = {
            let st = self.inner.lock().unwrap();
            st.buffers[idx].clone()
        };

        {
            let b = buf_arc.lock().unwrap();
            if b.dirty {
                return ReloadResult::BlockedByDirty;
            }
        }

        // read from disk
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return ReloadResult::IoError(e),
        };

        {
            let mut b = buf_arc.lock().unwrap();
            b.load_from_text(&content);
            b.saved_text = Some(content);
            b.dirty = false;
        }

        ReloadResult::Reloaded
    }

    /// Force-reload: discard in-memory edits and replace buffer with on-disk contents.
    pub fn resolve_reload_discard(&self, path: &Path) -> ReloadResult {
        let idx_opt = {
            let st = self.inner.lock().unwrap();
            st.paths.iter().rposition(|p| match p {
                Some(pp) => pp == path,
                None => false,
            })
        };
        let idx = match idx_opt {
            Some(i) => i,
            None => return ReloadResult::BufferNotFound,
        };

        let buf_arc = {
            let st = self.inner.lock().unwrap();
            st.buffers[idx].clone()
        };

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return ReloadResult::IoError(e),
        };

        {
            let mut b = buf_arc.lock().unwrap();
            b.load_from_text(&content);
            b.saved_text = Some(content);
            b.dirty = false;
        }

        ReloadResult::Reloaded
    }

    /// Force-save current buffer to disk and then mark as saved (used to resolve reload conflicts).
    pub fn resolve_reload_save(&self, path: &Path) -> ReloadResult {
        let idx_opt = {
            let st = self.inner.lock().unwrap();
            st.paths.iter().rposition(|p| match p {
                Some(pp) => pp == path,
                None => false,
            })
        };
        let idx = match idx_opt {
            Some(i) => i,
            None => return ReloadResult::BufferNotFound,
        };

        let buf_arc = {
            let st = self.inner.lock().unwrap();
            st.buffers[idx].clone()
        };

        let text = {
            let b = buf_arc.lock().unwrap();
            b.to_text()
        };

        if let Err(e) = std::fs::write(path, text.as_bytes()) {
            return ReloadResult::IoError(e);
        }

        {
            let mut b = buf_arc.lock().unwrap();
            b.set_saved_state();
        }

        ReloadResult::Reloaded
    }
}

// Submodules split for maintainability (behavior preserved).
mod close;
mod dirty_close;
mod editing;
mod management;
mod persistence;
mod session_close;
mod snapshot;
