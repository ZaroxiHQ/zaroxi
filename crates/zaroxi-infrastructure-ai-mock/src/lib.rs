// Lightweight mock AI adapter implementing application-ai::AiClient.
// This crate is an infra adapter used only in Phase 0 / Phase 1 to validate wiring.
//
// The implementation returns a canned response after a tiny async delay.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep; // note: using tokio in the composition harness is acceptable for the slice

use crates::zaroxi_application_ai::ports::{AiClient, AiResponseDTO, AiError}; // placeholder import path
// In the real workspace the path will be: use zaroxi_application_ai::ports::{...};

/// MockAiClient implements AiClient and returns a canned response.
pub struct MockAiClient;

impl MockAiClient {
    pub fn new() -> Self {
        MockAiClient
    }
}

impl AiClient for MockAiClient {
    fn request(&self, prompt: String) -> crate::zaroxi_application_ai::ports::BoxFuture<'static, Result<AiResponseDTO, AiError>> {
        Box::pin(async move {
            // simulate latency
            sleep(Duration::from_millis(50)).await;
            let reply = format!("(mocked) explanation for prompt: {}", prompt);
            Ok(AiResponseDTO { text: reply })
        })
    }
}

// Export a boxed dynamic adapter helper.
pub fn into_dyn(client: MockAiClient) -> std::sync::Arc<dyn AiClient> {
    std::sync::Arc::new(client)
}
