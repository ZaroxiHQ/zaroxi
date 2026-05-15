/// Minimal workspace state (placeholder for v1).
#[derive(Debug, Clone, Default)]
pub struct WorkspaceState {
    /// Number of open documents (simple metric for now).
    pub open_docs: usize,
    /// Number of workspace items (placeholder).
    pub workspace_items: usize,
}

impl WorkspaceState {
    pub fn new() -> Self {
        Self { open_docs: 0, workspace_items: 0 }
    }

    pub fn set_counts(&mut self, open_docs: usize, workspace_items: usize) {
        self.open_docs = open_docs;
        self.workspace_items = workspace_items;
    }
}
