use zaroxi_core_editor_buffer::buffer::{Buffer, Selection};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::io;

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
        let state = BuffersState {
            paths: vec![None],
            buffers: vec![buf_arc.clone()],
            active: Some(0),
        };
        Self {
            buffer: buf_arc,
            inner: Mutex::new(state),
        }
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
        Ok(Self {
            buffer: buf_arc,
            inner: Mutex::new(state),
        })
    }

    // --------------------------
    // Buffer management / queries
    // --------------------------

    /// Open a file into a new buffer and activate it.
    pub fn open_file(&self, path: &Path) -> io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let buf_arc = Arc::new(Mutex::new(Buffer::from_text(&content)));
        let mut st = self.inner.lock().unwrap();
        st.paths.push(Some(path.to_path_buf()));
        st.buffers.push(buf_arc.clone());
        // Activate newly opened buffer to keep behavior deterministic for tests.
        st.active = Some(st.buffers.len() - 1);
        Ok(())
    }

    /// Return a clone of opened buffer paths (None for unnamed).
    pub fn opened_paths(&self) -> Vec<Option<PathBuf>> {
        let st = self.inner.lock().unwrap();
        st.paths.clone()
    }

    /// Return active buffer index if any.
    pub fn active_index(&self) -> Option<usize> {
        let st = self.inner.lock().unwrap();
        st.active
    }

    /// Helper: obtain the Arc<Mutex<Buffer>> for the active buffer (if any).
    fn get_active_buffer_arc(&self) -> Option<Arc<Mutex<Buffer>>> {
        let st = self.inner.lock().unwrap();
        match st.active {
            Some(i) => Some(st.buffers[i].clone()),
            None => None,
        }
    }

    // --------------------------
    // Snapshot / text helpers
    // --------------------------

    /// Snapshot for presenter consumption (adapter in interface layer will map 0-based -> 1-based).
    /// If no active buffer exists this returns an empty/none snapshot.
    pub fn snapshot(&self) -> EditorSnapshot {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let b = buf_arc.lock().unwrap();
            let cursor_line = Some(b.cursor_line as u32 + 1);
            let cursor_column = Some(b.cursor_col as u32);
            let selection = b.selection.as_ref().map(|s| {
                let (sl, sc, el, ec) = s.normalized();
                // convert to 1-based line indices for convenience in presenters
                (sl as u32 + 1, sc as u32, el as u32 + 1, ec as u32)
            });
            EditorSnapshot {
                lines: b.lines.clone(),
                top_line: 1,
                cursor_line,
                cursor_column,
                selection,
                dirty: b.dirty,
            }
        } else {
            EditorSnapshot {
                lines: Vec::new(),
                top_line: 1,
                cursor_line: None,
                cursor_column: None,
                selection: None,
                dirty: false,
            }
        }
    }

    /// Convenience test helper to read full text from active buffer.
    pub fn get_text(&self) -> String {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let b = buf_arc.lock().unwrap();
            b.to_text()
        } else {
            String::new()
        }
    }

    /// Convenience test helper to inspect selection (0-based normalized) from active buffer.
    pub fn get_selection_normalized(&self) -> Option<(usize, usize, usize, usize)> {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let b = buf_arc.lock().unwrap();
            b.selection.as_ref().map(|s| s.normalized())
        } else {
            None
        }
    }

    // --------------------------
    // Persistence (save/reload)
    // --------------------------

    /// Save current active buffer contents to the given path and mark as saved.
    /// Returns an io::Error if there is no active buffer or writing fails.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let buf_arc = match self.get_active_buffer_arc() {
            Some(a) => a,
            None => return Err(io::Error::new(io::ErrorKind::NotFound, "no active buffer")),
        };
        // copy text under lock to avoid holding lock while writing to disk
        let text = {
            let b = buf_arc.lock().unwrap();
            b.to_text()
        };
        std::fs::write(path, text.as_bytes())?;
        // update buffer saved state
        let mut b = buf_arc.lock().unwrap();
        b.set_saved_state();
        Ok(())
    }

    /// Reload active buffer contents from disk: replace buffer text and reset history.
    pub fn reload(&self, path: &Path) -> io::Result<()> {
        let content = std::fs::read_to_string(path)?;
        let buf_arc = match self.get_active_buffer_arc() {
            Some(a) => a,
            None => return Err(io::Error::new(io::ErrorKind::NotFound, "no active buffer")),
        };
        let mut b = buf_arc.lock().unwrap();
        b.load_from_text(&content);
        Ok(())
    }

    // --------------------------
    // Editing / clipboard / undo
    // --------------------------

    /// Move arrow with optional shift (expand selection).
    pub fn arrow_left(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.move_left(shift);
        }
    }
    pub fn arrow_right(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.move_right(shift);
        }
    }
    pub fn arrow_up(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.move_up(shift);
        }
    }
    pub fn arrow_down(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.move_down(shift);
        }
    }

    pub fn home(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.home(shift);
        }
    }

    pub fn end(&self, shift: bool) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.end(shift);
        }
    }

    /// Type a string (inserts or replaces selection).
    pub fn type_text(&self, text: &str) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.replace_selection_or_insert(text);
        }
    }

    pub fn backspace(&self) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.backspace();
        }
    }

    pub fn delete(&self) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.delete();
        }
    }

    pub fn enter(&self) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.enter();
        }
    }

    /// Copy selection into a String (application-layer returns the text; the interface
    /// layer owns the clipboard seam).
    pub fn copy_selection(&self) -> Option<String> {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let b = buf_arc.lock().unwrap();
            b.selection_text()
        } else {
            None
        }
    }

    /// Delete selection content (cut should call copy_selection first).
    pub fn delete_selection(&self) -> bool {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            b.delete_selection_and_return_cursor_at_start(true)
        } else {
            false
        }
    }

    /// Paste: read clipboard and paste into active buffer.
    pub fn paste_text(&self, text: &str) {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            // record insertion start
            let start_line = b.cursor_line;
            let start_col = b.cursor_col;
            b.replace_selection_or_insert(text);
            // record insertion end (cursor is placed at end of inserted text)
            let end_line = b.cursor_line;
            let end_col = b.cursor_col;
            b.selection = Some(Selection {
                anchor_line: start_line,
                anchor_col: start_col,
                active_line: end_line,
                active_col: end_col,
            });
        }
    }

    /// Undo last edit (returns true if an undo was performed).
    pub fn undo(&self) -> bool {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            let res = b.undo();
            // After undo, if buffer has saved_text, recompute dirty accordingly
            b.dirty = b
                .saved_text
                .as_ref()
                .map(|s| s != &b.to_text())
                .unwrap_or(true);
            res
        } else {
            false
        }
    }

    /// Redo previously undone edit (returns true if a redo was performed).
    pub fn redo(&self) -> bool {
        if let Some(buf_arc) = self.get_active_buffer_arc() {
            let mut b = buf_arc.lock().unwrap();
            let res = b.redo();
            b.dirty = b
                .saved_text
                .as_ref()
                .map(|s| s != &b.to_text())
                .unwrap_or(true);
            res
        } else {
            false
        }
    }

    // --------------------------
    // Close buffer behavior
    // --------------------------

    /// Close the buffer that matches the provided path.
    /// Returns Closed on success, BlockedByDirty if buffer has unsaved changes,
    /// or BufferNotFound if no opened buffer matches the path.
    pub fn close_buffer(&self, path: &Path) -> CloseResult {
        let mut st = self.inner.lock().unwrap();
        // find index
        let idx = st.paths.iter().position(|p| match p {
            Some(pp) => pp == path,
            None => false,
        });
        let idx = match idx {
            Some(i) => i,
            None => return CloseResult::BufferNotFound,
        };

        // check dirty
        if st.buffers[idx].lock().unwrap().dirty {
            return CloseResult::BlockedByDirty;
        }

        // perform removal
        st.buffers.remove(idx);
        st.paths.remove(idx);

        // determine new active index
        if st.buffers.is_empty() {
            st.active = None;
        } else if let Some(active_i) = st.active {
            if active_i == idx {
                // closed the active buffer: prefer previous neighbor, else next (which became idx)
                if idx > 0 {
                    st.active = Some(idx - 1);
                } else {
                    st.active = Some(0);
                }
            } else if active_i > idx {
                // shift due to removal
                st.active = Some(active_i - 1);
            } else {
                // active stays the same
                st.active = Some(active_i);
            }
        } else {
            // no active before; pick first
            st.active = Some(0);
        }

        CloseResult::Closed
    }

    /// Close the currently-active buffer (if any) with the same semantics as close_buffer.
    pub fn close_active(&self) -> CloseResult {
        let active_path_opt = {
            let st = self.inner.lock().unwrap();
            st.active.and_then(|i| st.paths[i].clone())
        };
        match active_path_opt {
            Some(p) => self.close_buffer(&p),
            None => {
                // If active buffer is unnamed, close by index
                let mut st = self.inner.lock().unwrap();
                match st.active {
                    Some(idx) => {
                        if st.buffers[idx].lock().unwrap().dirty {
                            CloseResult::BlockedByDirty
                        } else {
                            st.buffers.remove(idx);
                            st.paths.remove(idx);
                            if st.buffers.is_empty() {
                                st.active = None;
                            } else if idx > 0 {
                                st.active = Some(idx - 1);
                            } else {
                                st.active = Some(0);
                            }
                            CloseResult::Closed
                        }
                    }
                    None => CloseResult::BufferNotFound,
                }
            }
        }
    }

    /// Attempt to resolve a previously-blocked dirty-close by saving the buffer
    /// to its associated path and then closing it.
    ///
    /// - If the buffer is not found -> BufferNotFound
    /// - If the buffer is not dirty -> NotDirty
    /// - On successful save+close -> ClosedAfterSave
    /// - On save failure -> SaveFailed(io::Error) and the buffer remains open
    pub fn resolve_dirty_close_save(&self, path: &Path) -> ResolveDirtyCloseResult {
        // locate index under lock
        let idx_opt = {
            let st = self.inner.lock().unwrap();
            st.paths.iter().position(|p| match p {
                Some(pp) => pp == path,
                None => false,
            })
        };

        let idx = match idx_opt {
            Some(i) => i,
            None => return ResolveDirtyCloseResult::BufferNotFound,
        };

        // get buffer arc and check dirty under its lock
        let buf_arc = {
            let st = self.inner.lock().unwrap();
            st.buffers[idx].clone()
        };

        {
            let b = buf_arc.lock().unwrap();
            if !b.dirty {
                return ResolveDirtyCloseResult::NotDirty;
            }
        }

        // Acquire text under buffer lock, write to disk, then update saved state.
        let text = {
            let b = buf_arc.lock().unwrap();
            b.to_text()
        };

        if let Err(e) = std::fs::write(path, text.as_bytes()) {
            return ResolveDirtyCloseResult::SaveFailed(e);
        }

        // Mark buffer as saved (clear dirty, update saved_text)
        {
            let mut b = buf_arc.lock().unwrap();
            b.set_saved_state();
        }

        // Now remove buffer from workspace (re-find its index in case of concurrent changes).
        let mut st = self.inner.lock().unwrap();
        let idx = match st.paths.iter().position(|p| match p {
            Some(pp) => pp == path,
            None => false,
        }) {
            Some(i) => i,
            None => return ResolveDirtyCloseResult::BufferNotFound,
        };

        st.buffers.remove(idx);
        st.paths.remove(idx);

        // determine new active index (same semantics as close_buffer)
        if st.buffers.is_empty() {
            st.active = None;
        } else if let Some(active_i) = st.active {
            if active_i == idx {
                // closed the active buffer: prefer previous neighbor, else next (which became idx)
                if idx > 0 {
                    st.active = Some(idx - 1);
                } else {
                    st.active = Some(0);
                }
            } else if active_i > idx {
                // shift due to removal
                st.active = Some(active_i - 1);
            } else {
                // active stays the same
                st.active = Some(active_i);
            }
        } else {
            // no active before; pick first
            st.active = Some(0);
        }

        ResolveDirtyCloseResult::ClosedAfterSave
    }

    /// Attempt to resolve a previously-blocked dirty-close by discarding in-memory edits
    /// and then closing the buffer.
    ///
    /// - If the buffer is not found -> BufferNotFound
    /// - If the buffer is not dirty -> NotDirty
    /// - If the buffer is file-backed: on success the on-disk content replaces in-memory edits.
    /// - If the buffer is unnamed: discard simply removes the buffer.
    /// - On success -> ClosedAfterDiscard
    /// - On IO/read failure when trying to restore on-disk content -> IoError(io::Error)
    pub fn resolve_dirty_close_discard(&self, path: &Path) -> ResolveDirtyCloseResult {
        // find index
        let idx_opt = {
            let st = self.inner.lock().unwrap();
            st.paths.iter().position(|p| match p {
                Some(pp) => pp == path,
                None => false,
            })
        };

        let idx = match idx_opt {
            Some(i) => i,
            None => return ResolveDirtyCloseResult::BufferNotFound,
        };

        // get buffer arc and check dirty
        let buf_arc = {
            let st = self.inner.lock().unwrap();
            st.buffers[idx].clone()
        };

        {
            let b = buf_arc.lock().unwrap();
            if !b.dirty {
                return ResolveDirtyCloseResult::NotDirty;
            }
        }

        // If file-backed, reload from disk to abandon in-memory edits.
        let file_path_opt = {
            let st = self.inner.lock().unwrap();
            st.paths[idx].clone()
        };

        if let Some(fp) = file_path_opt {
            // read disk content
            let content = match std::fs::read_to_string(&fp) {
                Ok(c) => c,
                Err(e) => return ResolveDirtyCloseResult::IoError(e),
            };
            // replace buffer content and reset history/state
            {
                let mut b = buf_arc.lock().unwrap();
                b.load_from_text(&content);
            }
        } else {
            // unnamed buffer: discarding means just closing without writing.
            // nothing to restore in-memory; we can drop it.
        }

        // Now remove buffer from workspace (re-find index)
        let mut st = self.inner.lock().unwrap();
        let idx = match st.paths.iter().position(|p| match p {
            Some(pp) => pp == path,
            None => false,
        }) {
            Some(i) => i,
            None => return ResolveDirtyCloseResult::BufferNotFound,
        };

        st.buffers.remove(idx);
        st.paths.remove(idx);

        // determine new active index (same semantics as close_buffer)
        if st.buffers.is_empty() {
            st.active = None;
        } else if let Some(active_i) = st.active {
            if active_i == idx {
                // closed the active buffer: prefer previous neighbor, else next (which became idx)
                if idx > 0 {
                    st.active = Some(idx - 1);
                } else {
                    st.active = Some(0);
                }
            } else if active_i > idx {
                // shift due to removal
                st.active = Some(active_i - 1);
            } else {
                // active stays the same
                st.active = Some(active_i);
            }
        } else {
            // no active before; pick first
            st.active = Some(0);
        }

        ResolveDirtyCloseResult::ClosedAfterDiscard
    }
}
