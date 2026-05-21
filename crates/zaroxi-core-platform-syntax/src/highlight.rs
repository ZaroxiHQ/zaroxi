//! Syntax highlighting using Tree-sitter queries.

use crate::error::SyntaxError;
use crate::language::LanguageId;
use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

/// A highlight span in the document.
///
/// Represents a contiguous byte range in the source with its highlight
/// classification. Consumers use these spans to drive renderer tokenization
/// or presenter-level styling.
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    /// Start byte offset
    pub start: usize,
    /// End byte offset
    pub end: usize,
    /// Highlight type (maps to zaroxi_theme token categories)
    pub highlight: Highlight,
}

/// Highlight types (maps to Tree-sitter capture names).
///
/// These categories are intentionally small and map from Tree-sitter capture
/// names to a limited set of highlighting roles used by the presenter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Highlight {
    /// Comment text (line/block/documentation).
    Comment,
    /// String and literal text.
    String,
    /// Language keywords and control tokens.
    Keyword,
    /// Function and call sites.
    Function,
    /// Variables and identifiers.
    Variable,
    /// Type names and annotations.
    Type,
    /// Constants and literal constants.
    Constant,
    /// Attributes, annotations, or decorators.
    Attribute,
    /// Operators and punctuation-like operators.
    Operator,
    /// Numeric literals.
    Number,
    /// Property/field-like accessors.
    Property,
    /// Namespace/module identifiers and imports.
    Namespace,
    /// Plain text / no special highlighting.
    Plain,
}

/// The highlighting engine that manages parsing and query execution.
///
/// This struct is designed to be reused across multiple highlight operations
/// to avoid re-creating parsers and queries unnecessarily.
#[derive(Debug, Clone)]
pub struct HighlightEngine {
    /// Cached runtime for resolving language resources.
    runtime: crate::runtime::Runtime,
}

impl HighlightEngine {
    /// Create a new highlighting engine.
    pub fn new() -> Self {
        Self {
            runtime: crate::runtime::Runtime::new(),
        }
    }

    /// Highlight a syntax tree for a given language.
    ///
    /// Returns a vector of `HighlightSpan`s sorted by start position.
    /// Returns an empty vector if highlighting is not possible (e.g., unknown
    /// language, missing query, or large file).
    pub fn highlight(
        &self,
        language: LanguageId,
        source: &str,
        tree: &Tree,
    ) -> Result<Vec<HighlightSpan>, SyntaxError> {
        // For plaintext or unknown languages, return empty spans
        if language == LanguageId::PlainText {
            eprintln!("DEBUG: PlainText language, returning empty highlights");
            return Ok(Vec::new());
        }

        eprintln!("DEBUG: Highlighting for language: {:?}", language);

        // Get the Tree-sitter language
        let _ts_lang = match language.tree_sitter_language() {
            Some(lang) => {
                eprintln!("DEBUG: Got Tree-sitter language for {:?}", language);
                lang
            }
            None => {
                eprintln!("DEBUG: No Tree-sitter language available for {:?}", language);
                // No grammar available, return empty spans
                return Ok(Vec::new());
            }
        };

        // Load and compile the highlighting query
        let query = match self.load_query(&language) {
            Some(q) => q,
            None => {
                eprintln!("DEBUG: No query available for {:?}", language);
                // No query available, return empty spans
                return Ok(Vec::new());
            }
        };

        // Execute the query against the syntax tree
        let spans = self.execute_query(&query, source, tree);
        eprintln!("DEBUG: Got {} raw highlight spans", spans.len());

        // Post-process spans: sort and filter covered plain spans
        let filtered = self.filter_spans(spans);
        eprintln!("DEBUG: After filtering: {} highlight spans", filtered.len());

        Ok(filtered)
    }

