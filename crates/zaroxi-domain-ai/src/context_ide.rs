//! IDE context ingestion model for AI conversations.
//!
//! Defines the types for collecting, classifying, and assembling IDE context
//! (current file, selection, workspace info, diagnostics, etc.) to attach to
//! AI requests. These are pure domain types — no rendering or retrieval logic.

use serde::{Deserialize, Serialize};

/// What kind of context source produced this item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextSourceKind {
    ActiveFile,
    EditorSelection,
    EditorViewport,
    WorkspaceRoot,
    OpenTabs,
    Diagnostics,
    TerminalOutput,
    GitStatus,
    UserAttached,
    McpContext,
}

impl ContextSourceKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ContextSourceKind::ActiveFile => "Active File",
            ContextSourceKind::EditorSelection => "Selection",
            ContextSourceKind::EditorViewport => "Visible Content",
            ContextSourceKind::WorkspaceRoot => "Workspace",
            ContextSourceKind::OpenTabs => "Open Tabs",
            ContextSourceKind::Diagnostics => "Diagnostics",
            ContextSourceKind::TerminalOutput => "Terminal",
            ContextSourceKind::GitStatus => "Git Status",
            ContextSourceKind::UserAttached => "Attached",
            ContextSourceKind::McpContext => "MCP",
        }
    }

    pub fn is_auto_attached(&self) -> bool {
        matches!(
            self,
            ContextSourceKind::ActiveFile
                | ContextSourceKind::EditorSelection
                | ContextSourceKind::WorkspaceRoot
        )
    }
}

/// Attachment policy: when is this context included in requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentPolicy {
    Auto,
    OptOut,
    OptIn,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdeContextItem {
    pub source_kind: ContextSourceKind,
    pub label: String,
    pub content: String,
    pub approximate_tokens: usize,
    pub policy: AttachmentPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextPack {
    pub items: Vec<IdeContextItem>,
    pub total_tokens: usize,
    pub max_context_tokens: usize,
}

impl Default for ContextPack {
    fn default() -> Self {
        Self { items: Vec::new(), total_tokens: 0, max_context_tokens: 32_000 }
    }
}

impl ContextPack {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_context_tokens: max_tokens, ..Default::default() }
    }

    pub fn try_add(&mut self, item: IdeContextItem) -> bool {
        if self.total_tokens + item.approximate_tokens > self.max_context_tokens {
            return false;
        }
        self.total_tokens += item.approximate_tokens;
        self.items.push(item);
        true
    }

    pub fn has_any_context(&self) -> bool {
        !self.items.is_empty()
    }

    pub fn format_for_prompt(&self) -> String {
        let mut out = String::new();
        for item in &self.items {
            out.push_str(&format!(
                "<context source=\"{}\" label=\"{}\">\n{}\n</context>\n\n",
                item.source_kind.display_name(),
                item.label,
                item.content
            ));
        }
        out
    }

    pub fn auto_attached(&self) -> Vec<&IdeContextItem> {
        self.items.iter().filter(|i| i.policy == AttachmentPolicy::Auto).collect()
    }
}

/// Builder for assembling a context pack from IDE state.
pub struct ContextPackBuilder {
    max_tokens: usize,
    items: Vec<IdeContextItem>,
}

