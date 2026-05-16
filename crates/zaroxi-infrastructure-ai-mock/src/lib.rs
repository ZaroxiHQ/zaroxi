 // Lightweight mock AI adapter implementing application-ai::AiClient.
 // This crate is an infra adapter used only in Phase 0 / Phase 1 to validate wiring.
 //
 // The implementation returns a canned response after a tiny async delay.

 use std::sync::Arc;
 use std::time::Duration;
 use std::pin::Pin;
 use std::future::Future;

 use tokio::time::sleep; // acceptable for the harness

 // Import the application-owned port types. Use the public crate name of the application-ai crate.
 use zaroxi_application_ai::ports::{AiClient, AiResponseDTO, AiError, BoxFuture};

 /// MockAiClient implements AiClient and returns a canned response.
 pub struct MockAiClient;

 impl MockAiClient {
     pub fn new() -> Self {
         MockAiClient
     }
 }

 impl AiClient for MockAiClient {
     fn request(&self, prompt: String) -> BoxFuture<'static, Result<AiResponseDTO, AiError>> {
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
