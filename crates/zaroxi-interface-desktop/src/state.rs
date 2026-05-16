use crate::commands::EditorCommand;
use serde::{Deserialize, Serialize};
use zaroxi_domain_buffer::Document;
use zaroxi_kernel_types::Id;

/// In-memory editor state.
///
/// Stores a small set of open documents and an optional active document id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorState {
    /// Open documents.
    pub open_documents: Vec<Document>,
    /// Active document id.
    pub active_document: Option<Id>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            open_documents: Vec::new(),
            active_document: None,
        }
    }

    /// Open a document in the editor (adds to open_documents and activates it).
    pub fn open_document(&mut self, doc: Document) {
        self.active_document = Some(doc.id);
        self.open_documents.push(doc);
    }

    /// Get a reference to the active document if any.
    pub fn active_document(&self) -> Option<&Document> {
        match self.active_document {
            Some(id) => self.open_documents.iter().find(|d| d.id == id),
            None => None,
        }
    }

    /// Apply a simple command. Keep the command surface small and explicit.
    pub fn apply(&mut self, cmd: EditorCommand) {
        match cmd {
            EditorCommand::InsertText { doc_id, offset, text } => {
                if let Some(doc) = self.open_documents.iter_mut().find(|d| d.id == doc_id) {
                    // naive insert
                    let mut content = doc.text.clone();
                    let byte_pos = content.char_indices().nth(offset).map(|(b, _)| b).unwrap_or(content.len());
                    content.insert_str(byte_pos, &text);
                    doc.text = content;
                    doc.dirty = true;
                }
            }
        }
    }
}
