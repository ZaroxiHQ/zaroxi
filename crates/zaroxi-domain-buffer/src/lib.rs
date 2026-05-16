#![allow(dead_code)]
#![allow(unused_imports)]

// Auto-generated domain crate: zaroxi-domain-buffer
// Responsibility: Provide lightweight domain models for document buffers.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Domain representation of a document buffer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier for the document.
    pub id: Uuid,
    /// Display name (file name or untitled).
    pub display_name: String,
    /// Text contents of the buffer.
    pub text: String,
    /// Dirty flag indicating unsaved changes.
    pub dirty: bool,
}

/// Marker to make the crate non-empty for packaging.
pub fn _crate_marker() {
    // intentionally empty
}
