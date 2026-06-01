//! Syntax manager for coordinating multiple documents and languages.

use crate::error::SyntaxError;
use crate::highlight::{HighlightEngine, HighlightSpan};
use crate::language::LanguageId;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Parser, Tree};

/// Manager that coordinates documents, parsing, and highlighting.
///
/// `SyntaxManager` holds document text, parser instances and the highlight
/// engine. Consumers use this type to update documents and obtain highlight
/// spans in a thread-local manager instance.
pub struct SyntaxManager {
    documents: HashMap<String, SyntaxDocument>,
    // Cache parsers per language to avoid recreating them
    parsers: HashMap<LanguageId, Parser>,
    /// Whether large file mode is active (disables syntax features)
    large_file_mode: bool,
    /// The highlighting engine for computing highlight spans.
    highlight_engine: HighlightEngine,
}

/// Internal per-document representation.
///
/// Kept private to avoid exposing internal buffer/text details to callers.
struct SyntaxDocument {
    text: String,
    language: LanguageId,
    tree: Option<Tree>,
}

impl SyntaxManager {
    /// Create a new SyntaxManager with empty state.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            parsers: HashMap::new(),
            large_file_mode: false,
            highlight_engine: HighlightEngine::new(),
        }
    }

    /// Set large-file mode on or off.
    ///
    /// When enabled parsing/highlighting is disabled and existing syntax trees
    /// are dropped to conserve memory for very large files.
    pub fn set_large_file_mode(&mut self, enabled: bool) {
        self.large_file_mode = enabled;
        if enabled {
            // Clear all trees to free memory
            for doc in self.documents.values_mut() {
                doc.tree = None;
            }
        }
    }

    /// Return true when large-file mode is active.
    pub fn is_large_file_mode(&self) -> bool {
        self.large_file_mode
    }

    /// Insert or replace a document's contents and (re)parse it.
    ///
    /// If the manager is in large-file mode or no grammar is available for the
    /// detected language, the document is stored without a parse tree.
    pub fn update_document(
        &mut self,
        doc_id: &str,
        text: &str,
        path: &Path,
    ) -> Result<(), SyntaxError> {
        let language = LanguageId::from_path(path);

        // If in large file mode, store document without a tree
        if self.large_file_mode {
            let doc = SyntaxDocument { text: text.to_string(), language, tree: None };
            self.documents.insert(doc_id.to_string(), doc);
            return Ok(());
        }

        // Try to get the language
        let ts_lang = match language.tree_sitter_language() {
            Some(lang) => lang,
            None => {
                // If no language is available, store document without a tree
                let doc = SyntaxDocument { text: text.to_string(), language, tree: None };
                self.documents.insert(doc_id.to_string(), doc);
                return Ok(());
            }
        };

        // Try to get or create a parser for this language
        let parser = self.parsers.entry(language).or_insert_with(|| {
            let mut parser = Parser::new();
            // Try to set the language, but don't panic if it fails
            let _ = parser.set_language(&ts_lang);
            parser
        });

        // Parse the document
        let tree = parser.parse(text, None);

        let doc = SyntaxDocument { text: text.to_string(), language, tree };
        self.documents.insert(doc_id.to_string(), doc);
        Ok(())
    }

    /// Apply an edit to an existing document.
    ///
    /// The edit is applied to the stored text and the parse tree is cleared;
    /// the tree will be re-parsed on the next highlight request.
    pub fn edit_document(
        &mut self,
        doc_id: &str,
        start_byte: usize,
        old_end_byte: usize,
        new_text: &str,
    ) -> Result<(), SyntaxError> {
        // Find the document
        if let Some(doc) = self.documents.get_mut(doc_id) {
            // Apply the edit to the text
            let mut text = doc.text.clone();
            if start_byte <= old_end_byte && old_end_byte <= text.len() {
                text.replace_range(start_byte..old_end_byte, new_text);
                doc.text = text;

                // Re-parse the document only if not in large file mode
                if !self.large_file_mode {
                    // For now, we'll clear the tree and it will be re-parsed on next highlight
                    doc.tree = None;
                }
            }
        }
        Ok(())
    }

    /// Return true if a document with the given id is managed.
    pub fn contains_document(&self, doc_id: &str) -> bool {
        self.documents.contains_key(doc_id)
    }

    /// Compute highlight spans for a document.
    ///
    /// Returns an empty vector when no parse tree is available or when in
    /// large-file mode.
    pub fn highlight_spans(&self, doc_id: &str) -> Result<Vec<HighlightSpan>, SyntaxError> {
        // If in large file mode, return empty highlights
        if self.large_file_mode {
            eprintln!("DEBUG: highlight_spans: large file mode, returning empty");
            return Ok(Vec::new());
        }

        let doc = self.documents.get(doc_id).ok_or_else(|| {
            eprintln!("DEBUG: highlight_spans: document '{}' not found", doc_id);
            SyntaxError::DocumentNotFound
        })?;
        match &doc.tree {
            Some(tree) => {
                eprintln!(
                    "DEBUG: highlight_spans: tree exists for doc '{}', language {:?}",
                    doc_id, doc.language
                );
                self.highlight_engine.highlight(doc.language, &doc.text, tree)
            }
            None => {
                eprintln!("DEBUG: highlight_spans: tree is None for doc '{}'", doc_id);
                Ok(Vec::new())
            }
        }
    }

    /// Initialize dynamic grammars and preload queries
    pub fn initialize_dynamic_grammars(&mut self) {
        use crate::dynamic_loader::preload_available_grammars;
        use crate::query_cache::preload_queries;

        // Preload available grammars
        preload_available_grammars();

        // Preload queries
        preload_queries();
    }
}

impl Default for SyntaxManager {
    fn default() -> Self {
        Self::new()
    }
}
