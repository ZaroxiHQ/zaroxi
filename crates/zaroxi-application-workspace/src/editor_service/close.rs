use super::*;

impl EditorService {
    /// Close the buffer that matches the provided path.
    /// Returns Closed on success, BlockedByDirty if buffer has unsaved changes,
    /// or BufferNotFound if no opened buffer matches the path.
    pub fn close_buffer(&self, path: &Path) -> CloseResult {
        let mut st = self.inner.lock().unwrap();
        // find index (prefer the most-recently-opened matching buffer)
        let idx = st.paths.iter().rposition(|p| match p {
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
}
