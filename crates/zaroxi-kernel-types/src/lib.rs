#![deny(missing_docs)]
//! Kernel shared plain-value types.
//!
//! Minimal, stable value types permitted at the kernel layer. These are pure
//! data containers only (no IO, no runtime, no platform).

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt;

/// Strongly-typed identifier backed by a UUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(Uuid);

impl Id {
    /// Create a new random Id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Construct from an existing Uuid.
    pub fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }

    /// Return the inner Uuid.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Position in a text-like sequence (line/column).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// 0-based line index.
    pub line: u32,
    /// 0-based column index (in characters).
    pub column: u32,
}

impl Position {
    /// Create a zero position.
    pub fn zero() -> Self {
        Self { line: 0, column: 0 }
    }
}

/// Inclusive span between two positions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    /// Start position (inclusive).
    pub start: Position,
    /// End position (exclusive).
    pub end: Position,
}

impl Span {
    /// Create a span from start to end.
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Is the span empty?
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}
