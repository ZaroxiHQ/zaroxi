// Application workspace orchestrator trait skeleton - Phase 5 enhancements.
//
// This trait composes domain and core ports to implement use cases like "open workspace",
// multi-buffer management (list/set/get active buffer) and explain-active-buffer.
// Keep surface minimal and typed for Phase 5.

use crate as _; // placeholder for crate root
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
pub use zaroxi_core_editor_buffer::ports::{BufferId, TextEdit};
use zaroxi_kernel_types::Id;

use chrono::{DateTime, Utc};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

/// Boxed future alias (replace with kernel::BoxFuture in real code)
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Kernel-backed workspace identifier.
pub type WorkspaceId = Id;

/// Kernel-backed session identifier (application-scoped wrapper).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionId(pub Id);

impl From<Id> for SessionId {
    fn from(id: Id) -> Self {
        SessionId(id)
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// DTO: workspace session created by the application
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceSessionDTO {
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
}

/// Request to boot/open a workspace and create a session.
#[derive(Clone, Debug)]
pub struct WorkspaceBootRequest {
    pub path: PathBuf,
}

/// Response from workspace boot.
#[derive(Clone, Debug)]
pub struct WorkspaceBootResponse {
    pub session: WorkspaceSessionDTO,
}

/// Request to open a buffer within a session.
#[derive(Clone, Debug)]
pub struct OpenBufferRequest {
    pub session_id: SessionId,
    pub path: PathBuf,
}

/// Response from opening a buffer.
#[derive(Clone, Debug)]
pub struct OpenBufferResponse {
    pub buffer_id: BufferId,
}

/// Request to list opened buffers for a session.
#[derive(Clone, Debug)]
pub struct ListBuffersRequest {
    pub session_id: SessionId,
}

/// Response listing opened buffers and the active buffer (if any).
#[derive(Clone, Debug)]
pub struct ListBuffersResponse {
    pub buffer_ids: Vec<BufferId>,
    pub active_buffer: Option<BufferId>,
}

/// Request to set the active buffer for a session.
#[derive(Clone, Debug)]
pub struct SetActiveBufferRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
}

/// Response for set active buffer.
#[derive(Clone, Debug)]
pub struct SetActiveBufferResponse {
    pub ok: bool,
}

/// Request to get currently active buffer for a session.
#[derive(Clone, Debug)]
pub struct GetActiveBufferRequest {
    pub session_id: SessionId,
}

/// Response returning the active buffer id.
/// If there is no active buffer for the session the use-case returns an explicit error instead.
#[derive(Clone, Debug)]
pub struct GetActiveBufferResponse {
    pub buffer_id: BufferId,
}

/// Request to update buffer content.
#[derive(Clone, Debug)]
pub struct UpdateBufferRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
    pub new_content: String,
}

/// Response from updating buffer content.
#[derive(Clone, Debug)]
pub struct UpdateBufferResponse {
    pub ok: bool,
}

/// Request to apply a typed text transaction to a buffer within a session.
/// The `transaction` uses the core `TextEdit` type (character-indexed).
#[derive(Clone, Debug)]
pub struct ApplyTextTransactionRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
    pub transaction: TextEdit,
}

/// Response after applying a transaction. Returns the updated transient editor
/// state for the buffer and the new buffer content (if present).
#[derive(Clone, Debug)]
pub struct ApplyTextTransactionResponse {
    pub ok: bool,
    pub state: EditorState,
    pub content: Option<String>,
}

/// Typed errors for application use-cases (Phase 5).
#[derive(Clone, Debug)]
pub enum UseCaseError {
    UnknownSession,
    UnknownWorkspace,
    UnknownBuffer,
    NoActiveBuffer,
    InvalidActiveBuffer(String),
    InvalidMutation(String),
    AiFailure(String),
    /// Checkpoint was malformed or could not be applied.
    InvalidCheckpoint(String),
    /// Durability / storage related failure.
    DurabilityFailure(String),
    /// Attempt to restore a checkpoint that would re-use an existing session id.
    SessionAlreadyExists(SessionId),
}

