use crate::ports::{WorkspaceEvent, WorkspaceEventKind, BufferId};
use std::path::PathBuf;

/// Tiny shell-facing one-line projection that answers "what was the last event?"
/// The projection is intentionally minimal and adapter-local; it only exposes a
/// single `text` string suitable for a status line or tiny shell summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastEventLine {
    /// One-line human readable description of the most recent event.
    pub text: String,
}

impl LastEventLine {
    pub fn new<T: Into<String>>(s: T) -> Self {
        Self { text: s.into() }
    }
}

/// Summarize a WorkspaceEventKind into a short, readable one-line string.
///
/// This helper is public so tests can exercise the core mapping without needing
/// to construct a full workspace event (which may require kernel types).
pub fn summarize_event_kind(kind: &WorkspaceEventKind) -> String {
    // Helper to format optional BufferId values.
    fn fmt_opt_bufid(b: &Option<BufferId>) -> String {
        match b {
            Some(id) => format!("{:?}", id),
            None => "<none>".to_string(),
        }
    }

    match kind {
        WorkspaceEventKind::SessionOpened { .. } => "SessionOpened".to_string(),
        WorkspaceEventKind::BufferOpened { path, .. } => {
            // Prefer showing a friendly path when available.
            let disp = if path.as_os_str().is_empty() {
                "<unnamed>".to_string()
            } else {
                path.to_string_lossy().to_string()
            };
            format!("BufferOpened: {}", disp)
        }
        WorkspaceEventKind::BufferUpdated { buffer_id } => {
            format!("BufferUpdated: {:?}", buffer_id)
        }
        WorkspaceEventKind::ActiveBufferChanged { old, new } => {
            format!("ActiveBufferChanged: {} -> {}", fmt_opt_bufid(old), fmt_opt_bufid(new))
        }
        WorkspaceEventKind::ExplainExecuted { buffer_id, result } => {
            // Keep result short if it's multi-line or long; show a small suffix.
            let short = if result.len() > 80 {
                format!("{}...", &result[..80])
            } else {
                result.clone()
            };
            format!("ExplainExecuted: {:?}: {}", buffer_id, short)
        }
    }
}

/// Summarize the last event (optionally) into a LastEventLine.
/// If `last` is None, returns "No events".
pub fn summarize_last_event(last: Option<&WorkspaceEvent>) -> LastEventLine {
    match last {
        Some(ev) => LastEventLine::new(summarize_event_kind(&ev.kind)),
        None => LastEventLine::new("No events"),
    }
}
