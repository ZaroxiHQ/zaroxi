//! Prompt construction.

use super::context::ContextCollection;
use super::packing::ContextPacker;

/// Build a prompt from context and a query.
pub struct PromptBuilder {
    /// System message template.
    pub system_template: String,
    /// User message template.
    pub user_template: String,
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self {
            system_template: "You are a helpful programming assistant.".to_string(),
            user_template: "Context:\n{context}\n\nQuestion: {query}".to_string(),
        }
    }
}

impl PromptBuilder {
    /// Build a prompt.
    pub fn build(&self, context: &ContextCollection, query: &str, max_tokens: usize) -> String {
        let packer = ContextPacker::new(max_tokens);
        let packed = packer.pack(context);

        let user = self.user_template.replace("{context}", &packed).replace("{query}", query);

        format!("{}\n\n{}", self.system_template, user)
    }
}
