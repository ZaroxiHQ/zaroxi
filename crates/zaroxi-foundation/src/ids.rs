use uuid::Uuid;
use std::fmt;

/// Strongly-typed document identifier.
///
/// Wrapping Uuid keeps cross-crate APIs explicit and prevents accidental mixups
/// between different id types.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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

/// Buffer id alias for clarity. Currently identical to DocumentId but kept
/// separate in case we evolve different id spaces later.
pub type BufferId = DocumentId;
