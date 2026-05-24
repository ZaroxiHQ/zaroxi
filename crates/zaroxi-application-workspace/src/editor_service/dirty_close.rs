use super::*;

impl EditorService {
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
