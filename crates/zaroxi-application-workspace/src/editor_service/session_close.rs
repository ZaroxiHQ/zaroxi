use super::*;
use std::fs;

/// Session-level close helpers: attempt close and resolution paths.
impl EditorService {
    /// Attempt to close the entire workspace/session. If no buffers are dirty the
    /// session is closed (buffers removed) and AttemptCloseSessionResult::Closed is returned.
    /// If one or more buffers are dirty the call is non-destructive and returns
    /// AttemptCloseSessionResult::BlockedByDirty with a list of dirty buffers.
    pub fn attempt_close_session(&self) -> AttemptCloseSessionResult {
        // Snapshot buffer arcs and paths to check dirty flags without holding inner lock.
        let buffers: Vec<(usize, Option<PathBuf>, Arc<Mutex<Buffer>>)> = {
            let st = self.inner.lock().unwrap();
            st.buffers
                .iter()
                .enumerate()
                .map(|(i, a)| (i, st.paths[i].clone(), a.clone()))
                .collect()
        };

        let mut dirty: Vec<(usize, Option<PathBuf>)> = Vec::new();
        for (i, path_opt, arc) in buffers.iter() {
            let b = arc.lock().unwrap();
            if b.dirty {
                dirty.push((*i, path_opt.clone()));
            }
        }

        if dirty.is_empty() {
            // Safe to close: remove all buffers deterministically.
            let mut st = self.inner.lock().unwrap();
            st.buffers.clear();
            st.paths.clear();
            st.active = None;
            AttemptCloseSessionResult::Closed
        } else {
            AttemptCloseSessionResult::BlockedByDirty { dirty_buffers: dirty }
        }
    }

    /// Resolve a previously-blocked session close by attempting to save all dirty buffers.
    ///
    /// Behavior:
    /// - If there are no dirty buffers -> NothingToResolve.
    /// - Attempts to write all dirty, file-backed buffers to disk. Unnamed dirty buffers
    ///   are considered failures (no path to write to).
    /// - If any write fails the workspace is left unchanged and SaveAllFailed is returned
    ///   with the failing buffer list.
    /// - On success all buffers are marked saved and the workspace is closed (buffers removed).
    pub fn resolve_close_session_save_all(&self) -> ResolveCloseSessionResult {
        // Snapshot buffer arcs and paths to avoid holding the workspace lock during I/O.
        let buffers: Vec<(usize, Option<PathBuf>, Arc<Mutex<Buffer>>)> = {
            let st = self.inner.lock().unwrap();
            st.buffers
                .iter()
                .enumerate()
                .map(|(i, a)| (i, st.paths[i].clone(), a.clone()))
                .collect()
        };

        let mut dirty_list: Vec<(usize, Option<PathBuf>, Arc<Mutex<Buffer>>)> = Vec::new();
        for (i, p, arc) in buffers.into_iter() {
            let b = arc.lock().unwrap();
            if b.dirty {
                dirty_list.push((i, p, arc.clone()));
            }
        }

        if dirty_list.is_empty() {
            return ResolveCloseSessionResult::NothingToResolve;
        }

        // First pass: attempt to write all dirty buffers to disk without mutating workspace state.
        let mut failed: Vec<(usize, Option<PathBuf>)> = Vec::new();
        for (i, p_opt, arc) in dirty_list.iter() {
            match p_opt {
                Some(path) => {
                    let text = {
                        let b = arc.lock().unwrap();
                        b.to_text()
                    };
                    if let Err(_) = fs::write(path, text.as_bytes()) {
                        failed.push((*i, Some(path.clone())));
                    }
                }
                None => {
                    // Unnamed buffer: cannot save without "save as" flow.
                    failed.push((*i, None));
                }
            }
        }

        if !failed.is_empty() {
            return ResolveCloseSessionResult::SaveAllFailed { failed_buffers: failed };
        }

        // All writes succeeded: update saved state for each dirty buffer.
        for (_i, _p, arc) in dirty_list.iter() {
            let mut b = arc.lock().unwrap();
            b.set_saved_state();
        }

        // Finally, close the session deterministically by removing all buffers.
        let mut st = self.inner.lock().unwrap();
        st.buffers.clear();
        st.paths.clear();
        st.active = None;

        ResolveCloseSessionResult::ClosedAfterSaveAll
    }

    /// Resolve a previously-blocked session close by discarding all in-memory edits and closing.
    ///
    /// - If there are no dirty buffers -> NothingToResolve.
    /// - For file-backed buffers, reload on-disk content to replace in-memory edits.
    /// - For unnamed buffers, discarding simply drops them.
    /// - On any IO read error while reloading -> IoError and workspace remains unchanged.
    /// - On success all buffers are closed (removed) and ClosedAfterDiscardAll is returned.
    pub fn resolve_close_session_discard_all(&self) -> ResolveCloseSessionResult {
        // Snapshot current buffers
        let buffers: Vec<(usize, Option<PathBuf>, Arc<Mutex<Buffer>>)> = {
            let st = self.inner.lock().unwrap();
            st.buffers
                .iter()
                .enumerate()
                .map(|(i, a)| (i, st.paths[i].clone(), a.clone()))
                .collect()
        };

        let mut dirty_list: Vec<(usize, Option<PathBuf>, Arc<Mutex<Buffer>>)> = Vec::new();
        for (i, p, arc) in buffers.into_iter() {
            let b = arc.lock().unwrap();
            if b.dirty {
                dirty_list.push((i, p, arc.clone()));
            }
        }

        if dirty_list.is_empty() {
            return ResolveCloseSessionResult::NothingToResolve;
        }

        // Reload disk content for file-backed buffers to abandon in-memory edits.
        for (_i, p_opt, arc) in dirty_list.iter() {
            if let Some(path) = p_opt {
                match fs::read_to_string(path) {
                    Ok(content) => {
                        let mut b = arc.lock().unwrap();
                        b.load_from_text(&content);
                    }
                    Err(e) => {
                        // Do not mutate workspace state on failure.
                        return ResolveCloseSessionResult::IoError(e);
                    }
                }
            } else {
                // unnamed buffer: nothing to reload; discarding will drop it on close.
            }
        }

        // Close the session deterministically by removing all buffers.
        let mut st = self.inner.lock().unwrap();
        st.buffers.clear();
        st.paths.clear();
        st.active = None;

        ResolveCloseSessionResult::ClosedAfterDiscardAll
    }
}
