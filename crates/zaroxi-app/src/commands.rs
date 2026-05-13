use crate::panels::BottomPanel;
use zaroxi_foundation::DocumentId;

/// High-level application commands used by UI and runtime.
///
/// Commands are intentionally explicit (no catch-all) to make the pipeline
/// clear and maintainable.
#[derive(Debug, Clone)]
pub enum AppCommand {
    // Workspace
    OpenFolder { path: String },
    OpenFile { path: String },
    SelectSidebarItem { index: Option<usize> },

    // Tabs / Documents
    OpenDocument { doc_id: DocumentId },
    CloseTab { doc_id: DocumentId },
    ActivateTab { doc_id: DocumentId },

    // Editor
    InsertText { doc_id: DocumentId, offset: usize, text: String },
    SaveActiveDocument,

    // Panels
    ToggleBottomPanel,
    SetBottomPanel { panel: BottomPanel },

    // Assistant
    SetAssistantInput { input: String },
    SendAssistantPrompt,
    InsertAssistantSuggestion { doc_id: DocumentId, text: String },

    // Status
    SetStatusMessage { message: String },
}
