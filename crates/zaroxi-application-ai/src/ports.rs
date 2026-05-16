 // Application AI port: AiClient trait and small DTOs for the first slice.
 //
 // The intelligence/application-ai crate defines the trait; infra implements it.

 use std::sync::Arc;
 use std::fmt;
 use zaroxi_kernel_types::Id;
 use zaroxi_core_editor_buffer::ports::BufferId;
 
 /// Boxed future alias for the skeleton (import kernel BoxFuture in real code).
 pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
 
 /// Simple AI response DTO
 #[derive(Clone, Debug)]
 pub struct AiResponseDTO {
     pub text: String,
 }
 
 /// Small Ai error
 #[derive(Clone, Debug)]
 pub struct AiError(pub String);

 impl From<&str> for AiError {
     fn from(s: &str) -> Self {
         AiError(s.to_string())
     }
 }

 impl fmt::Display for AiError {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         write!(f, "{}", self.0)
     }
 }

 /// Structured AI request model (Phase 2).
 #[derive(Clone, Debug)]
 pub struct AiRequest {
     pub session_id: Id,
     pub workspace_id: Id,
     pub buffer_id: BufferId,
     pub content_snapshot: String,
 }

 /// AiClient port: async request/response.
 pub trait AiClient: Send + Sync {
     fn request(&self, req: AiRequest) -> BoxFuture<'static, Result<AiResponseDTO, AiError>>;
 }

 pub type DynAiClient = Arc<dyn AiClient>;
