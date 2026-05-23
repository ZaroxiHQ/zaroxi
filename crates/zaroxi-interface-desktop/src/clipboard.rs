use std::sync::{Arc, Mutex};

/// Simple explicit clipboard seam for interface-desktop.
/// This is an in-memory clipboard implementation suitable for testing and
/// for later replacement with a platform-specific adapter.
#[derive(Clone)]
pub struct InMemoryClipboard {
    inner: Arc<Mutex<Option<String>>>,
}

impl InMemoryClipboard {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(None)) }
    }

    pub fn set(&self, text: impl Into<String>) {
        let mut guard = self.inner.lock().unwrap();
        *guard = Some(text.into());
    }

    pub fn get(&self) -> Option<String> {
        let guard = self.inner.lock().unwrap();
        guard.clone()
    }

    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap();
        *guard = None;
    }
}
