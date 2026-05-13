use crate::commands::AppCommand;
use crate::status::StatusState;
use zaroxi_config::AppConfig;
use zaroxi_editor_core::EditorState;
use zaroxi_editor_buffer::Document;
use zaroxi_workspace::{WorkspaceState, WorkspaceItem};

/// Top-level app state assembled from domain parts.
///
/// The runtime/renderer should observe or borrow this state to render UI. All
/// mutations should be expressed via AppCommand and handled through a single
/// entry-point (for now `apply` on AppState).
#[derive(Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub workspace: WorkspaceState,
    pub editor: EditorState,
    pub status: StatusState,
}

impl AppState {
    /// Create a new AppState using provided config and sensible defaults.
    pub fn new(config: &AppConfig) -> Self {
        // workspace placeholders
        let mut workspace = WorkspaceState::new();
        workspace.items.push(WorkspaceItem::file("README.md", Some("README.md".to_string())));
        workspace.items.push(WorkspaceItem::file("src/main.rs", Some("src/main.rs".to_string())));
        workspace.items.push(WorkspaceItem::file("Cargo.toml", Some("Cargo.toml".to_string())));
        workspace.select(Some(0));

        // editor with a welcome document
        let mut editor = EditorState::new();
        let welcome = Document::welcome();
        editor.open_document(welcome);

        let status = StatusState::default();

        Self {
            config: config.clone(),
            workspace,
            editor,
            status,
        }
    }

    /// Apply an application-level command. This mutates the app state in a
    /// single place, keeping the command pipeline explicit and easy to audit.
    pub fn apply(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::OpenFolder { path: _ } => {
                // placeholder: set root and mark status
                self.status.message = "Opened folder (placeholder)".to_string();
            }
            AppCommand::OpenFile { path: _ } => {
                // placeholder: update status only for v1
                self.status.message = "Opened file (placeholder)".to_string();
            }
            AppCommand::SelectSidebarItem { index } => {
                self.workspace.select(index);
            }
            AppCommand::SetStatusMessage { message } => {
                self.status.message = message;
            }
            AppCommand::ActivateDocument { doc_id } => {
                self.editor.active_document = Some(doc_id);
            }
            AppCommand::InsertText { doc_id, offset, text } => {
                self.editor.apply(zaroxi_editor_core::EditorCommand::InsertText {
                    doc_id,
                    offset,
                    text,
                });
            }
            AppCommand::SaveActiveDocument => {
                self.status.message = "Save requested (placeholder)".to_string();
            }
        }
    }
}
