#![doc = "Kernel-level typed Id primitives for Zaroxi. Minimal, std-only implementation."]
#![allow(dead_code)]

use core::cmp::{Eq, PartialEq};
use core::fmt;
use core::fmt::Debug;
use core::hash::Hash;
use core::marker::PhantomData;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

/// Compatibility UUID-backed identifier used across older crates.
///
/// Some existing crates import `zaroxi_kernel_id::UuidId`. To preserve
/// compatibility we provide a small, ergonomic UuidId wrapper here. The
/// representation uses the `uuid` crate for stability.
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

/// Strongly-typed identifier newtype using a phantom marker.
///
/// The underlying value is a u64 chosen for simplicity and cheap copying in
/// kernel-layer contexts. The typed wrapper prevents accidental mixing of
/// different id kinds (BufferId vs TabId, etc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id<T> {
    value: u64,
    marker: PhantomData<T>,
}

impl<T> Id<T> {
    /// Create a new Id with the provided value.
    pub const fn new(value: u64) -> Self {
        Self { value, marker: PhantomData }
    }

    /// Consume and return the underlying numeric value.
    pub const fn value(self) -> u64 {
        self.value
    }
}

/// Generator for typed Id<T>.
pub struct IdGen<T> {
    next: AtomicU64,
    marker: PhantomData<T>,
}

impl<T> IdGen<T> {
    /// Create a new generator starting at `start`.
    pub const fn new(start: u64) -> Self {
        Self { next: AtomicU64::new(start), marker: PhantomData }
    }

    /// Allocate the next id. Uses relaxed ordering for performance; this is
    /// sufficient for unique id generation without stronger synchronization.
    pub fn next(&self) -> Id<T> {
        let v = self.next.fetch_add(1, Ordering::Relaxed);
        Id::new(v)
    }
}

// Marker types for common kernel ids.
pub struct BufferMarker;
pub struct TabMarker;
pub struct WidgetMarker;
pub struct PanelMarker;

// Type aliases for convenience.
pub type BufferId = Id<BufferMarker>;
pub type TabId = Id<TabMarker>;
pub type WidgetId = Id<WidgetMarker>;
pub type PanelId = Id<PanelMarker>;

// Implement Display for Id<T> for debug-friendly output.
impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