impl std::fmt::Display for UseCaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UseCaseError::UnknownSession => write!(f, "unknown session"),
            UseCaseError::UnknownWorkspace => write!(f, "unknown workspace"),
            UseCaseError::UnknownBuffer => write!(f, "unknown buffer"),
            UseCaseError::NoActiveBuffer => write!(f, "no active buffer for session"),
            UseCaseError::InvalidActiveBuffer(s) => write!(f, "invalid active buffer: {}", s),
            UseCaseError::InvalidMutation(s) => write!(f, "invalid mutation: {}", s),
            UseCaseError::AiFailure(s) => write!(f, "ai failure: {}", s),
            UseCaseError::InvalidCheckpoint(s) => write!(f, "invalid checkpoint: {}", s),
            UseCaseError::DurabilityFailure(s) => write!(f, "durability failure: {}", s),
            UseCaseError::SessionAlreadyExists(sid) => write!(f, "session already exists: {}", sid),
        }
    }
}

impl std::error::Error for UseCaseError {}

// Re-export narrow command concepts owned by the new application-command crate.
pub use zaroxi_application_command::ports::{
    AppCommand, CommandKind, CommandRecord, CommandResult, DispatchCommandResponse,
};

/// Small workspace event model for important transitions (internal records).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WorkspaceEventKind {
    SessionOpened { session_id: SessionId, workspace_id: WorkspaceId },
    BufferOpened { buffer_id: BufferId, path: PathBuf },
    BufferUpdated { buffer_id: BufferId },
    ActiveBufferChanged { old: Option<BufferId>, new: Option<BufferId> },
    ExplainExecuted { buffer_id: BufferId, result: String },
}

/// Workspace event record with metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub kind: WorkspaceEventKind,
}

/// History repository port: infra may implement to persist/serve history and events.
/// Note: command records are owned by application-command but the history trait remains
/// here so history queries and session-typed APIs stay in application-workspace.
pub trait HistoryRepository: Send + Sync {
    fn record_command(&self, rec: CommandRecord) -> BoxFuture<'static, Result<(), String>>;
    fn record_event(&self, ev: WorkspaceEvent) -> BoxFuture<'static, Result<(), String>>;
    fn get_recent_commands(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> BoxFuture<'static, Result<Vec<CommandRecord>, String>>;
    fn get_recent_events(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> BoxFuture<'static, Result<Vec<WorkspaceEvent>, String>>;
}

pub type DynHistoryRepository = Arc<dyn HistoryRepository>;

/// Durability errors for checkpoint persistence.
#[derive(Clone, Debug)]
pub enum DurabilityError {
    Io(String),
    NotFound(String),
    Malformed(String),
    UnknownVersion(u32),
}

impl From<&str> for DurabilityError {
    fn from(s: &str) -> Self {
        DurabilityError::Io(s.to_string())
    }
}

impl std::fmt::Display for DurabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DurabilityError::Io(s) => write!(f, "io error: {}", s),
            DurabilityError::NotFound(s) => write!(f, "not found: {}", s),
            DurabilityError::Malformed(s) => write!(f, "malformed checkpoint: {}", s),
            DurabilityError::UnknownVersion(v) => write!(f, "unknown checkpoint version: {}", v),
        }
    }
}

/// Durability port: simple store for checkpoints.
pub trait DurabilityRepository: Send + Sync {
    /// Persist a checkpoint and return a location identifier (opaque string).
    fn save_checkpoint(
        &self,
        checkpoint: Checkpoint,
    ) -> BoxFuture<'static, Result<String, DurabilityError>>;

    /// Load a checkpoint by location id and return the deserialized checkpoint.
    fn load_checkpoint(
        &self,
        location: String,
    ) -> BoxFuture<'static, Result<Checkpoint, DurabilityError>>;
}

pub type DynDurabilityRepository = Arc<dyn DurabilityRepository>;

/// Request to query recent commands for a session.
#[derive(Clone, Debug)]
pub struct GetRecentCommandsRequest {
    pub session_id: SessionId,
    pub limit: usize,
}

/// Response for recent commands query.
#[derive(Clone, Debug)]
pub struct GetRecentCommandsResponse {
    pub commands: Vec<CommandRecord>,
}

/// Request to query recent workspace events for a session.
#[derive(Clone, Debug)]
pub struct GetRecentEventsRequest {
    pub session_id: SessionId,
    pub limit: usize,
}

/// Response for recent workspace events query.
#[derive(Clone, Debug)]
pub struct GetRecentEventsResponse {
    pub events: Vec<WorkspaceEvent>,
}

/// Request to dispatch an application command.
#[derive(Clone, Debug)]
pub struct DispatchCommandRequest {
    pub session_id: SessionId,
    pub command: AppCommand,
}

// CommandResult and DispatchCommandResponse are owned and defined by the
// `zaroxi-application-command` crate and are re-exported above via:
// `pub use zaroxi_application_command::ports::{..., CommandResult, DispatchCommandResponse};`

