use crate::ports as ai_ports;
use std::pin::Pin;
use std::future::Future;

/// Deterministic application-level mock AiClient used for Phase 10.
/// Behavior: return a simple text response based on the buffer id.
pub struct MockAiClient;

impl MockAiClient {
    pub fn new() -> Self {
        MockAiClient
    }
}

impl ai_ports::AiClient for MockAiClient {
    fn request(
        &self,
        req: ai_ports::AiRequest,
    ) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
        let buf = req.buffer_id.clone();
        Box::pin(async move {
            let text = format!("fake-explain: {}", buf);
            Ok(ai_ports::AiResponseDTO { text })
        })
    }
}
