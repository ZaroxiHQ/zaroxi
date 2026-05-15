use serde::{Deserialize, Serialize};
use zaroxi_foundation::DocumentId;
use zaroxi_editor_buffer::Document;

/// Tab representing an open document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub doc_id: DocumentId,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    pub tabs: Vec<Tab>,
    /// Index into `tabs` that is active
    pub active: Option<usize>,
}

impl TabState {
    pub fn new() -> Self {
        Self { tabs: Vec::new(), active: None }
    }

    pub fn open_tab_for_document(&mut self, doc: &Document) {
        // avoid duplicate tabs for the same document
        if self.tabs.iter().any(|t| t.doc_id == doc.id) {
            self.activate_by_doc_id(doc.id);
            return;
        }
        let idx = self.tabs.len();
        self.tabs.push(Tab {
            doc_id: doc.id,
            title: doc.display_name.clone(),
        });
        self.active = Some(idx);
    }

    pub fn activate_by_doc_id(&mut self, doc_id: DocumentId) {
        if let Some(idx) = self.tabs.iter().position(|t| t.doc_id == doc_id) {
            self.active = Some(idx);
        }
    }

    pub fn close_by_doc_id(&mut self, doc_id: DocumentId) {
        if let Some(pos) = self.tabs.iter().position(|t| t.doc_id == doc_id) {
            self.tabs.remove(pos);
            // adjust active index
            self.active = if self.tabs.is_empty() {
                None
            } else {
                Some((pos.saturating_sub(1)).min(self.tabs.len().saturating_sub(1)))
            };
        }
    }

    pub fn active_doc_id(&self) -> Option<DocumentId> {
        self.active.and_then(|i| self.tabs.get(i)).map(|t| t.doc_id)
    }
}
