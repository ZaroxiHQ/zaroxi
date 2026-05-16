 // Application workspace orchestrator trait skeleton.
 //
 // This trait composes domain and core ports to implement use cases like "open workspace"
 // and "open buffer" from the UI. Keep it minimal for the first slice.

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
 
 /// Typed errors for application use-cases (Phase 4).
 #[derive(Clone, Debug)]
 pub enum UseCaseError {
     UnknownSession,
     UnknownWorkspace,
     UnknownBuffer,
     InvalidMutation(String),
     AiFailure(String),
 }
 
 impl std::fmt::Display for UseCaseError {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
         match self {
             UseCaseError::UnknownSession => write!(f, "unknown session"),
             UseCaseError::UnknownWorkspace => write!(f, "unknown workspace"),
             UseCaseError::UnknownBuffer => write!(f, "unknown buffer"),
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
 /// Methods are explicit use-case entry points for Phase 2.
 pub trait WorkspaceService: Send + Sync {
     /// Boot/open a workspace and create a UI session.
     fn boot_workspace(&self, req: WorkspaceBootRequest) -> BoxFuture<'static, Result<WorkspaceBootResponse, String>>;

     /// Open a buffer inside an active session.
     fn open_buffer(&self, req: OpenBufferRequest) -> BoxFuture<'static, Result<OpenBufferResponse, String>>;

     /// Dispatch a high-level application command (AI requests, edits, etc).
     fn dispatch_command(&self, req: DispatchCommandRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, String>>;
 }

 pub type DynWorkspaceService = Arc<dyn WorkspaceService>;
