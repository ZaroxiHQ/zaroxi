 // Application workspace orchestrator trait skeleton - Phase 5 enhancements.
 //
 // This trait composes domain and core ports to implement use cases like "open workspace",
 // multi-buffer management (list/set/get active buffer) and explain-active-buffer.
 // Keep surface minimal and typed for Phase 5.

 use std::path::PathBuf;
 use std::sync::Arc;
 use crate as _; // placeholder for crate root
 use serde::{Serialize, Deserialize};
 use zaroxi_kernel_types::Id;
 use zaroxi_core_editor_buffer::ports::BufferId;
 
 use std::pin::Pin;
 use std::future::Future;
 use chrono::{DateTime, Utc};
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
 pub use zaroxi_application_command::ports::{AppCommand, CommandKind, CommandRecord, CommandResult, DispatchCommandResponse};
 
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
     fn get_recent_commands(&self, session_id: SessionId, limit: usize) -> BoxFuture<'static, Result<Vec<CommandRecord>, String>>;
     fn get_recent_events(&self, session_id: SessionId, limit: usize) -> BoxFuture<'static, Result<Vec<WorkspaceEvent>, String>>;
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
     fn save_checkpoint(&self, checkpoint: Checkpoint) -> BoxFuture<'static, Result<String, DurabilityError>>;
 
     /// Load a checkpoint by location id and return the deserialized checkpoint.
     fn load_checkpoint(&self, location: String) -> BoxFuture<'static, Result<Checkpoint, DurabilityError>>;
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
 
 /// Response from restoring a checkpoint.
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
 pub trait WorkspaceService: Send + Sync {
     /// Boot/open a workspace and create a UI session.
     fn boot_workspace(&self, req: WorkspaceBootRequest) -> BoxFuture<'static, Result<WorkspaceBootResponse, UseCaseError>>;
 
     /// Open a buffer inside an active session.
     fn open_buffer(&self, req: OpenBufferRequest) -> BoxFuture<'static, Result<OpenBufferResponse, UseCaseError>>;
 
     /// List opened buffers for a session and indicate the active buffer (if any).
     fn list_open_buffers(&self, req: ListBuffersRequest) -> BoxFuture<'static, Result<ListBuffersResponse, UseCaseError>>;
 
     /// Set the active buffer for a session.
     fn set_active_buffer(&self, req: SetActiveBufferRequest) -> BoxFuture<'static, Result<SetActiveBufferResponse, UseCaseError>>;
 
     /// Get the currently active buffer for a session.
     fn get_active_buffer(&self, req: GetActiveBufferRequest) -> BoxFuture<'static, Result<GetActiveBufferResponse, UseCaseError>>;
 
     /// Shorthand use-case: explain the currently active buffer (uses the AiClient).
     fn explain_active_buffer(&self, req: GetActiveBufferRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, UseCaseError>>;
 
     /// Dispatch a high-level application command (AI requests, edits, etc).
     fn dispatch_command(&self, req: DispatchCommandRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, UseCaseError>>;
 
     /// Update or replace buffer content within a session.
     fn update_buffer(&self, req: UpdateBufferRequest) -> BoxFuture<'static, Result<UpdateBufferResponse, UseCaseError>>;
 
     /// Query recent command history for a session.
     fn get_recent_commands(&self, req: GetRecentCommandsRequest) -> BoxFuture<'static, Result<GetRecentCommandsResponse, UseCaseError>>;
 
     /// Query recent workspace events for a session.
     fn get_recent_events(&self, req: GetRecentEventsRequest) -> BoxFuture<'static, Result<GetRecentEventsResponse, UseCaseError>>;
 
     /// Read-only snapshot query for the current workspace session.
     /// Returns a compact, explicit read-model of the session state.
     fn get_session_snapshot(&self, req: GetSessionSnapshotRequest) -> BoxFuture<'static, Result<GetSessionSnapshotResponse, UseCaseError>>;
 
     /// Create a typed checkpoint capturing a session snapshot suitable for restore.
     fn create_checkpoint(&self, req: CreateCheckpointRequest) -> BoxFuture<'static, Result<CreateCheckpointResponse, UseCaseError>>;
 
     /// Persist a previously created or freshly created checkpoint using the durability port.
     fn save_checkpoint(&self, req: SaveCheckpointRequest) -> BoxFuture<'static, Result<SaveCheckpointResponse, UseCaseError>>;
 
     /// Load a checkpoint from the durability port and restore it into the orchestrator.
     fn load_checkpoint(&self, req: LoadCheckpointRequest) -> BoxFuture<'static, Result<LoadCheckpointResponse, UseCaseError>>;
 
     /// Restore a session from a checkpoint. Returns the restored session and optional replaced id.
     fn restore_checkpoint(&self, req: RestoreCheckpointRequest) -> BoxFuture<'static, Result<RestoreCheckpointResponse, UseCaseError>>;
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
     fn get_buffer_content(&self, buffer_id: BufferId) -> BoxFuture<'static, Result<Option<String>, UseCaseError>>;

     /// Get the content of the currently active buffer for the provided session.
     /// Returns NoActiveBuffer or UnknownSession as appropriate.
     fn get_active_buffer_content(&self, session_id: SessionId) -> BoxFuture<'static, Result<Option<String>, UseCaseError>>;
 }

 pub type DynWorkspaceView = Arc<dyn WorkspaceView>;
