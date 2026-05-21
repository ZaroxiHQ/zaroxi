#![doc = include_str!("../README.md")]
//! zaroxi-kernel-id
//!
//! Small, stable kernel-layer identity primitives for the workspace.
//!
//! This crate provides a minimal, durable first API for identity values that
//! must remain stable and low-level. The initial type is a UUID-backed newtype
//! intended for use in kernel/core layers where strong typing and stable
//! representations matter.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Opaque UUID-backed identifier type.
///
/// This newtype intentionally exposes a small, stable surface: creation,
/// formatting/parsing, and conversions to/from the inner `Uuid`. It derives
/// `Serialize`/`Deserialize` so kernel-level crates can persist and transfer
/// IDs with the workspace-wide serde settings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UuidId(pub Uuid);

impl UuidId {
    /// Generate a new random (v4) identifier.
    pub fn new_v4() -> Self {
        UuidId(Uuid::new_v4())
    }

    /// Construct from an existing Uuid.
    pub fn from_uuid(u: Uuid) -> Self {
        UuidId(u)
    }

    /// Borrow the inner Uuid.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Consume the wrapper and return the inner Uuid.
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Display for UuidId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for UuidId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UuidId(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for UuidId {
    fn from(u: Uuid) -> Self {
        UuidId(u)
    }
}

impl From<UuidId> for Uuid {
    fn from(id: UuidId) -> Uuid {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn ser_de_roundtrip() {
        let id = UuidId::new_v4();
        let s = serde_json::to_string(&id).unwrap();
        let id2: UuidId = serde_json::from_str(&s).unwrap();
        assert_eq!(id, id2);
    }

    #[test]
    fn fmt_parse() {
        let id = UuidId::new_v4();
        let s = id.to_string();
        let parsed: UuidId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }
}
