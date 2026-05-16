// Application AI port: AiClient trait and small DTOs for the first slice.
//
// The intelligence/application-ai crate defines the trait; infra implements it.

use std::sync::Arc;
use std::fmt;

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

/// AiClient port: async request/response.
pub trait AiClient: Send + Sync {
    fn request(&self, prompt: String) -> BoxFuture<'static, Result<AiResponseDTO, AiError>>;
}

pub type DynAiClient = Arc<dyn AiClient>;
