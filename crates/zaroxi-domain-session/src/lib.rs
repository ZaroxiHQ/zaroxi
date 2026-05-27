// zaroxi-domain-session
// Auto-generated crate stub for the Zaroxi migration.
// Responsibility: Auto-generated crate

#![allow(dead_code)]
#![allow(unused_imports)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid;
use zaroxi_kernel_id::UuidId;

/// Small, semantic session identifier newtype backed by the canonical kernel id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub UuidId);

impl SessionId {
    /// Generate a new random session id (v4).
    pub fn new_v4() -> Self {
        SessionId(UuidId::new_v4())
    }

    /// Borrow inner UuidId.
    pub fn as_uuid(&self) -> &UuidId {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SessionId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SessionId(s.parse()?))
    }
}

impl From<UuidId> for SessionId {
    fn from(u: UuidId) -> Self {
        SessionId(u)
    }
}

impl From<SessionId> for UuidId {
    fn from(s: SessionId) -> UuidId {
        s.0
    }
}

pub fn _crate_marker() {
    // Marker function to make the crate non-empty for packaging.
}
