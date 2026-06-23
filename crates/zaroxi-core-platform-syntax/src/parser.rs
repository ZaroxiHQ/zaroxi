//! Incremental parser management for Tree-sitter.
//!
//! This module provides:
//! - `SyntaxTree`: A parsed syntax tree with its associated text and language
//! - `ParserPool`: A thread-safe pool of Tree-sitter parsers for reuse
//! - Incremental parsing support for efficient re-parsing after edits

use parking_lot::Mutex;
use ropey::Rope;
use std::sync::Arc;
use tree_sitter::{InputEdit, Parser, Tree};

use crate::SyntaxError;
use crate::language::LanguageId;

/// A thread-safe pool of Tree-sitter parsers.
///
/// Parsers are expensive to create, so we reuse them across highlight operations.
/// Each parser is associated with a specific language and can be reused for
/// incremental parsing.
pub struct ParserPool {
    parsers: Mutex<std::collections::HashMap<LanguageId, Vec<Parser>>>,
    unavailable: Mutex<std::collections::HashSet<LanguageId>>,
}

impl std::fmt::Debug for ParserPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num_parsers = self.parsers.lock().len();
        let num_unavailable = self.unavailable.lock().len();
        f.debug_struct("ParserPool")
            .field("num_parsers", &num_parsers)
            .field("num_unavailable", &num_unavailable)
            .finish()
    }
}

impl ParserPool {
    /// Create a new empty parser pool.
    pub fn new() -> Self {
        Self {
            parsers: Mutex::new(std::collections::HashMap::new()),
            unavailable: Mutex::new(std::collections::HashSet::new()),
        }
    }

    /// Get a parser for the given language.
    ///
    /// If a parser is available in the pool, it is returned. Otherwise, a new
    /// parser is created and configured with the language's grammar.
    pub fn acquire(&self, language: &LanguageId) -> Option<Parser> {
        if *language == LanguageId::PlainText {
            return None;
        }

        if self.unavailable.lock().contains(language) {
            return None;
        }

        let mut pool = self.parsers.lock();

        if let Some(parsers) = pool.get_mut(language) {
            if let Some(parser) = parsers.pop() {
                return Some(parser);
            }
        }

        let mut parser = Parser::new();

        let ts_lang = match language.tree_sitter_language() {
            Some(lang) => lang,
            None => {
                self.unavailable.lock().insert(*language);
                return None;
            }
        };

        match parser.set_language(&ts_lang) {
            Ok(()) => Some(parser),
            Err(_e) => {
                self.unavailable.lock().insert(*language);
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
        if language == LanguageId::PlainText {
            return Err(SyntaxError::GrammarLoadError("PlainText has no grammar".to_string()));
        }

        let mut parser = pool.acquire(&language).ok_or_else(|| {
            SyntaxError::GrammarLoadError(format!(
                "Failed to acquire parser for language '{}'",
                language.as_str()
            ))
        })?;

        let tree = parser.parse(text, None).ok_or_else(|| SyntaxError::ParseError)?;

        pool.release(&language, parser);

        Ok(Self { tree, text: Rope::from_str(text), language, pool })
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

    /// Apply an edit to BOTH the text rope and the parse tree, keeping them in
    /// sync so a subsequent [`Self::reparse`] is genuinely incremental.
    ///
    /// `start_byte..old_end_byte` is the replaced region (in the *pre-edit*
    /// text) and `new_text` is the inserted replacement. Rope edits are
    /// O(log n); the tree-sitter [`InputEdit`] confines the next reparse to the
    /// dirty span. This replaces the previous "full reparse from scratch on
    /// every keystroke" cost on large files.
    pub fn apply_edit(&mut self, start_byte: usize, old_end_byte: usize, new_text: &str) {
        let start_pos = byte_to_point(&self.text, start_byte);
        let old_end_pos = byte_to_point(&self.text, old_end_byte);

        let start_char = self.text.byte_to_char(start_byte);
        let old_end_char = self.text.byte_to_char(old_end_byte);
        if start_char < old_end_char {
            self.text.remove(start_char..old_end_char);
        }
        self.text.insert(start_char, new_text);

        let new_end_byte = start_byte + new_text.len();
        let new_end_pos = byte_to_point(&self.text, new_end_byte);

        self.edit(start_byte, old_end_byte, new_end_byte, start_pos, old_end_pos, new_end_pos);
    }

    /// Reparse the tree incrementally after edits.
    ///
    /// Feeds bytes to Tree-sitter directly from the rope in chunks via
    /// [`tree_sitter::Parser::parse_with_options`] — it never materializes the
    /// whole document into a `String` (the previous `self.text.to_string()` was
    /// O(n) per keystroke and dominated edit latency on 300k+ line files).
    /// Combined with [`Self::apply_edit`], Tree-sitter re-lexes only the dirty
    /// region using the previously edited tree as a starting point.
    pub fn reparse(&mut self) -> Result<(), SyntaxError> {
        let mut parser = self.pool.acquire(&self.language).ok_or_else(|| {
            SyntaxError::GrammarLoadError(format!(
                "Failed to acquire parser for language '{}'",
                self.language.as_str()
            ))
        })?;

        // Borrow the rope + old tree as disjoint fields; both immutable borrows
        // end with the parse call so `self.tree = new_tree` below is allowed.
        let rope = &self.text;
        let total_bytes = rope.len_bytes();
        let old_tree = &self.tree;
        let new_tree = parser
            .parse_with_options(
                &mut |byte: usize, _pos: tree_sitter::Point| -> &[u8] {
                    if byte >= total_bytes {
                        return &[];
                    }
                    // `chunk_at_byte` returns the chunk containing `byte` plus
                    // its starting byte offset, so we can hand Tree-sitter a
                    // zero-copy slice from the middle of a chunk.
                    let (chunk, chunk_start, _, _) = rope.chunk_at_byte(byte);
                    &chunk.as_bytes()[byte - chunk_start..]
                },
                Some(old_tree),
                None,
            )
            .ok_or(SyntaxError::ParseError)?;

        self.pool.release(&self.language, parser);
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

/// Map a byte offset in `rope` to a Tree-sitter [`tree_sitter::Point`]
/// (`row` = 0-based line, `column` = byte offset within that line). Offsets past
/// the end clamp to the document end so callers never panic on stale indices.
fn byte_to_point(rope: &Rope, byte: usize) -> tree_sitter::Point {
    let byte = byte.min(rope.len_bytes());
    let line = rope.byte_to_line(byte);
    let line_start = rope.line_to_byte(line);
    tree_sitter::Point { row: line, column: byte - line_start }
}

#[cfg(test)]
mod tests {
    use super::byte_to_point;
    use ropey::Rope;

    #[test]
    fn byte_to_point_maps_rows_and_columns() {
        // "abc\n" = 4 bytes (rows: 0:abc, 1:defg, 2:hi)
        let rope = Rope::from_str("abc\ndefg\nhi");

        let p = byte_to_point(&rope, 0);
        assert_eq!((p.row, p.column), (0, 0), "start of document");

        let p = byte_to_point(&rope, 2);
        assert_eq!((p.row, p.column), (0, 2), "mid first line");

        let p = byte_to_point(&rope, 4);
        assert_eq!((p.row, p.column), (1, 0), "start of second line");

        let p = byte_to_point(&rope, 6);
        assert_eq!((p.row, p.column), (1, 2), "mid second line");

        // Past the end clamps to the document end (last line).
        let p = byte_to_point(&rope, 9_999);
        assert_eq!(p.row, 2, "clamped to last line");
    }
}