    /// Load and compile the highlighting query for a language.
    ///
    /// Returns `None` if the query cannot be loaded or compiled.
    fn load_query(&self, language: &LanguageId) -> Option<Query> {
        let language_id = language.as_str();

        // Get the Tree-sitter language for query compilation
        let ts_lang = language.tree_sitter_language()?;

        // Read the query file from the runtime directory
        let query_path = self
            .runtime
            .language_dir(language_id)
            .join("queries")
            .join("highlights.scm");

        let query_text = match std::fs::read_to_string(&query_path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("DEBUG: Query file not found for {}: {} (path: {})", language_id, e, query_path.display());
                // Try alternative path: look for queries directly in language directory
                let alt_path = self.runtime.language_dir(language_id).join("highlights.scm");
                match std::fs::read_to_string(&alt_path) {
                    Ok(text) => text,
                    Err(e2) => {
                        eprintln!("DEBUG: Alternative path also failed: {}", e2);
                        return None;
                    }
                }
            }
        };

        if query_text.trim().is_empty() {
            eprintln!("DEBUG: Query file for {} is empty", language_id);
            return None;
        }

        // Compile the query
        match Query::new(&ts_lang, &query_text) {
            Ok(q) => {
                eprintln!("DEBUG: Successfully compiled query for {}", language_id);
                Some(q)
            }
            Err(e) => {
                // Log the error for debugging but don't crash
                eprintln!(
                    "Warning: Failed to compile query for {}: {}",
                    language_id, e
                );
                None
            }
        }
    }

    /// Execute a highlighting query against a syntax tree.
    fn execute_query(
        &self,
        query: &Query,
        source: &str,
        tree: &Tree,
    ) -> Vec<HighlightSpan> {
        let mut cursor = QueryCursor::new();
        let root_node = tree.root_node();
        let mut spans = Vec::new();

        eprintln!("DEBUG: Executing query, source length: {} bytes", source.len());
        eprintln!("DEBUG: Root node: start={}, end={}", root_node.start_byte(), root_node.end_byte());

        // Iterate over query matches using QueryCursor::matches which returns a streaming
        // iterator. Use the StreamingIterator trait's `next` helper to walk matches.
        let mut matches = cursor.matches(query, root_node, source.as_bytes());
        let mut match_count = 0;
        while let Some(m) = StreamingIterator::next(&mut matches) {
            match_count += 1;
            for capture in m.captures.iter() {
                let node = capture.node;
                let start = node.start_byte();
                let end = node.end_byte();
                let capture_name = &query.capture_names()[capture.index as usize];
                let highlight = map_capture_name(capture_name);

                spans.push(HighlightSpan { start, end, highlight });
            }
        }

        eprintln!("DEBUG: Found {} query matches, {} total captures", match_count, spans.len());
        spans
    }

    /// Post-process highlight spans: sort and filter covered plain spans.
    fn filter_spans(&self, mut spans: Vec<HighlightSpan>) -> Vec<HighlightSpan> {
        // Sort by start position
        spans.sort_by_key(|span| span.start);

        // Filter out plain spans that are completely covered by other spans
        let mut filtered = Vec::new();
        for (i, span) in spans.iter().enumerate() {
            if span.highlight == Highlight::Plain {
                let mut covered = false;
                for (j, other) in spans.iter().enumerate() {
                    if i != j
                        && other.highlight != Highlight::Plain
                        && other.start <= span.start
                        && other.end >= span.end
                    {
                        covered = true;
                        break;
                    }
                }
                if !covered {
                    filtered.push(span.clone());
                }
            } else {
                filtered.push(span.clone());
            }
        }

        filtered
    }
}

