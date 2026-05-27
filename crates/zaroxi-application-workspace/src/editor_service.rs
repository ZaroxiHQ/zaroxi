use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use zaroxi_core_editor_buffer::buffer::{Buffer, Selection};

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
}

impl EditorService {
    /// Construct service with a single unnamed buffer from text.
    pub fn new_with_text(text: &str) -> Self {
        let buf_arc = Arc::new(Mutex::new(Buffer::from_text(text)));
        let state =
            BuffersState { paths: vec![None], buffers: vec![buf_arc.clone()], active: Some(0) };
        Self { buffer: buf_arc, inner: Mutex::new(state) }
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
        Ok(Self { buffer: buf_arc, inner: Mutex::new(state) })
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
