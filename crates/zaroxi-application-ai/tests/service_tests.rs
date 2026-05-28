use std::sync::Arc;

use zaroxi_domain_ai::types::AiEditRequest;
use zaroxi_application_ai::service::AiService;
use zaroxi_application_ai::ports::{AiClient, AiRequest, AiResponseDTO, AiError, BoxFuture};

struct FakeClient;

impl AiClient for FakeClient {
    fn request(&self, req: AiRequest) -> BoxFuture<'static, Result<AiResponseDTO, AiError>> {
        Box::pin(async move {
            Ok(AiResponseDTO { text: format!("mocked response for {}", req.buffer_id) })
        })
    }
}

#[tokio::test]
async fn ai_service_request_creates_proposal_and_stores_it() {
    let svc = AiService::new();
    let client: Arc<dyn AiClient> = Arc::new(FakeClient);

    let req = AiEditRequest {
        session_id: "s1".to_string(),
        buffer_id: "buf:main.rs".to_string(),
        content: "fn main() {}".to_string(),
    };

    let proposal = svc.request_ai_edit(req, client).await.expect("request ok");

    // Basic assertions about the returned proposal
    assert!(proposal.id.len() > 0);
    assert_eq!(proposal.buffer_id, "buf:main.rs");
    assert!(proposal.summary.len() > 0);
    assert!(proposal.proposal_text.contains("mocked response for"));
}
