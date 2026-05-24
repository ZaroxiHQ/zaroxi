
/// Small, single-source-of-truth model describing an in-progress close resolution
/// flow that the desktop UI will present to the user.
///
/// This model is intentionally tiny and serializable (Clone/Debug) so it can be
/// stored directly on DesktopComposition and rendered by presenters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingClose {
    /// A request to close a specific buffer (tab). UI should present Save / Discard / Cancel.
    BufferClose {
        buffer_id: crate::ports::BufferId,
        /// Optional human-friendly display for the buffer (path or label).
        display: Option<String>,
        /// Whether the buffer is known to be dirty. When unknown, treat as dirty and show UI.
        dirty: bool,
    },
    /// A request to close the entire session (window). UI should present Save All / Discard All / Cancel.
    SessionClose {
        /// List of buffers that are dirty (or considered dirty) and require resolution.
        dirty_buffers: Vec<crate::ports::BufferId>,
        /// Small human-friendly summary (e.g. "3 dirty buffers")
        summary: String,
    },
    /// A resolution failure: present message to the user and keep pending state until action.
    ResolutionFailure {
        message: String,
    },
}

impl PendingClose {
    /// Render a compact single-line summary suitable for tiny shell banners/tests.
    pub fn render_summary(&self) -> String {
        match self {
            PendingClose::BufferClose { display, dirty, .. } => {
                if *dirty {
                    format!("Close buffer '{}' (unsaved changes)", display.clone().unwrap_or_else(|| "<unnamed>".to_string()))
                } else {
                    format!("Close buffer '{}'", display.clone().unwrap_or_else(|| "<unnamed>".to_string()))
                }
            }
            PendingClose::SessionClose { summary, .. } => format!("Close session: {}", summary),
            PendingClose::ResolutionFailure { message } => format!("Close failed: {}", message),
        }
    }
}
