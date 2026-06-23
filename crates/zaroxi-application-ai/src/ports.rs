// Application AI port: AiClient trait and small DTOs for the first slice.
//
// The intelligence/application-ai crate defines the trait; infra implements it.

use std::fmt;
use std::sync::Arc;
pub use zaroxi_core_editor_buffer::ports::BufferId;
use zaroxi_kernel_types::Id;

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

/// One item in a streamed AI response.
#[derive(Clone, Debug, PartialEq)]
pub enum AiStreamItem {
    /// A streamed token (model output chunk).
    Token(String),
    /// End-of-stream marker.
    Done,
}

/// AiClient port: async request/response, with an optional streaming variant.
pub trait AiClient: Send + Sync {
    /// Non-streaming request: resolves to the full response text.
    fn request(&self, req: AiRequest) -> BoxFuture<'static, Result<AiResponseDTO, AiError>>;

    /// Streaming request: drives `req` and pushes [`AiStreamItem`]s into `tx`
    /// (one or more `Token`s followed by exactly one `Done`).
    ///
    /// The default implementation adapts a non-streaming [`AiClient::request`]
    /// into a stream by tokenizing the full response, so every existing backend
    /// (including the mock) becomes streamable without changes. Backends with
    /// native streaming should override this to emit tokens as they arrive.
    fn request_stream(
        &self,
        req: AiRequest,
        tx: tokio::sync::mpsc::UnboundedSender<AiStreamItem>,
    ) -> BoxFuture<'static, Result<(), AiError>> {
        // `request` returns a 'static future (it does not borrow self), so it
        // can be moved into the spawned/awaited streaming future.
        let fut = self.request(req);
        Box::pin(async move {
            let resp = fut.await?;
            for token in tokenize_for_stream(&resp.text) {
                // Ignore send errors: a dropped receiver just cancels streaming.
                if tx.send(AiStreamItem::Token(token)).is_err() {
                    return Ok(());
                }
            }
            let _ = tx.send(AiStreamItem::Done);
            Ok(())
        })
    }
}

/// Split text into coarse "tokens" (whitespace-preserving word chunks) for the
/// default streaming adapter. Not a real tokenizer — just enough to exercise
/// the streaming path and produce a meaningful tokens/sec for non-streaming
/// backends.
pub fn tokenize_for_stream(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split_inclusive(char::is_whitespace).map(|s| s.to_string()).collect()
}

pub type DynAiClient = Arc<dyn AiClient>;
