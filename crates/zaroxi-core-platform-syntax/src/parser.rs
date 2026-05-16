//! Incremental parser management for Tree-sitter.
//!
//! This module provides:
//! - `SyntaxTree`: A parsed syntax tree with its associated text and language
//! - `ParserPool`: A thread-safe pool of Tree-sitter parsers for reuse
//! - Incremental parsing support for efficient re-parsing after edits

use tree_sitter::{Parser, Tree, InputEdit};
use ropey::Rope;
use std::sync::Arc;
use parking_lot::Mutex;

use crate::language::LanguageId;
use crate::SyntaxError;

/// A thread-safe pool of Tree-sitter parsers.
///
/// Parsers are expensive to create, so we reuse them across highlight operations.
/// Each parser is associated with a specific language and can be reused for
/// incremental parsing.
pub struct ParserPool {
    /// Map of language ID to a pool of parsers for that language.
    parsers: Mutex<std::collections::HashMap<LanguageId, Vec<Parser>>>,
}

impl std::fmt::Debug for ParserPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num_parsers = self.parsers.lock().len();
        f.debug_struct("ParserPool")
            .field("num_parsers", &num_parsers)
            .finish()
    }
}

impl ParserPool {
    /// Create a new empty parser pool.
    pub fn new() -> Self {
        Self {
            parsers: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Get a parser for the given language.
    ///
    /// If a parser is available in the pool, it is returned. Otherwise, a new
    /// parser is created and configured with the language's grammar.
    pub fn acquire(&self, language: &LanguageId) -> Option<Parser> {
        // PlainText has no grammar – return None immediately
        if *language == LanguageId::PlainText {
            eprintln!("DEBUG: ParserPool::acquire: PlainText, returning None");
            return None;
        }

        let mut pool = self.parsers.lock();

        // Try to reuse an existing parser
        if let Some(parsers) = pool.get_mut(language) {
            if let Some(parser) = parsers.pop() {
                eprintln!("DEBUG: ParserPool::acquire: reusing parser for {:?}", language);
                return Some(parser);
            }
        }

        eprintln!("DEBUG: ParserPool::acquire: creating new parser for {:?}", language);

        // Create a new parser
        let mut parser = Parser::new();

        // Set the language for the parser
        let ts_lang = match language.tree_sitter_language() {
            Some(lang) => {
                eprintln!("DEBUG: ParserPool::acquire: got tree_sitter_language for {:?}", language);
                lang
            }
            None => {
                eprintln!("DEBUG: ParserPool::acquire: tree_sitter_language returned None for {:?}", language);
                return None;
            }
        };

        match parser.set_language(&ts_lang) {
            Ok(()) => {
                eprintln!("DEBUG: ParserPool::acquire: set_language succeeded for {:?}", language);
                Some(parser)
            }
            Err(e) => {
                eprintln!("DEBUG: ParserPool::acquire: set_language failed for {:?}: {}", language, e);
                None
            }
        }
    }

    /// Return a parser to the pool for reuse.
    pub fn release(&self, language: &LanguageId, parser: Parser) {
        let mut pool = self.parsers.lock();
        pool.entry(*language).or_insert_with(Vec::new).push(parser);
    }
}

impl Default for ParserPool {
    fn default() -> Self {
        Self::new()
    }
}

/// A syntax tree with its associated text and language.
///
/// This struct manages the lifecycle of a Tree-sitter parse tree and supports
/// incremental re-parsing after text edits.
#[derive(Clone)]
pub struct SyntaxTree {
    /// The Tree-sitter parse tree.
    tree: Tree,
    /// The text content as a rope for efficient editing.
    text: Rope,
    /// Language of this tree.
    language: LanguageId,
    /// Parser pool for acquiring parsers.
    pool: Arc<ParserPool>,
}

impl std::fmt::Debug for SyntaxTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntaxTree")
            .field("text", &self.text)
            .field("language", &self.language)
            .field("pool", &self.pool)
            .finish()
    }
}

impl SyntaxTree {
    /// Create a new syntax tree by parsing text.
    ///
    /// Acquires a parser from the pool, parses the text, and returns the tree.
    /// The parser is returned to the pool after parsing.
    pub fn new(
        pool: Arc<ParserPool>,
        text: &str,
        language: LanguageId,
    ) -> Result<Self, SyntaxError> {
        eprintln!("DEBUG: SyntaxTree::new: language={:?}, text length={}", language, text.len());

        // PlainText has no grammar – return error immediately
        if language == LanguageId::PlainText {
            eprintln!("DEBUG: SyntaxTree::new: PlainText, returning error");
            return Err(SyntaxError::GrammarLoadError(
                "PlainText has no grammar".to_string(),
            ));
        }

        let mut parser = pool
            .acquire(&language)
            .ok_or_else(|| {
                eprintln!("DEBUG: SyntaxTree::new: failed to acquire parser for {:?}", language);
                SyntaxError::GrammarLoadError(format!(
                    "Failed to acquire parser for language '{}'",
                    language.as_str()
                ))
            })?;

        eprintln!("DEBUG: SyntaxTree::new: parser acquired, parsing...");

        let tree = parser
            .parse(text, None)
            .ok_or_else(|| {
                eprintln!("DEBUG: SyntaxTree::new: parse returned None");
                SyntaxError::ParseError
            })?;

        eprintln!("DEBUG: SyntaxTree::new: parse succeeded, root node count: {}", tree.root_node().child_count());

        // Return the parser to the pool
        pool.release(&language, parser);

        Ok(Self {
            tree,
            text: Rope::from_str(text),
            language,
            pool,
        })
    }

    /// Update the syntax tree with an edit.
    ///
    /// This applies the edit to the existing tree, enabling incremental
    /// re-parsing. The text rope is NOT updated here; it should be updated
    /// separately in the document model.
    pub fn edit(
        &mut self,
        start_byte: usize,
        old_end_byte: usize,
        new_end_byte: usize,
        start_position: tree_sitter::Point,
        old_end_position: tree_sitter::Point,
        new_end_position: tree_sitter::Point,
    ) {
        let edit = InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position,
            old_end_position,
            new_end_position,
        };

        self.tree.edit(&edit);
    }

    /// Reparse the tree incrementally after edits.
    ///
    /// Uses the old tree as a starting point for incremental parsing,
    /// which is significantly faster than re-parsing from scratch.
    pub fn reparse(&mut self) -> Result<(), SyntaxError> {
        let mut parser = self
            .pool
            .acquire(&self.language)
            .ok_or_else(|| SyntaxError::GrammarLoadError(format!(
                "Failed to acquire parser for language '{}'",
                self.language.as_str()
            )))?;

        let text_str = self.text.to_string();

        // Parse with the old tree for incremental parsing
        let new_tree = parser
            .parse(&text_str, Some(&self.tree))
            .ok_or_else(|| SyntaxError::ParseError)?;

        // Return the parser to the pool
        self.pool.release(&self.language, parser);

        // Replace the old tree with the new one
        self.tree = new_tree;

        Ok(())
    }

    /// Get the text as a string.
    pub fn text(&self) -> String {
        self.text.to_string()
    }

    /// Get the underlying Tree-sitter tree.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Get the language.
    pub fn language(&self) -> LanguageId {
        self.language
    }

    /// Get a mutable reference to the underlying Tree-sitter tree.
    pub fn tree_mut(&mut self) -> &mut Tree {
        &mut self.tree
    }
}