impl ContextPackBuilder {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens, items: Vec::new() }
    }

    fn tokens(&self) -> usize {
        self.items.iter().map(|i| i.approximate_tokens).sum()
    }

    fn can_add(&self, estimate: usize) -> bool {
        self.tokens() + estimate <= self.max_tokens
    }

    fn push(&mut self, item: IdeContextItem) {
        self.items.push(item);
    }

    pub fn add_active_file(mut self, path: &str, content: &str, token_estimate: usize) -> Self {
        if self.can_add(token_estimate) {
            self.push(IdeContextItem {
                source_kind: ContextSourceKind::ActiveFile,
                label: path.to_string(),
                content: content.to_string(),
                approximate_tokens: token_estimate,
                policy: AttachmentPolicy::Auto,
            });
        }
        self
    }

    pub fn add_selection(mut self, label: &str, text: &str, token_estimate: usize) -> Self {
        if self.can_add(token_estimate) {
            self.push(IdeContextItem {
                source_kind: ContextSourceKind::EditorSelection,
                label: label.to_string(),
                content: text.to_string(),
                approximate_tokens: token_estimate,
                policy: AttachmentPolicy::Auto,
            });
        }
        self
    }

    pub fn add_workspace_root(mut self, path: &str, name: &str, token_estimate: usize) -> Self {
        if self.can_add(token_estimate) {
            self.push(IdeContextItem {
                source_kind: ContextSourceKind::WorkspaceRoot,
                label: name.to_string(),
                content: format!("Workspace: {path}"),
                approximate_tokens: token_estimate,
                policy: AttachmentPolicy::Auto,
            });
        }
        self
    }

    pub fn add_diagnostics(mut self, summary: &str, token_estimate: usize) -> Self {
        if self.can_add(token_estimate) {
            self.push(IdeContextItem {
                source_kind: ContextSourceKind::Diagnostics,
                label: "Problems".into(),
                content: summary.to_string(),
                approximate_tokens: token_estimate,
                policy: AttachmentPolicy::OptOut,
            });
        }
        self
    }

    pub fn add_open_tabs(mut self, tabs: &[String], token_estimate: usize) -> Self {
        let content = tabs.join("\n");
        let est = token_estimate.min(content.len() / 4);
        if self.can_add(est) {
            self.push(IdeContextItem {
                source_kind: ContextSourceKind::OpenTabs,
                label: format!("{} open tabs", tabs.len()),
                content,
                approximate_tokens: est,
                policy: AttachmentPolicy::OptIn,
            });
        }
        self
    }

    pub fn add_git_status(mut self, summary: &str, token_estimate: usize) -> Self {
        if self.can_add(token_estimate) {
            self.push(IdeContextItem {
                source_kind: ContextSourceKind::GitStatus,
                label: "Git".into(),
                content: summary.to_string(),
                approximate_tokens: token_estimate,
                policy: AttachmentPolicy::OptIn,
            });
        }
        self
    }

    pub fn build(self) -> ContextPack {
        let total_tokens: usize = self.items.iter().map(|i| i.approximate_tokens).sum();
        ContextPack { items: self.items, total_tokens, max_context_tokens: self.max_tokens }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_attached_sources() {
        assert!(ContextSourceKind::ActiveFile.is_auto_attached());
        assert!(ContextSourceKind::EditorSelection.is_auto_attached());
        assert!(ContextSourceKind::WorkspaceRoot.is_auto_attached());
        assert!(!ContextSourceKind::OpenTabs.is_auto_attached());
        assert!(!ContextSourceKind::Diagnostics.is_auto_attached());
    }

    #[test]
    fn context_pack_budget_respected() {
        let mut pack = ContextPack::new(100);
        let added = pack.try_add(IdeContextItem {
            source_kind: ContextSourceKind::ActiveFile,
            label: "main.rs".into(),
            content: "fn main() {}".into(),
            approximate_tokens: 80,
            policy: AttachmentPolicy::Auto,
        });
        assert!(added);
        let added2 = pack.try_add(IdeContextItem {
            source_kind: ContextSourceKind::Diagnostics,
            label: "Problems".into(),
            content: "no errors".into(),
            approximate_tokens: 30,
            policy: AttachmentPolicy::OptOut,
        });
        assert!(!added2);
    }

    #[test]
    fn builder_adds_auto_items() {
        let pack = ContextPackBuilder::new(1000)
            .add_active_file("src/main.rs", "fn main() {}", 5)
            .add_selection("L1-L3", "hello", 3)
            .add_workspace_root("/home/project", "my-project", 4)
            .build();

        assert_eq!(pack.items.len(), 3);
        assert_eq!(pack.total_tokens, 12);
        assert_eq!(pack.auto_attached().len(), 3);
    }

    #[test]
    fn format_for_prompt_includes_all_sources() {
        let pack = ContextPackBuilder::new(1000).add_active_file("test.txt", "content", 2).build();
        let formatted = pack.format_for_prompt();
        assert!(formatted.contains("<context source=\"Active File\""));
        assert!(formatted.contains("content"));
    }
}
