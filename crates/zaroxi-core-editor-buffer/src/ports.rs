// Core editor buffer port: BufferStore and simple BufferId DTO.
//
// The core provides the trait; application uses it. Implementations can be in core or infra.

use std::path::PathBuf;
use std::sync::Arc;
use std::fmt;

/// Boxed future alias for this skeleton (import from kernel in future).
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Simple BufferId DTO (opaque newtype).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferId(pub String);

impl From<&str> for BufferId {
    fn from(s: &str) -> Self {
        BufferId(s.to_string())
    }
}

impl fmt::Display for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Simple error for buffer operations.
#[derive(Debug, Clone)]
pub struct BufferError(pub String);

impl From<&str> for BufferError {
    fn from(s: &str) -> Self {
        BufferError(s.to_string())
    }
}

/// Port trait: BufferStore
pub trait BufferStore: Send + Sync {
    /// Open a buffer backed by a filesystem path. Returns a BufferId.
    fn open_buffer(&self, path: PathBuf) -> BoxFuture<'static, Result<BufferId, BufferError>>;

    /// Get full text for a buffer if available (sync read path for the slice).
    fn get_text(&self, id: &BufferId) -> Option<String>;

    /// Set or replace the full text for a buffer.
    /// Returns Ok(()) on success or BufferError on failure (e.g. buffer not found).
    fn set_text(&self, id: &BufferId, content: String) -> BoxFuture<'static, Result<(), BufferError>>;
}

pub type DynBufferStore = Arc<dyn BufferStore>;
