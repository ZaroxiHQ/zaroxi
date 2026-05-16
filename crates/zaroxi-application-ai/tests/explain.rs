use zaroxi_application_ai::ports as ports;
use zaroxi_application_ai::ports::AiClient;
use zaroxi_kernel_types::Id;

/// Simple unit tests for the application-ai ports and request/response contract.
///
/// These tests ensure the explain-buffer request shape is usable and that
/// adapters implementing AiClient can be exercised in isolation.

struct FakeAi;
impl ports::AiClient for FakeAi {
    fn request(&self, req: ports::AiRequest) -> ports::BoxFuture<'static, Result<ports::AiResponseDTO, ports::AiError>> {
        Box::pin(async move {
            Ok(ports::AiResponseDTO { text: format!("explain: {}", req.buffer_id) })
        })
    }
}

#[tokio::test]
async fn ai_request_roundtrip() {
    let ai = FakeAi;
    let req = ports::AiRequest {
        session_id: Id::new(),
        workspace_id: Id::new(),
        buffer_id: ports::BufferId::from("buf:1"),
        content_snapshot: "fn main() {}".to_string(),
    };
 
    let res = ai.request(req).await.expect("ai responded");
    assert!(res.text.contains("explain: buf:1"));
}
