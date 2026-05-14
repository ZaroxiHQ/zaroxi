use crate::commands::AppCommand;
use crate::status::StatusState;
use crate::assistant::AssistantState;
use crate::panels::BottomPanelState;
use crate::tabs::TabState;
use log::info;
use zaroxi_config::AppConfig;
use zaroxi_editor_core::EditorState;
use zaroxi_editor_buffer::Document;
use zaroxi_workspace::{WorkspaceState, WorkspaceItem};
use zaroxi_theme::ZaroxiTheme;

/// A simple panel descriptor owned by the application layer.
///
/// Each panel has a stable id, a human-facing title, visibility, and a
/// small placeholder content string. The renderer will consume these panel
/// entries (via AppState) to render labels and content; the runtime/layout
/// layer maps them to pixel rects.
#[derive(Debug, Clone)]
pub struct PanelEntry {
    pub id: &'static str,
    pub title: String,
    pub visible: bool,
    pub content: String,
}

impl PanelEntry {
    pub fn new(id: &'static str, title: impl Into<String>, content: impl Into<String>, visible: bool) -> Self {
        Self {
            id,
            title: title.into(),
            visible,
            content: content.into(),
        }
    }
}

/// Top-level app state assembled from domain parts.
///
/// The runtime/renderer should observe or borrow this state to render UI. All
/// mutations should be expressed via AppCommand and handled through a single
/// entry-point (for now `apply` on AppState`).
#[derive(Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub workspace: WorkspaceState,
    pub editor: EditorState,
    pub status: StatusState,
    pub assistant: AssistantState,
    pub panels: BottomPanelState,
    pub tabs: TabState,
    /// User-selected theme mode (Light/Dark/System).
    pub theme_mode: ZaroxiTheme,
    /// Application-owned panel descriptors (title/content/visibility).
    pub app_panels: Vec<PanelEntry>,
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
        editor.open_document(welcome.clone());

        // tabs: open welcome doc in tabs
        let mut tabs = TabState::new();
        tabs.open_tab_for_document(&welcome);

        let status = StatusState::default();
        let assistant = AssistantState::default();
        let panels = BottomPanelState::default();

        // Create explicit app-owned panel entries (owned by the app layer).
        let mut app_panels = Vec::new();
        app_panels.push(PanelEntry::new("titlebar", "Zaroxi Studio", config.title.clone(), true));
        app_panels.push(PanelEntry::new("sidebar", "Explorer", "Workspace", true));
        app_panels.push(PanelEntry::new("editor", "Editor", welcome.display_name.clone(), true));
        app_panels.push(PanelEntry::new("right_panel", "Assistant", "AI Assistant (v1)", true));
        app_panels.push(PanelEntry::new("bottom_panel", "Terminal", "Terminal (placeholder)", true));
        app_panels.push(PanelEntry::new("status_bar", "Status", "Ready", true));

        // Log created panels for visibility
        for p in &app_panels {
            info!("created panel: {} ({}) visible={}", p.id, p.title, p.visible);
        }

        Self {
            config: config.clone(),
            workspace,
            editor,
            status,
            assistant,
            panels,
            tabs,
            theme_mode: ZaroxiTheme::Dark,
            app_panels,
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

            AppCommand::OpenFile { path } => {
                // Create a simple Document from the path (placeholder content)
                let content = format!("Contents of {}", path);
                let doc = Document::new(path.clone(), content);
                self.editor.open_document(doc.clone());
                self.tabs.open_tab_for_document(&doc);
                self.status.message = format!("Opened {}", path);

                // Reflect active editor title in the editor panel content if present
                if let Some(panel) = self.app_panels.iter_mut().find(|p| p.id == "editor") {
                    panel.content = doc.display_name.clone();
                }
            }

            AppCommand::SelectSidebarItem { index } => {
                self.workspace.select(index);
                if let Some(i) = index {
                    if let Some(item) = self.workspace.items.get(i) {
                        if let Some(path) = &item.path {
                            // simulate opening the selected file
                            let content = format!("Contents of {}", path);
                            let doc = Document::new(path.clone(), content);
                            self.editor.open_document(doc.clone());
                            self.tabs.open_tab_for_document(&doc);
                            self.status.message = format!("Selected {}", path);

                            // update editor panel content
                            if let Some(panel) = self.app_panels.iter_mut().find(|p| p.id == "editor") {
                                panel.content = doc.display_name.clone();
                            }
                        }
                    }
                }
            }

            AppCommand::OpenDocument { doc_id } => {
                self.editor.active_document = Some(doc_id);
                self.tabs.activate_by_doc_id(doc_id);
            }

            AppCommand::CloseTab { doc_id } => {
                self.tabs.close_by_doc_id(doc_id);
                // If the closed tab was the active document, clear editor active_document if not present
                if let Some(active) = self.editor.active_document {
                    if active == doc_id {
                        // set active to the tabs' active doc if any
                        if let Some(active_doc_id) = self.tabs.active_doc_id() {
                            self.editor.active_document = Some(active_doc_id);
                        } else {
                            self.editor.active_document = None;
                        }
                    }
                }
            }

            AppCommand::ActivateTab { doc_id } => {
                self.tabs.activate_by_doc_id(doc_id);
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

            AppCommand::ToggleBottomPanel => {
                self.panels.visible = !self.panels.visible;
            }

            AppCommand::SetBottomPanel { panel } => {
                self.panels.active = panel;
                self.panels.visible = true;
            }

            AppCommand::SetAssistantInput { input } => {
                self.assistant.input = input;
            }

            AppCommand::SendAssistantPrompt => {
                // placeholder: push input as a message and clear the input
                let prompt = self.assistant.input.clone();
                if !prompt.is_empty() {
                    self.assistant.messages.push(format!("User: {}", prompt));
                    self.assistant.input.clear();
                    // push a dummy assistant response
                    self.assistant.messages.push("Assistant: (placeholder response)".to_string());
                    self.status.message = "Assistant responded (placeholder)".to_string();
                }
            }

            AppCommand::InsertAssistantSuggestion { doc_id, text } => {
                // insert suggestion into the document via editor core
                self.editor.apply(zaroxi_editor_core::EditorCommand::InsertText {
                    doc_id,
                    offset: 0,
                    text,
                });
                self.status.message = "Inserted assistant suggestion".to_string();
            }

            AppCommand::SetStatusMessage { message } => {
                self.status.message = message;

                // keep status panel in sync
                if let Some(p) = self.app_panels.iter_mut().find(|p| p.id == "status_bar") {
                    p.content = message.clone();
                }
            }

            AppCommand::SetThemeMode { mode } => {
                self.theme_mode = mode;
                self.status.message = format!("Theme set to {:?}", mode);
            }
        }
    }
}
