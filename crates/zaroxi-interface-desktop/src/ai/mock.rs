use crate::ports::BufferId;

/// Deterministic mock AI provider used for Phase 10.
/// Behavior: produce a simple prepend-comment proposal containing the original content.
pub struct MockAiProvider;

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
