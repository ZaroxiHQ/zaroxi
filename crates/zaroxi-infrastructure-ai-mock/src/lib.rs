// Lightweight mock AI adapter implementing application-ai::AiClient.
// This crate is an infra adapter used only in Phase 0 / Phase 1 to validate wiring.
//
// The implementation returns a canned response after a tiny async delay.

// Avoid requiring the tokio `time` feature in the workspace; no artificial sleep needed in the mock.

// Import the application-owned port types. Use the public crate name of the application-ai crate.
use zaroxi_application_ai::ports::{AiClient, AiError, AiRequest, AiResponseDTO, BoxFuture};

/// MockAiClient implements AiClient and returns a canned response.
pub struct MockAiClient;

impl MockAiClient {
    pub fn new() -> Self {
        MockAiClient
    }
}

impl AiClient for MockAiClient {
    fn request(&self, req: AiRequest) -> BoxFuture<'static, Result<AiResponseDTO, AiError>> {
        Box::pin(async move {
            // Mock adapter: immediate response (no artificial sleep to avoid tokio::time feature).
            // Echo a helpful mocked explanation including a bit of the content snapshot.
            let snippet = if req.content_snapshot.len() > 80 {
                format!("{}...", &req.content_snapshot[..80])
            } else {
                req.content_snapshot.clone()
            };
            let reply = format!("(mocked) explanation for buffer {}: {}", req.buffer_id, snippet);
            Ok(AiResponseDTO { text: reply })
        })
    }
}

// Export a boxed dynamic adapter helper.
pub fn into_dyn(client: MockAiClient) -> std::sync::Arc<dyn AiClient> {
    std::sync::Arc::new(client)
}
