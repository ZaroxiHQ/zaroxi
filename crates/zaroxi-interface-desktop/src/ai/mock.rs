use crate::ports::BufferId;

/// Thin shim in the interface layer that delegates to the application-level AI mock.
/// Keeps the old `propose_edit` adapter API but routes requests to the application mock
/// so orchestration and policy live in the application crate.
pub struct MockAiProvider;

impl MockAiProvider {
    pub fn new() -> Self {
        MockAiProvider
    }

    /// Propose an edit for the given buffer by delegating to the application mock AiClient.
    /// Returns the full replacement text.
    pub async fn propose_edit(&self, buffer_id: BufferId, content: Option<String>) -> String {
        // Delegate to application-level mock client.
        let client = zaroxi_application_ai::mock::MockAiClient::new();
        // Build a minimal AiRequest using fresh ids for session/workspace since this shim
        // is presentation-only and callers should pass authoritative session info when available.
        let ai_req = zaroxi_application_ai::ports::AiRequest {
            session_id: zaroxi_kernel_types::Id::new(),
            workspace_id: zaroxi_kernel_types::Id::new(),
            buffer_id: buffer_id.clone(),
            content_snapshot: content.clone().unwrap_or_default(),
        };

        match client.request(ai_req).await {
            Ok(res) => format!("// AI Edit: proposed change\n{}", res.text),
            Err(_) => {
                // Fallback to previous simple behavior when ai client fails.
                let body = content.unwrap_or_default();
                format!("// AI Edit: proposed change\n{}", body)
            }
        }
    }
}