/// Snapshot of a single buffer (id + optional current content).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BufferSnapshot {
    pub buffer_id: BufferId,
    pub content: Option<String>,
}

/// Editor transient cursor position (line/column, 0-based).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorCursor {
    /// 0-based line index.
    pub line: u32,
    /// 0-based column index.
    pub column: u32,
}

impl EditorCursor {
    /// Create a zero cursor at start of document.
    pub fn zero() -> Self {
        Self { line: 0, column: 0 }
    }
}

/// Simple selection (anchor and active positions).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Selection {
    pub anchor: EditorCursor,
    pub active: EditorCursor,
}

/// Lightweight editor-state for an open buffer inside a session.
/// - cursor: current caret position
/// - selection: optional selection (anchor/active)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorState {
    pub cursor: EditorCursor,
    pub selection: Option<Selection>,
}

/// Request to set the editor cursor for a buffer in a session.
#[derive(Clone, Debug)]
pub struct SetEditorCursorRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
    pub cursor: EditorCursor,
}

/// Response when setting cursor.
#[derive(Clone, Debug)]
pub struct SetEditorCursorResponse {
    pub ok: bool,
}

/// Request to set a selection.
#[derive(Clone, Debug)]
pub struct SetSelectionRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
    pub selection: Selection,
}

/// Response when setting selection.
#[derive(Clone, Debug)]
pub struct SetSelectionResponse {
    pub ok: bool,
}

/// Request to clear selection for a buffer.
#[derive(Clone, Debug)]
pub struct ClearSelectionRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
}

/// Response when clearing selection.
#[derive(Clone, Debug)]
pub struct ClearSelectionResponse {
    pub ok: bool,
}

/// Request to get editor-state for a buffer.
#[derive(Clone, Debug)]
pub struct GetEditorStateRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
}

/// Response for editor-state query. If no transient state is present for the buffer,
/// `state` may be None.
#[derive(Clone, Debug)]
pub struct GetEditorStateResponse {
    pub state: Option<EditorState>,
}

/// Line-based viewport state for an open buffer.
/// - `top_line` is 1-based index of the first visible line in the viewport.
/// - `window_height` is the number of lines visible in the viewport.
/// - `center_cursor` when true indicates that projection should prefer centering
///   the cursor into the viewport when computing the visible window; otherwise
///   the explicit `top_line` is authoritative.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewportState {
    pub top_line: usize,
    pub window_height: usize,
    pub center_cursor: bool,
}

/// Request to set the viewport state for a buffer in a session.
#[derive(Clone, Debug)]
pub struct SetViewportRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
    pub viewport: ViewportState,
}

/// Response for set viewport request.
#[derive(Clone, Debug)]
pub struct SetViewportResponse {
    pub ok: bool,
}

/// Request to scroll the viewport by a signed line delta (positive => down).
#[derive(Clone, Debug)]
pub struct ScrollViewportRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
    /// Signed line delta: positive scrolls down, negative scrolls up.
    pub delta_lines: isize,
}

/// Response for scroll request (returns new viewport state).
#[derive(Clone, Debug)]
pub struct ScrollViewportResponse {
    pub ok: bool,
    pub new_viewport: ViewportState,
}

/// Request to obtain visible lines for a specific buffer/session using the stored viewport state.
#[derive(Clone, Debug)]
pub struct GetVisibleLinesRequest {
    pub session_id: SessionId,
    pub buffer_id: BufferId,
}

/// Response carrying a VisibleLinesWindow coming from the view seam.
#[derive(Clone, Debug)]
pub struct GetVisibleLinesResponse {
    pub window: crate::view::VisibleLinesWindow,
}

/// Editor document/view model returned by the view seam.
/// Combines current buffer content with editor transient state for presentation.
#[derive(Clone, Debug)]
pub struct EditorDocument {
    /// Buffer identifier (core canonical BufferId).
    pub buffer_id: BufferId,
    /// Current buffer content (if available).
    pub content: Option<String>,
    /// Current caret/cursor position.
    pub cursor: EditorCursor,
    /// Optional selection state.
    pub selection: Option<Selection>,
    /// Number of lines in the content snapshot.
    pub line_count: usize,
    /// The specific line string at `cursor.line` when available.
    pub current_line: Option<String>,
}

/// Request to obtain the editor document for the active buffer in a session.
#[derive(Clone, Debug)]
pub struct GetActiveEditorDocumentRequest {
    pub session_id: SessionId,
}

