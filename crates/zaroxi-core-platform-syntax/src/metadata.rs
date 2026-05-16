//! Language metadata definition and loading.

use serde::Deserialize;
use std::path::PathBuf;

use crate::runtime::Runtime;
use crate::SyntaxError;

/// Metadata describing a Tree-sitter language.
#[derive(Debug, Clone, Deserialize)]
pub struct LanguageMetadata {
    /// Unique identifier (e.g., "rust", "toml").
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// File extensions that trigger this language, without the leading dot.
    pub extensions: Vec<String>,
    /// Optional exact filenames that trigger this language (e.g., "Cargo.toml").
    pub filenames: Vec<String>,
    /// Name of the shared library file (without directory).
    /// Example: "libtree-sitter-rust.so"
    pub library: Option<String>,
    /// Name of the Tree-sitter language symbol (e.g., "tree_sitter_rust").
    /// If omitted, defaults to `tree_sitter_{id}`.
    pub symbol: Option<String>,
    /// Subdirectory (relative to the language directory) containing query files.
    /// Defaults to "queries".
    pub queries: Option<String>,
    /// Whether this language is built‑in and always available.
    #[serde(default)]
    pub builtin: bool,
}

impl LanguageMetadata {
    /// Load metadata for all languages present in the runtime.
    ///
    /// If the runtime directory does not exist or contains no valid metadata,
    /// returns a minimal set of built‑in languages (at least "plaintext").
    pub fn load_all(runtime: &Runtime) -> Result<Vec<Self>, SyntaxError> {
        let mut languages = Vec::new();

        let languages_root = runtime.root().join("languages");
        if languages_root.is_dir() {
            for entry in std::fs::read_dir(&languages_root)
                .map_err(|e| SyntaxError::MetadataError(format!("Cannot read languages dir: {}", e)))?
            {
                let entry = entry.map_err(|e| SyntaxError::MetadataError(format!("Dir entry error: {}", e)))?;
                let lang_dir = entry.path();
                if lang_dir.is_dir() {
                    let toml_path = lang_dir.join("language.toml");
                    if toml_path.is_file() {
                        let content = std::fs::read_to_string(&toml_path)
                            .map_err(|e| SyntaxError::MetadataError(format!("Failed to read {}: {}", toml_path.display(), e)))?;
                        let meta: Self = ::toml::from_str(&content)
                            .map_err(|e| SyntaxError::MetadataError(format!("Invalid TOML in {}: {}", toml_path.display(), e)))?;
                        languages.push(meta);
                    }
                }
            }
        }

        // Ensure plaintext is always present.
        if !languages.iter().any(|l| l.id == "plaintext") {
            languages.push(Self::plaintext());
        }

        Ok(languages)
    }

    /// Create the default "plaintext" language metadata.
    pub fn plaintext() -> Self {
        Self {
            id: "plaintext".to_string(),
            name: "Plain Text".to_string(),
            extensions: Vec::new(),
            filenames: Vec::new(),
            library: None,
            symbol: None,
            queries: None,
            builtin: true,
        }
    }

    /// Create a default "rust" language metadata (used as fallback).
    pub fn rust() -> Self {
        Self {
            id: "rust".to_string(),
            name: "Rust".to_string(),
            extensions: vec!["rs".to_string()],
            filenames: Vec::new(),
            library: Some(if cfg!(windows) {
                "tree-sitter-rust.dll".to_string()
            } else if cfg!(target_os = "macos") {
                "libtree-sitter-rust.dylib".to_string()
            } else {
                "libtree-sitter-rust.so".to_string()
            }),
            symbol: Some("tree_sitter_rust".to_string()),
            queries: Some("queries".to_string()),
            builtin: true,
        }
    }

    /// Get the query directory path for this language.
    pub fn query_dir(&self, runtime: &Runtime) -> PathBuf {
        let lang_dir = runtime.language_dir(&self.id);
        let subdir = self.queries.as_deref().unwrap_or("queries");
        lang_dir.join(subdir)
    }

    /// Read the highlights query file.
    pub fn load_highlights_query(&self, runtime: &Runtime) -> Result<String, SyntaxError> {
        let query_path = self.query_dir(runtime).join("highlights.scm");
        std::fs::read_to_string(&query_path)
            .map_err(|e| SyntaxError::QueryError(format!("Failed to read {}: {}", query_path.display(), e)))
    }

    /// Determine if this language is supported (has a grammar and queries).
    pub fn is_supported(&self) -> bool {
        self.library.is_some()
    }
}
