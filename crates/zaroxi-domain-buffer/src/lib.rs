#![allow(dead_code)]
#![allow(unused_imports)]

// Auto-generated domain crate: zaroxi-domain-buffer
// Responsibility: Provide lightweight domain models for document buffers.

pub mod rules;

use serde::{Deserialize, Serialize};
use zaroxi_kernel_types::Id;
use std::path::PathBuf;

/// Domain representation of a document buffer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier for the document (kernel Id).
    pub id: Id,
    /// Optional file path on disk for this buffer.
    pub path: Option<PathBuf>,
    /// Display name (file name or untitled).
    pub display_name: String,
    /// Text contents of the buffer.
    pub text: String,
    /// Dirty flag indicating unsaved changes.
    pub dirty: bool,
}

impl Document {
    /// Mark buffer dirty.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark buffer as saved (not dirty).
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

/// Marker to make the crate non-empty for packaging.
pub fn _crate_marker() {
    // intentionally empty
}
