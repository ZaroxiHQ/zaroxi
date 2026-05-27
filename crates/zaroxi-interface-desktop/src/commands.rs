use zaroxi_kernel_types::Id;

/// Minimal set of editor core commands.
/// These are intentionally small; app-level commands will live in `zaroxi-app`.
#[derive(Debug, Clone)]
pub enum EditorCommand {
    /// Insert text at a character offset in the specified document.
    InsertText { doc_id: Id, offset: usize, text: String },
}

/// Small AI command surface for the desktop.
/// These are explicit user-invoked commands to request/apply/cancel AI edits for the active buffer.
#[derive(Debug, Clone)]
pub enum AiCommand {
    /// Request an AI edit for the current active buffer in the given session.
    RequestEditActive { session_id: zaroxi_application_workspace::ports::SessionId },

    /// Apply a previously proposed AI edit for the active buffer.
    ApplyEditActive { session_id: zaroxi_application_workspace::ports::SessionId },

    /// Cancel any pending AI edit proposal without mutating the buffer.
    CancelEditActive { session_id: zaroxi_application_workspace::ports::SessionId },
}
