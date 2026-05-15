//! Context packing for prompt construction.

use super::context::ContextCollection;

/// Pack context items into a prompt, respecting token limits.
pub struct ContextPacker {
    /// Maximum tokens allowed.
    pub max_tokens: usize,
}

impl ContextPacker {
    /// Create a new packer with the given token limit.
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Pack context items into a string.
    pub fn pack(&self, collection: &ContextCollection) -> String {
        let mut result = String::new();
        let mut used_tokens = 0;

        for item in &collection.items {
            // Simple token estimation: 1 token ≈ 4 characters
            let estimated_tokens = item.content.chars().count() / 4;
            if used_tokens + estimated_tokens > self.max_tokens {
                break;
            }
            result.push_str(&format!("[{}] {}\n", item.source, item.content));
            used_tokens += estimated_tokens;
        }

        result
    }
}
