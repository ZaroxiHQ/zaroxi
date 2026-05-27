#![allow(dead_code)]
// Auto-generated stub for `zaroxi-core-text-buffer`.

pub mod buffer;
pub mod ports;
pub use buffer::{Buffer, Selection};

pub const CRATE_NAME: &str = "zaroxi-core-text-buffer";

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use zaroxi_kernel_id::UuidId;

/// Buffer identity newtype backed by the canonical kernel id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BufferId(pub UuidId);

impl BufferId {
    /// Create a new random buffer id (v4).
    pub fn new_v4() -> Self {
        BufferId(UuidId::new_v4())
    }

    /// Borrow inner UuidId.
    pub fn as_uuid(&self) -> &UuidId {
        &self.0
    }
}

impl fmt::Display for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for BufferId {
    type Err = <UuidId as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let u: UuidId = s.parse()?;
        Ok(BufferId(u))
    }
}

impl From<UuidId> for BufferId {
    fn from(u: UuidId) -> Self {
        BufferId(u)
    }
}

impl From<BufferId> for UuidId {
    fn from(b: BufferId) -> UuidId {
        b.0
    }
}

pub fn info() -> &'static str {
    CRATE_NAME
}
