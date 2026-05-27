use zaroxi_kernel_types::Id;

/// Minimal set of editor core commands.
/// These are intentionally small; app-level commands will live in `zaroxi-app`.
#[derive(Debug, Clone)]
pub enum EditorCommand {
    /// Insert text at a character offset in the specified document.
    InsertText { doc_id: Id, offset: usize, text: String },
}
