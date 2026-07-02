use crate::ports::BufferId;

/// Deterministic mock AI provider used for Phase 10.
/// Behavior: produce a simple prepend-comment proposal containing the original content.
///
/// Note: this shim intentionally keeps the interface-local behavior minimal and
/// avoids introducing a direct dependency on the application ai crate here so that
/// desktop remains a thin adapter. The application-level mock lives in
/// `crates/zaroxi-application-ai` and will be used by orchestrators; this shim
/// keeps previous presentation-only semantics for tests/harnesses that call it.
pub struct MockAiProvider;

impl Default for MockAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAiProvider {
    pub fn new() -> Self {
        MockAiProvider
    }

    /// Propose an edit for the given buffer. Returns the full replacement text.
    pub async fn propose_edit(&self, _buffer_id: BufferId, content: Option<String>) -> String {
        let body = content.unwrap_or_default();
        format!("// AI Edit: proposed change\n{}", body)
    }
}
