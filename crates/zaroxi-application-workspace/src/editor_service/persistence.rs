use super::*;
use std::fs;

impl EditorService {
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
        fs::write(path, text.as_bytes())?;
        // update buffer saved state
        let mut b = buf_arc.lock().unwrap();
        b.set_saved_state();
        Ok(())
    }

    /// Reload active buffer contents from disk: replace buffer text and reset history.
    pub fn reload(&self, path: &Path) -> io::Result<()> {
        let content = fs::read_to_string(path)?;
        let buf_arc = match self.get_active_buffer_arc() {
            Some(a) => a,
            None => return Err(io::Error::new(io::ErrorKind::NotFound, "no active buffer")),
        };
        let mut b = buf_arc.lock().unwrap();
        b.load_from_text(&content);
        Ok(())
    }
}
