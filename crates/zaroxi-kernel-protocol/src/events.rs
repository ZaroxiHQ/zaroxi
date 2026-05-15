//! Event definitions for Zaroxi RPC.

use serde::{Deserialize, Serialize};

/// An event that can be emitted by a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Workspace-related events.
    Workspace(WorkspaceEvent),
    /// Editor-related events.
    Editor(EditorEvent),
    /// AI-related events.
    Ai(AiEvent),
}

/// Workspace-specific events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceEvent {
    /// Workspace opened.
    WorkspaceOpened { workspace_id: String, path: String },
    /// Directory listed.
    DirectoryListed { path: String, entries: Vec<String> },
    /// File opened.
    FileOpened { path: String, content: String },
}

/// Editor-specific events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorEvent {
    /// Document changed.
    DocumentChanged { buffer_id: String, version: u64 },
    /// Cursor moved.
    CursorMoved { buffer_id: String, line: u32, column: u32 },
}

/// AI-specific events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiEvent {
    /// AI task started.
    TaskStarted { task_id: String },
    /// AI task completed.
    TaskCompleted { task_id: String, result: String },
    /// AI task failed.
    TaskFailed { task_id: String, error: String },
}
