use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_kernel_types::Id;

/// Application command surface (current slice).
///
/// This crate is intentionally narrow: it owns only command-related DTOs and
/// small helpers clearly belonging to the "command" concept. It does NOT own
/// workspace/session lifecycle, events, or history storage policy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AppCommand {
    AiExplain { buffer_id: BufferId },
    InsertText { buffer_id: BufferId, offset: usize, text: String },
}

/// Command kind for history records (typed minimal).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CommandKind {
    BootWorkspace { path: PathBuf },
    OpenBuffer { path: PathBuf },
    UpdateBuffer { buffer_id: BufferId },
    SetActiveBuffer { buffer_id: BufferId },
    ExplainActiveBuffer,
    DispatchAppCommand { command: AppCommand },
}

/// Command execution record stored in history.
///
/// Note: we intentionally use kernel-level Id (zaroxi_kernel_types::Id)
/// for session_id/workspace_id to avoid upward dependency cycles. The
/// application-workspace layer will map its SessionId wrapper to the raw Id
/// when producing these records.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandRecord {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub kind: CommandKind,
    /// Kernel-level id for session (None for system-level or anonymous records).
    pub session_id: Option<Id>,
    pub workspace_id: Option<Id>,
    pub buffer_id: Option<BufferId>,
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Small result for dispatched commands (shorthand).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub message: String,
}

/// Response wrapper returned by dispatch entry points.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DispatchCommandResponse {
    pub result: CommandResult,
}
