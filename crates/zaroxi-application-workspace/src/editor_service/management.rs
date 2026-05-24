use super::*;
use std::fs;

impl EditorService {
    /// Open a file into a new buffer and activate it.
    pub fn open_file(&self, path: &Path) -> io::Result<()> {
        let content = fs::read_to_string(path)?;
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
    pub(crate) fn get_active_buffer_arc(&self) -> Option<Arc<Mutex<Buffer>>> {
        let st = self.inner.lock().unwrap();
        match st.active {
            Some(i) => Some(st.buffers[i].clone()),
            None => None,
        }
    }
}
