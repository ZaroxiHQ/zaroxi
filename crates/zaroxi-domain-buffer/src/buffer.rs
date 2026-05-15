use serde::{Deserialize, Serialize};
use zaroxi_kernel_types::Id;

/// Document model used by the editor buffer crate.
///
/// For v1 the text is stored as a plain String. This intentionally avoids
/// premature optimization; a rope can be introduced later behind this API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier.
    pub id: Id,
    /// Display name (file name or untitled).
    pub display_name: String,
    /// Text contents of the buffer.
    pub text: String,
    /// Dirty flag indicating unsaved changes.
    pub dirty: bool,
}

impl Document {
    /// Create a new in-memory document with provided contents.
    pub fn new(display_name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            display_name: display_name.into(),
            text: text.into(),
            dirty: false,
        }
    }

    /// Convenience: create a welcome document used by the initial app state.
    pub fn welcome() -> Self {
        let text = r#"Welcome to Zaroxi Studio!

This is a minimal editor shell (v1). The UI is a placeholder backed by
a small, well-structured application model. Explore the workspace on the
left, and the editor surface in the center.

More features will be added iteratively with a focus on clean architecture."#;
        Self::new("Welcome", text)
    }
}
