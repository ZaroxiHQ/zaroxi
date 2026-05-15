//! Command definitions for Zaroxi RPC.

use serde::{Deserialize, Serialize};

/// A command that can be sent to a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// Workspace-related commands.
    Workspace(WorkspaceCommand),
    /// Editor-related commands.
    Editor(EditorCommand),
    /// AI-related commands.
    Ai(AiCommand),
}

/// Workspace-specific commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkspaceCommand {
    /// Open a workspace at the given path.
    OpenWorkspace { path: String },
    /// List directory contents.
    ListDirectory { path: String },
    /// Open a file.
    OpenFile { path: String },
}

/// Editor-specific commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorCommand {
    /// Get document content.
    GetDocument { buffer_id: String },
    /// Apply an edit to a document.
    ApplyEdit { buffer_id: String, edit: String },
    /// Save a document.
    SaveDocument { buffer_id: String },
}

/// AI-specific commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiCommand {
    /// Start an AI task.
    StartTask { task_id: String, prompt: String },
    /// Cancel an AI task.
    CancelTask { task_id: String },
}
