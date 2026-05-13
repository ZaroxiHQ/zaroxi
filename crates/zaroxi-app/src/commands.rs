use zaroxi_foundation::DocumentId;

/// High-level application commands used by UI and runtime.
///
/// Commands are intentionally explicit (no catch-all) to make the pipeline
/// clear and maintainable.
#[derive(Debug, Clone)]
pub enum AppCommand {
    OpenFolder { path: String },
    OpenFile { path: String },
    SelectSidebarItem { index: Option<usize> },
    SetStatusMessage { message: String },
    ActivateDocument { doc_id: DocumentId },
    InsertText { doc_id: DocumentId, offset: usize, text: String },
    SaveActiveDocument,
}