impl Default for HighlightEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a Tree-sitter capture name to a Highlight classification.
///
/// This provides a stable mapping from varied capture names used by different
/// grammars into the small set of Highlight categories used by the presenter.
pub fn map_capture_name(name: &str) -> Highlight {
    match name {
        // Comments
        "comment" | "comment.line" | "comment.block" | "comment.documentation" => Highlight::Comment,
        
        // Strings
        "string" | "string.quoted" | "string.quoted.single" | "string.quoted.double" 
        | "string.quoted.triple" | "string.quoted.raw" | "string.quoted.other" 
        | "string.special" | "string.special.path" | "string.special.url" 
        | "string.special.symbol" | "string.special.regex" | "string.special.format" 
        | "string.interpolation" | "string.template" => Highlight::String,
        
        // Escape sequences
        "escape" | "string.escape" | "character.escape" | "escape_sequence" => Highlight::String,
        
        // Keywords
        "keyword" | "keyword.control" | "keyword.control.conditional" 
        | "keyword.control.repeat" | "keyword.control.import" 
        | "keyword.control.exception" | "keyword.control.flow" | "keyword.operator" 
        | "keyword.directive" | "keyword.directive.define" | "keyword.directive.include" 
        | "keyword.storage" | "keyword.storage.modifier" | "keyword.storage.type" 
        | "keyword.function" | "keyword.other" | "keyword.other.unit" 
        | "keyword.other.special-method" | "keyword.other.import" 
        | "keyword.control.as" | "keyword.control.use" | "keyword.control.mod" 
        | "keyword.control.where" | "keyword.control.let" | "keyword.control.match" 
        | "keyword.control.if" | "keyword.control.else" | "keyword.control.for" 
        | "keyword.control.while" | "keyword.control.loop" | "keyword.control.in" 
        | "keyword.control.break" | "keyword.control.continue" 
        | "keyword.control.return" | "keyword.control.yield" | "keyword.control.await" 
        | "keyword.control.async" | "keyword.control.unsafe" | "keyword.control.pub" 
        | "keyword.control.crate" | "keyword.control.super" | "keyword.control.self" 
        | "keyword.control.static" | "keyword.control.const" | "keyword.control.mut" 
        | "keyword.control.ref" | "keyword.control.move" | "keyword.control.dyn" 
        | "keyword.control.impl" | "keyword.control.trait" | "keyword.control.enum" 
        | "keyword.control.struct" | "keyword.control.type" | "keyword.control.fn" 
        | "keyword.control.extern" | "keyword.control.macro" | "keyword.control.union" => Highlight::Keyword,
        
        // Functions and methods
        "function" | "function.call" | "function.method" | "function.builtin" 
        | "method" | "method.call" | "constructor" | "function.macro" | "macro" => Highlight::Function,
        
        // Variables
        "variable" | "variable.parameter" | "variable.other" | "variable.other.member" 
        | "label" | "definition" => Highlight::Variable,
        
        // Built-in variables
        "variable.builtin" | "variable.language" | "variable.special" => Highlight::Type,
        
        // Types
        "type" | "type.builtin" | "type.parameter" | "type.qualifier" 
        | "lifetime" | "storageclass" => Highlight::Type,
        
        // Constants
        "constant" | "constant.builtin" | "boolean" | "constant.language" 
        | "constant.numeric" | "constant.character" | "constant.other" => Highlight::Constant,
        
        // Attributes
        "attribute" | "attribute.builtin" | "decorator" | "annotation" => Highlight::Attribute,
        
        // Operators
        "operator" | "operator.assignment" | "operator.arithmetic" 
        | "operator.comparison" | "operator.logical" | "operator.bitwise" 
        | "operator.unary" | "operator.ternary" | "operator.spread" 
        | "punctuation.bracket" | "punctuation.delimiter" | "punctuation.special" 
        | "punctuation" => Highlight::Operator,
        
        // Numbers
        "number" | "number.float" | "number.integer" | "number.hex" 
        | "number.octal" | "number.binary" | "number.scientific" => Highlight::Number,
        
        // Properties
        "property" | "property.definition" | "property.readonly" 
        | "field" | "field.definition" => Highlight::Property,
        
        // Namespaces
        "namespace" | "module" | "package" | "import" | "include" => Highlight::Namespace,
        
        // Markdown-specific captures
        "emphasis" | "text.emphasis" => Highlight::Comment,
        "strong_emphasis" | "text.strong" => Highlight::Keyword,
        "code_span" | "inline_code" | "text.literal" | "code_block.content" => Highlight::Constant,
        "link_text" | "shortcut_link" | "full_reference_link" | "collapsed_reference_link" 
        | "inline_link" | "link" | "reference_link" | "reference_definition" 
        | "footnote_reference" | "footnote_definition" | "text.reference" => Highlight::Variable,
        "link_destination" | "link_title" | "uri_autolink" | "email_autolink" 
        | "url" | "email" | "text.uri" => Highlight::String,
        "image" | "image.description" => Highlight::Variable,
        "html_tag" | "html" | "html_block" | "html_inline" | "tag" => Highlight::Attribute,
        "hard_line_break" | "line_break" | "soft_line_break" => Highlight::Operator,
        "strikethrough" => Highlight::Comment,
        "backslash_escape" => Highlight::String,
        "latex" | "text.math" => Highlight::Constant,
        
        // Headings
        "heading" | "heading.1" | "heading.2" | "heading.3" | "heading.4" | "heading.5" | "heading.6" 
        | "atx_heading" | "setext_heading" | "setext_heading_text" | "heading_content" 
        | "text.title" => Highlight::Type,
        
        // Code blocks
        "code_block" | "fenced_code_block" | "code_block.delimiter" | "fenced_code_block_delimiter" => Highlight::Property,
        
        // Blockquotes
        "blockquote" | "block_quote" | "text.quote" => Highlight::Comment,
        "blockquote.marker" | "block_quote_marker" => Highlight::Operator,
        
        // Lists
        "list" | "list.item" | "list_item" | "text.environment" => Highlight::Property,
        "list.marker" | "list_item_marker" | "task_list_marker" => Highlight::Operator,
        
        // Tables
        "table" | "table.row" => Highlight::Property,
        "table.header" => Highlight::Type,
        "table.cell" => Highlight::Plain,
        "table.delimiter" => Highlight::Operator,
        
        // Other
        "thematic_break" | "atx_heading_marker" => Highlight::Operator,
        "paragraph" | "inline" | "block" | "document" | "plain" => Highlight::Plain,
        
        // Catch-all
        _ => Highlight::Plain,
    }
}

