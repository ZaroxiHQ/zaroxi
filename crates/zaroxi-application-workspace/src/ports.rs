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

 use std::pin::Pin;
 use std::future::Future;

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
     pub buffer_id: String,
 }

 /// Request to list opened buffers for a session.
 #[derive(Clone, Debug)]
 pub struct ListBuffersRequest {
     pub session_id: SessionId,
 }

 /// Response listing opened buffers and the active buffer (if any).
 #[derive(Clone, Debug)]
 pub struct ListBuffersResponse {
     pub buffer_ids: Vec<String>,
     pub active_buffer: Option<String>,
 }

 /// Request to set the active buffer for a session.
 #[derive(Clone, Debug)]
 pub struct SetActiveBufferRequest {
     pub session_id: SessionId,
     pub buffer_id: String,
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
     pub buffer_id: String,
 }
 
 /// Request to update buffer content.
 #[derive(Clone, Debug)]
 pub struct UpdateBufferRequest {
     pub session_id: SessionId,
     pub buffer_id: String,
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
         }
     }
 }
 
 impl std::error::Error for UseCaseError {}
 

 /// Application-level commands that the UI may dispatch.
 /// Phase 2: AiExplain is buffer-focused and does not carry a free-form prompt here.
 #[derive(Clone, Debug)]
 pub enum AppCommand {
     AiExplain { buffer_id: String },
     InsertText { buffer_id: String, offset: usize, text: String },
 }

 /// Request to dispatch an application command.
 #[derive(Clone, Debug)]
 pub struct DispatchCommandRequest {
     pub session_id: SessionId,
     pub command: AppCommand,
 }

 /// Result returned from a dispatched command.
 #[derive(Clone, Debug)]
 pub struct CommandResult {
     pub message: String,
 }

 /// Response from dispatch command
 #[derive(Clone, Debug)]
 pub struct DispatchCommandResponse {
     pub result: CommandResult,
 }

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
 }

 pub type DynWorkspaceService = Arc<dyn WorkspaceService>;
