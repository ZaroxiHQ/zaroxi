//! Syntax layer for Neote IDE.
//!
//! This crate provides Tree-sitter-based syntax parsing, highlighting,
//! and language support for the editor. It's designed to be:
//! - Incremental: updates syntax trees efficiently after edits
//! - Modular: clean separation between parsing, highlighting, and UI
//! - Extensible: easy to add new languages and features
//! - Performant: minimal overhead for large files and frequent edits

pub mod cache;
pub mod dynamic_loader;
pub mod error;
pub mod grammar_builder;
pub mod grammar_registry;
pub mod highlight;
pub mod language;
pub mod manager;
pub mod parser;
pub mod query_cache;
pub mod runtime;
pub mod theme_map;

// Re-export main types for convenience
pub use dynamic_loader::DynamicGrammarLoader;
pub use error::SyntaxError;
pub use grammar_builder::build_and_install_grammar;
pub use grammar_registry::{
    GrammarInfo,
    for_language,
    available_languages,
    is_grammar_installed,
    download_and_install_grammar,
    install_missing_grammars,
};
pub use highlight::{Highlight, HighlightSpan};
pub use language::LanguageId;
pub use parser::{SyntaxTree, ParserPool};
// Note: QueryCache::get returns Option<&'static Query>
pub use query_cache::QueryCache;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_language_detection() {
        use std::path::Path;

        assert_eq!(LanguageId::from_path(Path::new("Cargo.toml")), LanguageId::Toml);
        assert_eq!(LanguageId::from_path(Path::new("test.toml")), LanguageId::Toml);
        assert_eq!(LanguageId::from_path(Path::new(".clippy.toml")), LanguageId::Toml);
        assert_eq!(LanguageId::from_path(Path::new("pyproject.toml")), LanguageId::Toml);
        assert_eq!(LanguageId::from_path(Path::new("rustfmt.toml")), LanguageId::Toml);
        assert_eq!(LanguageId::from_path(Path::new("config.toml")), LanguageId::Toml);
    }

    #[test]
    fn test_markdown_language_detection() {
        use std::path::Path;

        assert_eq!(LanguageId::from_path(Path::new("README.md")), LanguageId::Markdown);
        assert_eq!(LanguageId::from_path(Path::new("document.markdown")), LanguageId::Markdown);
        assert_eq!(LanguageId::from_path(Path::new("notes.MD")), LanguageId::Markdown);
        assert_eq!(
            LanguageId::from_path(Path::new("test.mdx")),
            LanguageId::PlainText // .mdx is not supported by default
        );
    }

    #[test]
    fn test_markdown_highlight_captures() {
        use crate::highlight::map_capture_name;

        assert_eq!(map_capture_name("emphasis"), Highlight::Comment);
        assert_eq!(map_capture_name("strong_emphasis"), Highlight::Keyword);
        assert_eq!(map_capture_name("code_span"), Highlight::Constant);
        assert_eq!(map_capture_name("inline_code"), Highlight::Constant);
        assert_eq!(map_capture_name("link_text"), Highlight::Variable);
        assert_eq!(map_capture_name("link_destination"), Highlight::String);
        assert_eq!(map_capture_name("link_title"), Highlight::String);
        assert_eq!(map_capture_name("shortcut_link"), Highlight::Variable);
        assert_eq!(map_capture_name("full_reference_link"), Highlight::Variable);
        assert_eq!(map_capture_name("collapsed_reference_link"), Highlight::Variable);
        assert_eq!(map_capture_name("inline_link"), Highlight::Variable);
        assert_eq!(map_capture_name("image"), Highlight::Variable);
        assert_eq!(map_capture_name("image.description"), Highlight::Variable);
        assert_eq!(map_capture_name("html_tag"), Highlight::Attribute);
        assert_eq!(map_capture_name("hard_line_break"), Highlight::Operator);
        assert_eq!(map_capture_name("line_break"), Highlight::Operator);
        assert_eq!(map_capture_name("strikethrough"), Highlight::Comment);
        assert_eq!(map_capture_name("uri_autolink"), Highlight::String);
        assert_eq!(map_capture_name("email_autolink"), Highlight::String);
        assert_eq!(map_capture_name("backslash_escape"), Highlight::String);
        assert_eq!(map_capture_name("escape"), Highlight::String);
        assert_eq!(map_capture_name("latex"), Highlight::Constant);

        assert_eq!(map_capture_name("heading"), Highlight::Type);
        assert_eq!(map_capture_name("link"), Highlight::Variable);
        assert_eq!(map_capture_name("blockquote"), Highlight::Comment);
        assert_eq!(map_capture_name("list"), Highlight::Property);
        assert_eq!(map_capture_name("thematic_break"), Highlight::Operator);

        assert_eq!(map_capture_name("comment"), Highlight::Comment);
        assert_eq!(map_capture_name("string"), Highlight::String);
        assert_eq!(map_capture_name("keyword"), Highlight::Keyword);
    }

    #[test]
    fn test_markdown_language_parsing() {
        use crate::language::LanguageId;
        use std::path::Path;

        let lang = LanguageId::Markdown;
        assert_eq!(lang.as_str(), "markdown");

        let _ = lang.tree_sitter_language();
    }

    #[test]
    fn test_dynamic_language_registry() {
        use crate::grammar_registry::GrammarRegistry;

        let registry = GrammarRegistry::global();
        assert!(registry.contains_language("rust"));
        assert!(registry.contains_language("toml"));
        assert!(registry.contains_language("markdown"));

        let rust_info = registry.get("rust").unwrap();
        assert_eq!(rust_info.name, "Rust");
        assert!(rust_info.extensions.contains(&"rs".to_string()));
    }
}
