 // Application workspace orchestrator trait skeleton.
 //
 // This trait composes domain and core ports to implement use cases like "open workspace"
 // and "open buffer" from the UI. Keep it minimal for the first slice.

 use std::path::PathBuf;
 use std::sync::Arc;
 use crate as _; // placeholder for crate root
 use serde::{Serialize, Deserialize};

 use std::pin::Pin;
 use std::future::Future;

 /// Boxed future alias (replace with kernel::BoxFuture in real code)
 pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

 /// DTO: workspace session created by the application
 #[derive(Clone, Debug, Serialize, Deserialize)]
 pub struct WorkspaceSessionDTO {
     pub session_id: String,
     pub workspace_id: String,
 }

 /// Command used to request open workspace
 #[derive(Clone, Debug)]
 pub struct WorkspaceOpenCommand {
     pub path: PathBuf,
 }

 /// Application-level commands that the UI may dispatch.
 /// Keep this small for Phase 0: an AI explain command and a simple edit command.
 #[derive(Clone, Debug)]
 pub enum AppCommand {
     AiExplain { prompt: String },
     InsertText { buffer_id: String, offset: usize, text: String },
 }

 /// Result returned from a dispatched command.
 #[derive(Clone, Debug)]
 pub struct CommandResult {
     pub message: String,
 }

 /// Very small service trait. Implementations are in application layer.
 pub trait WorkspaceService: Send + Sync {
     /// Open a workspace and create a session for UI. Returns a session DTO.
     fn open_workspace(&self, cmd: WorkspaceOpenCommand) -> BoxFuture<'static, Result<WorkspaceSessionDTO, String>>;

     /// Open a buffer inside an active session (session_id is a string for the skeleton).
     fn open_buffer(&self, session_id: String, path: PathBuf) -> BoxFuture<'static, Result<String /* buffer id */, String>>;

     /// Dispatch a high-level application command (AI requests, edits, etc).
     /// The application service is responsible for routing this to the correct port (ai client, buffer store, etc).
     fn dispatch_command(&self, session_id: String, cmd: AppCommand) -> BoxFuture<'static, Result<CommandResult, String>>;
 }

 pub type DynWorkspaceService = Arc<dyn WorkspaceService>;