/// Retrieve the highlights.scm query text for the given language.
///
/// Returns an owned static string reference for efficient reuse, or an empty
/// string when no query is available. Read errors are logged and a graceful
/// empty query is returned so highlighting can fail softly.
pub fn get_query_for_language(language: LanguageId) -> Result<&'static str, SyntaxError> {
    let language_id = language.as_str();

    // For plaintext, return an empty query
    if language_id == "plaintext" {
        return Ok("");
    }

    let runtime = crate::runtime::Runtime::new();
    let query_path = runtime.language_dir(language_id).join("queries/highlights.scm");

    if query_path.exists() {
        match std::fs::read_to_string(&query_path) {
            Ok(query_text) => {
                // Check if the query is not empty
                if query_text.trim().is_empty() {
                    eprintln!("DEBUG: Query file for {} is empty", language_id);
                    return Ok("");
                }

                // Leak the string to make it static
                let leaked = Box::leak(query_text.into_boxed_str());
                Ok(leaked)
            }
            Err(e) => {
                // If we can't read the query file, return an empty query instead of failing
                // This allows syntax highlighting to fall back gracefully
                eprintln!("Warning: Failed to read query file for {}: {}", language_id, e);
                Ok("")
            }
        }
    } else {
        // If no query file exists, return an empty query
        // This is better than failing, as it allows the editor to work without syntax highlighting
        eprintln!("Warning: No query file found for {} at {}", language_id, query_path.display());
        Ok("")
    }
}