/// Response carrying the editor document.
#[derive(Clone, Debug)]
pub struct GetActiveEditorDocumentResponse {
    pub document: EditorDocument,
}

/// Read-model representing the current workspace session state.
#[derive(Clone, Debug)]
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub workspace_id: WorkspaceId,
    pub opened_buffers: Vec<BufferId>,
    pub active_buffer: Option<BufferId>,
    pub buffers: Vec<BufferSnapshot>,
    pub recent_commands: Vec<CommandRecord>,
    pub recent_events: Vec<WorkspaceEvent>,
}

/// Request for fetching a session snapshot (read-only).
#[derive(Clone, Debug)]
pub struct GetSessionSnapshotRequest {
    pub session_id: SessionId,
    /// How many recent commands/events to include in the snapshot.
    pub recent_limit: usize,
}

/// Response carrying the session snapshot.
#[derive(Clone, Debug)]
pub struct GetSessionSnapshotResponse {
    pub snapshot: SessionSnapshot,
}

/// Checkpoint DTO capturing a compact session state suitable for restore.
/// Includes an explicit `version` field to allow safe evolution of the format.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint serialization format version. Monotonic small integer.
    pub version: u32,
    /// Original session id carried by the checkpoint.
    pub session_id: SessionId,
    /// Workspace id referenced by the session.
    pub workspace_id: WorkspaceId,
    /// Opened buffer ids (in open order).
    pub opened_buffers: Vec<BufferId>,
    /// Active buffer id, if present.
    pub active_buffer: Option<BufferId>,
    /// Optional per-buffer snapshots (id + optional content).
    pub buffers: Vec<BufferSnapshot>,
    /// Recent commands/events included to preserve minimal history.
    pub recent_commands: Vec<CommandRecord>,
    pub recent_events: Vec<WorkspaceEvent>,
    /// Checkpoint creation time (informational).
    pub created_at: DateTime<Utc>,
}

/// Request to create a checkpoint for a session.
#[derive(Clone, Debug)]
pub struct CreateCheckpointRequest {
    pub session_id: SessionId,
}

/// Response carrying the created checkpoint.
#[derive(Clone, Debug)]
pub struct CreateCheckpointResponse {
    pub checkpoint: Checkpoint,
}

/// Request to restore a session from a checkpoint.
#[derive(Clone, Debug)]
pub struct RestoreCheckpointRequest {
    pub checkpoint: Checkpoint,
}

/// Response from restoring a checkpoint. Returns the restored session and optional replaced id.
#[derive(Clone, Debug)]
pub struct RestoreCheckpointResponse {
    /// The session that was restored (may equal the checkpoint.session_id).
    pub session: WorkspaceSessionDTO,
    /// Optional replacement id if a deterministic replacement policy was used.
    pub replaced_session_id: Option<SessionId>,
}

/// Request to persist a checkpoint using the configured durability adapter.
#[derive(Clone, Debug)]
pub struct SaveCheckpointRequest {
    pub session_id: SessionId,
}

/// Response returned when a checkpoint has been persisted.
#[derive(Clone, Debug)]
pub struct SaveCheckpointResponse {
    /// Opaque location identifier returned by the durability adapter (e.g. filename or id).
    pub location: String,
}

/// Request to load a checkpoint from the durability adapter and restore it.
#[derive(Clone, Debug)]
pub struct LoadCheckpointRequest {
    /// Opaque location identifier previously returned by save_checkpoint.
    pub location: String,
}

/// Response returns the restored session metadata (reuses existing restore response shape).
pub type LoadCheckpointResponse = RestoreCheckpointResponse;

/// Very small service trait. Implementations are in application layer.
/// Methods are explicit use-case entry points for Phase 5 multi-buffer behavior.
///
/// NOTE: we add a small, backwards-compatible session-close surface here so the
/// interface layer can coordinate visible session/window close flows without
/// introducing layer violations. Default implementations are conservative.
pub trait WorkspaceService: Send + Sync {
    /// Boot/open a workspace and create a UI session.
    fn boot_workspace(
        &self,
        req: WorkspaceBootRequest,
    ) -> BoxFuture<'static, Result<WorkspaceBootResponse, UseCaseError>>;

    /// Open a buffer inside an active session.
    fn open_buffer(
        &self,
        req: OpenBufferRequest,
    ) -> BoxFuture<'static, Result<OpenBufferResponse, UseCaseError>>;

    /// List opened buffers for a session and indicate the active buffer (if any).
    fn list_open_buffers(
        &self,
        req: ListBuffersRequest,
    ) -> BoxFuture<'static, Result<ListBuffersResponse, UseCaseError>>;

