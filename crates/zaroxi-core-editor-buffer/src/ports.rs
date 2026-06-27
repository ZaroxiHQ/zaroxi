// Core editor buffer port: BufferStore and simple BufferId DTO.
//
// The core provides the trait; application uses it. Implementations can be in core or infra.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

/// Boxed future alias for this skeleton (import from kernel in future).
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Simple BufferId DTO (opaque newtype).
///
/// This type is the canonical owner of lightweight Buffer identity semantics
/// for the slice. Keep the helpers small and explicit:
/// - serde Serialize/Deserialize so it can cross app boundaries where needed
/// - parse() for fallible parsing from string
/// - as_str() to access the inner representation
/// - from_path() helper to derive storage-style ids used in the slice
/// - path() helper to reverse `from_path()` when the representation encodes a path
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BufferId(pub String);

impl BufferId {
    /// Fallible parse from a string. Minimal validation: non-empty.
    /// Representation rules (prefixes like `buf:`) are considered an infra/core
    /// concern; the parser only rejects empty inputs.
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.trim().is_empty() {
            Err("buffer id must not be empty".to_string())
        } else {
            Ok(BufferId(s.to_string()))
        }
    }

    /// Borrow the inner string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct a BufferId from a filesystem path using the `buf:<path>` convention
    /// used in this slice. This helper keeps the representation in core.
    pub fn from_path(path: &std::path::Path) -> Self {
        BufferId(format!("buf:{}", path.to_string_lossy()))
    }

    /// If this BufferId follows the `buf:<path>` convention return the PathBuf part.
    /// Otherwise returns None. This helper centralizes the representation knowledge.
    pub fn path(&self) -> Option<std::path::PathBuf> {
        if self.0.starts_with("buf:") && self.0.len() > 4 {
            Some(std::path::PathBuf::from(&self.0[4..]))
        } else {
            None
        }
    }
}

impl From<&str> for BufferId {
    fn from(s: &str) -> Self {
        BufferId(s.to_string())
    }
}

impl From<std::path::PathBuf> for BufferId {
    fn from(p: PathBuf) -> Self {
        BufferId::from_path(&p)
    }
}

impl fmt::Display for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for BufferId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
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

/// Simple typed transaction/edit model expressed in character indices (not bytes).
/// The core layer owns the semantics of applying these edits to the underlying
/// buffer string/rope. Character indices are used for safety with UTF-8 content.
#[derive(Clone, Debug)]
pub enum TextEdit {
    /// Insert `text` at character index `index`.
    Insert { index: usize, text: String },

    /// Delete the inclusive..exclusive character range [start, end).
    Delete { start: usize, end: usize },

    /// Replace the inclusive..exclusive character range [start, end) with `text`.
    Replace { start: usize, end: usize, text: String },
}

/// Port trait: BufferStore
pub trait BufferStore: Send + Sync {
    /// Open a buffer backed by a filesystem path. Returns a BufferId.
    fn open_buffer(&self, path: PathBuf) -> BoxFuture<'static, Result<BufferId, BufferError>>;

    /// Get full text for a buffer if available (sync read path for the slice).
    fn get_text(&self, id: &BufferId) -> Option<String>;

    /// Set or replace the full text for a buffer.
    /// Returns Ok(()) on success or BufferError on failure (e.g. buffer not found).
    fn set_text(
        &self,
        id: &BufferId,
        content: String,
    ) -> BoxFuture<'static, Result<(), BufferError>>;

    /// Apply a typed text transaction/edit to the buffer content.
    /// The transaction uses character indices (0-based). Implementations must
    /// atomically apply the edit and persist the updated content.
    fn apply_transaction(
        &self,
        id: &BufferId,
        txn: TextEdit,
    ) -> BoxFuture<'static, Result<(), BufferError>>;

    /// Close and release a buffer from the store.
    /// Removes the buffer's content, freeing associated memory.
    /// Returns Ok(()) on success or BufferError if the buffer was not found.
    fn close_buffer(&self, _id: &BufferId) -> BoxFuture<'static, Result<(), BufferError>> {
        Box::pin(async move { Err(BufferError("close_buffer not implemented".to_string())) })
    }
}

pub type DynBufferStore = Arc<dyn BufferStore>;
