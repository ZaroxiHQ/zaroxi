use crate::commands::AppCommand;
use crate::status::StatusState;
use crate::assistant::AssistantState;
use crate::tabs::TabState;
use crate::panels::PanelEntry;
use zaroxi_config::AppConfig;
use zaroxi_editor_core::EditorState;
use zaroxi_editor_buffer::Document;
use zaroxi_workspace::{WorkspaceState, WorkspaceItem};
use zaroxi_theme::ZaroxiTheme;

/// Top-level app state assembled from domain parts.
///
/// This struct was extracted into its own module to keep the root `app.rs`
/// file small and focused on orchestration.
#[derive(Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub workspace: WorkspaceState,
    pub editor: EditorState,
    pub status: StatusState,
    pub assistant: AssistantState,
    pub tabs: TabState,
    /// User-selected theme mode (Light/Dark/System).
    pub theme_mode: ZaroxiTheme,
    /// Application-owned panel descriptors (title/content/visibility).
    pub app_panels: Vec<PanelEntry>,
}

impl AppState {
    /// Create a new AppState using provided config and sensible defaults.
    pub fn new(config: &AppConfig) -> Self {
        // workspace: start empty (workspace-first UX). User must open a folder to populate.
        let mut workspace = WorkspaceState::new();
        // no initial items; selection cleared
        workspace.select(None);

        // editor: workspace-first — start with no active document.
        let editor = EditorState::new();
        // create an empty document only to satisfy panel construction; do not open it.
        let empty_doc = Document::new("".to_string(), "".to_string());

        // tabs: empty initially
        let tabs = TabState::new();

        let status = StatusState::default();
        let assistant = AssistantState::default();

        // Build app-owned panels via the panels builder module.
        // Pass an empty document placeholder; panels will be updated when a workspace is loaded.
        let mut app_panels = crate::panels::default_panels(config, &empty_doc);

        // Ensure editor panel body starts empty (no placeholder welcome).
        if let Some(p) = app_panels.iter_mut().find(|p| p.id == "editor") {
            p.content = "".to_string();
        }

        // Log created panels for visibility
        for p in &app_panels {
            log::debug!("created panel: {} ({}) visible={}", p.id, p.title, p.visible);
        }

        Self {
            config: config.clone(),
            workspace,
            editor,
            status,
            assistant,
            tabs,
            theme_mode: ZaroxiTheme::Dark,
            app_panels,
        }
    }

    /// Apply an application-level command. This mutates the app state in a
    /// single place, keeping the command pipeline explicit and easy to audit.
    pub fn apply(&mut self, cmd: AppCommand) {
        match cmd {
            AppCommand::OpenFolder { path } => {
                use std::fs;
                use std::path::PathBuf;

                let root = PathBuf::from(&path);
                if !root.is_dir() {
                    self.status.message = format!("OpenFolder failed: not a directory: {}", path);
                } else {
                    // set workspace root and populate items by walking files under the root
                    self.workspace.root = Some(root.clone());
                    self.workspace.items.clear();

                    fn visit_dir(dir: &std::path::Path, base: &std::path::Path, items: &mut Vec<WorkspaceItem>) {
                        if let Ok(entries) = fs::read_dir(dir) {
                            for entry in entries.flatten() {
                                let p = entry.path();
                                if p.is_dir() {
                                    visit_dir(&p, base, items);
                                } else if p.is_file() {
                                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                        // store a relative display name and absolute path string
                                        let rel = p.strip_prefix(base).ok().and_then(|r| r.to_str()).unwrap_or(name).to_string();
                                        items.push(WorkspaceItem::file(rel, Some(p.to_string_lossy().to_string())));
                                    }
                                }
                            }
                        }
                    }

                    visit_dir(&root, &root, &mut self.workspace.items);

                    self.status.message = format!("Opened folder {}", path);

                    // Update Explorer/Sidebar panel content to show file listing for immediate visual feedback.
                    if let Some(panel) = self.app_panels.iter_mut().find(|p| p.id == "sidebar" || p.id == "explorer") {
                        let listing = self.workspace.items.iter().enumerate()
                            .map(|(i,it)| format!("{}: {}", i, it.name))
                            .collect::<Vec<_>>()
                            .join("\n");
                        panel.content = listing;
                    }
                }
            }

            AppCommand::OpenFile { path } => {
                // Read the file from disk and open it as a real document.
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let doc = Document::new(path.clone(), content);
                        self.editor.open_document(doc.clone());
                        self.tabs.open_tab_for_document(&doc);
                        self.status.message = format!("Opened {}", path);

                        // Reflect active editor text in the editor panel content if present
                        if let Some(panel) = self.app_panels.iter_mut().find(|p| p.id == "editor") {
                            panel.content = doc.text.clone();
                        }
                    }
                    Err(e) => {
                        self.status.message = format!("Failed to open {}: {}", path, e);
                    }
                }
            }

            AppCommand::SelectSidebarItem { index } => {
                self.workspace.select(index);
                if let Some(i) = index {
                    if let Some(item) = self.workspace.items.get(i) {
                        if let Some(path) = &item.path {
                            // Open the selected file from disk and show real contents in the editor.
                            match std::fs::read_to_string(&path) {
                                Ok(content) => {
                                    let display_name = std::path::Path::new(path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or(path)
                                        .to_string();
                                    let doc = Document::new(display_name, content);
                                    self.editor.open_document(doc.clone());
                                    self.tabs.open_tab_for_document(&doc);
                                    self.status.message = format!("Selected {}", path);

                                    // update editor panel content
                                    if let Some(panel) = self.app_panels.iter_mut().find(|p| p.id == "editor") {
                                        panel.content = doc.text.clone();
                                    }
                                }
                                Err(e) => {
                                    self.status.message = format!("Failed to open {}: {}", path, e);
                                }
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
                // Toggle visibility of the bottom panel via the app-owned panel entries.
                if let Some(p) = self.app_panels.iter_mut().find(|p| p.id == "bottom_panel") {
                    p.visible = !p.visible;
                }
            }

            AppCommand::SetBottomPanel { panel } => {
                // Ensure the requested panel id is visible.
                let id = panel.as_str();
                if let Some(p) = self.app_panels.iter_mut().find(|p| p.id == id) {
                    p.visible = true;
                }
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
                // Assign into status first, then derive panel content from it to avoid
                // moved-value usage of `message`.
                self.status.message = message;
                if let Some(p) = self.app_panels.iter_mut().find(|p| p.id == "status_bar") {
                    p.content = self.status.message.clone();
                }
            }

            AppCommand::SetThemeMode { mode } => {
                self.theme_mode = mode;
                self.status.message = format!("Theme set to {:?}", mode);
            }
        }
    }
}
