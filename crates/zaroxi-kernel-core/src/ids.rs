use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Foundation identifier types used across Zaroxi crates.
///
/// This file provides a single authoritative definition for common ID types:
/// - DocumentId
/// - BufferId
/// - WorkspaceId
///
/// All types are simple newtypes around `Uuid` and derive serde for convenient
/// serialization across crates. Keep this file small and stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(Uuid);

impl DocumentId {
    /// Create a new random document id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// A nil (zero) id useful for defaults/tests.
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// Expose the underlying uuid.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for DocumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A strongly-typed buffer identifier (kept distinct from DocumentId to allow
/// evolution of id spaces in future).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BufferId(Uuid);

impl BufferId {
    /// Create a new unique buffer id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Expose underlying uuid.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for BufferId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A strongly-typed workspace identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(Uuid);

impl WorkspaceId {
    /// Create a new unique workspace id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Expose underlying uuid.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for WorkspaceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