    /// Set the active buffer for a session.
    fn set_active_buffer(
        &self,
        req: SetActiveBufferRequest,
    ) -> BoxFuture<'static, Result<SetActiveBufferResponse, UseCaseError>>;

    /// Get the currently active buffer for a session.
    fn get_active_buffer(
        &self,
        req: GetActiveBufferRequest,
    ) -> BoxFuture<'static, Result<GetActiveBufferResponse, UseCaseError>>;

    /// Editor-state seam (Phase 3): typed cursor/selection APIs for open buffers.
    ///
    /// These methods only mutate or read editor transient state associated with an
    /// open buffer in a session. They do not mutate buffer text or affect rendering.
    fn set_editor_cursor(
        &self,
        req: SetEditorCursorRequest,
    ) -> BoxFuture<'static, Result<SetEditorCursorResponse, UseCaseError>>;

    fn set_editor_selection(
        &self,
        req: SetSelectionRequest,
    ) -> BoxFuture<'static, Result<SetSelectionResponse, UseCaseError>>;

    fn clear_editor_selection(
        &self,
        req: ClearSelectionRequest,
    ) -> BoxFuture<'static, Result<ClearSelectionResponse, UseCaseError>>;

    fn get_editor_state(
        &self,
        req: GetEditorStateRequest,
    ) -> BoxFuture<'static, Result<GetEditorStateResponse, UseCaseError>>;

    /// Viewport APIs: minimal typed viewport state seam for line-based scrolling and explicit control.
    /// - set_viewport_state: replace the stored viewport state for a buffer in a session.
    /// - scroll_viewport: adjust the stored top_line by a signed line delta (clamped to >= 1).
    fn set_viewport_state(
        &self,
        req: SetViewportRequest,
    ) -> BoxFuture<'static, Result<SetViewportResponse, UseCaseError>>;
    fn scroll_viewport(
        &self,
        req: ScrollViewportRequest,
    ) -> BoxFuture<'static, Result<ScrollViewportResponse, UseCaseError>>;

    /// Shorthand use-case: explain the currently active buffer (uses the AiClient).
    fn explain_active_buffer(
        &self,
        req: GetActiveBufferRequest,
    ) -> BoxFuture<'static, Result<DispatchCommandResponse, UseCaseError>>;

    /// Dispatch a high-level application command (AI requests, edits, etc).
    fn dispatch_command(
        &self,
        req: DispatchCommandRequest,
    ) -> BoxFuture<'static, Result<DispatchCommandResponse, UseCaseError>>;

    /// Update or replace buffer content within a session.
    fn update_buffer(
        &self,
        req: UpdateBufferRequest,
    ) -> BoxFuture<'static, Result<UpdateBufferResponse, UseCaseError>>;

    /// Apply a typed text transaction/edit to an open buffer within a session.
    /// This use-case composes the current editor transient state (cursor/selection)
    /// with the provided transaction and returns both the mutated content and the
    /// updated editor state. The core BufferStore is responsible for actually
    /// applying the edit to the underlying string/rope.
    fn apply_text_transaction(
        &self,
        req: ApplyTextTransactionRequest,
    ) -> BoxFuture<'static, Result<ApplyTextTransactionResponse, UseCaseError>>;

    /// Query recent command history for a session.
    fn get_recent_commands(
        &self,
        req: GetRecentCommandsRequest,
    ) -> BoxFuture<'static, Result<GetRecentCommandsResponse, UseCaseError>>;

    /// Query recent workspace events for a session.
    fn get_recent_events(
        &self,
        req: GetRecentEventsRequest,
    ) -> BoxFuture<'static, Result<GetRecentEventsResponse, UseCaseError>>;

    /// Read-only snapshot query for the current workspace session.
    /// Returns a compact, explicit read-model of the session state.
    fn get_session_snapshot(
        &self,
        req: GetSessionSnapshotRequest,
    ) -> BoxFuture<'static, Result<GetSessionSnapshotResponse, UseCaseError>>;

    /// Create a typed checkpoint capturing a session snapshot suitable for restore.
    fn create_checkpoint(
        &self,
        req: CreateCheckpointRequest,
    ) -> BoxFuture<'static, Result<CreateCheckpointResponse, UseCaseError>>;

    /// Persist a previously created or freshly created checkpoint using the durability port.
    fn save_checkpoint(
        &self,
        req: SaveCheckpointRequest,
    ) -> BoxFuture<'static, Result<SaveCheckpointResponse, UseCaseError>>;

    /// Load a checkpoint from the durability port and restore it into the orchestrator.
    fn load_checkpoint(
        &self,
        req: LoadCheckpointRequest,
    ) -> BoxFuture<'static, Result<LoadCheckpointResponse, UseCaseError>>;

    /// Restore a session from a checkpoint. Returns the restored session and optional replaced id.
    fn restore_checkpoint(
        &self,
        req: RestoreCheckpointRequest,
    ) -> BoxFuture<'static, Result<RestoreCheckpointResponse, UseCaseError>>;

    // ----------------------------
    // Session close helpers (small, conservative defaults)
    // ----------------------------
    /// Attempt to close the given session. Implementations should return `Closed`
    /// when there are no dirty buffers and it is safe to close immediately. When
    /// dirty buffers exist return `BlockedByDirty` listing affected buffer ids.
    fn attempt_close_session(
        &self,
        _req: GetSessionSnapshotRequest,
    ) -> BoxFuture<'static, Result<GetSessionSnapshotResponse, UseCaseError>> {
        // Conservative default: forward to get_session_snapshot (no layering violation),
        // but leave decision to the application. By default we simply return the snapshot
        // so callers may inspect buffer list; higher-level orchestrators should provide
        // a richer attempt_close_session when they can determine dirty state.
        let req =
            GetSessionSnapshotRequest { session_id: _req.session_id.clone(), recent_limit: 0 };
        self.get_session_snapshot(req)
    }

    /// Resolve a previously-blocked session close by saving all buffers and allowing close.
    /// Default implementation attempts a durability save via `save_checkpoint` and reports success.
    fn resolve_close_session_save_all(
        &self,
        req: SaveCheckpointRequest,
    ) -> BoxFuture<'static, Result<SaveCheckpointResponse, UseCaseError>> {
        // Default: delegate to save_checkpoint to persist a durable checkpoint and
        // return the checkpoint location as the success signal.
        self.save_checkpoint(req)
    }

    /// Resolve a previously-blocked session close by discarding all unsaved changes and allowing close.
    /// Default implementation signals success; orchestrators that need to perform reload should override.
    fn resolve_close_session_discard_all(
        &self,
        _req: SaveCheckpointRequest,
    ) -> BoxFuture<'static, Result<SaveCheckpointResponse, UseCaseError>> {
        // Default: no-op discard (caller will proceed to close). Return a dummy success.
        Box::pin(async move { Ok(SaveCheckpointResponse { location: String::new() }) })
    }
}

pub type DynWorkspaceService = Arc<dyn WorkspaceService>;

/// Read-only view port: thin query API for buffer-oriented views.
///
/// This trait provides small, read-only queries that a UI/harness can use to
/// obtain buffer content or the active buffer's content for a session.
/// Implementations should be cheap and avoid mutating state.
pub trait WorkspaceView: Send + Sync {
    /// Get the current content for a buffer by id.
    /// Returns Ok(Some(text)) when present, Ok(None) when the buffer has no text,
    /// or an Err(UseCaseError) for session/workspace-related errors when applicable.
    fn get_buffer_content(
        &self,
        buffer_id: BufferId,
    ) -> BoxFuture<'static, Result<Option<String>, UseCaseError>>;

    /// Get the content of the currently active buffer for the provided session.
    /// Returns NoActiveBuffer or UnknownSession as appropriate.
    fn get_active_buffer_content(
        &self,
        session_id: SessionId,
    ) -> BoxFuture<'static, Result<Option<String>, UseCaseError>>;

    /// Read-only query returning a structured editor document for the active buffer
    /// in the given session. The returned document merges the content snapshot with
    /// the transient editor state (cursor + selection) to enable presentation-only
    /// consumers to render or inspect a deterministic view model.
    ///
    /// Errors:
    /// - UnknownSession if the session is not known.
    /// - NoActiveBuffer if the session has no active buffer.
    fn get_active_editor_document(
        &self,
        req: GetActiveEditorDocumentRequest,
    ) -> BoxFuture<'static, Result<GetActiveEditorDocumentResponse, UseCaseError>>;

    /// Get visible lines for a buffer using the stored viewport state managed by the orchestrator.
    fn get_visible_lines(
        &self,
        req: GetVisibleLinesRequest,
    ) -> BoxFuture<'static, Result<GetVisibleLinesResponse, UseCaseError>>;
}

pub type DynWorkspaceView = Arc<dyn WorkspaceView>;
