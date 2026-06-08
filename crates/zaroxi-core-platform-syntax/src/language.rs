//! Language identification and grammar loading.

use std::path::Path;

/// Static and dynamic language identifiers used by the syntax subsystem.
///
/// - `Rust`, `Toml`, `Markdown`, `PlainText` are known static variants.
/// - `Dynamic(&'static str)` represents a runtime-registered grammar id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    /// Rust files (*.rs)
    Rust,
    /// TOML files (*.toml)
    Toml,
    /// Markdown files (*.md)
    Markdown,
    /// Plain text (no syntax)
    PlainText,
    /// Dynamic grammar identified by a `'static` string id
    Dynamic(&'static str),
}

impl LanguageId {
    /// Determine language from file path.
    pub fn from_path(path: &Path) -> Self {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();

        // Try to match against dynamic language registry first.
        if let Some(lang_id) = Self::from_filename_dynamic(&name) {
            return LanguageId::Dynamic(lang_id);
        }
        if let Some(lang_id) = Self::from_extension_dynamic(ext) {
            return LanguageId::Dynamic(lang_id);
        }

        // Fall back to well‑known built‑in mappings and a broad set of
        // common programming language extensions (map to dynamic grammars
        // when not represented as a static enum variant).
        //
        // Important:
        // - Do NOT return `LanguageId::Dynamic(ext)` for non‑'static ext strings.
        //   `from_extension_dynamic` / `from_filename_dynamic` above already consult
        //   the runtime registry and return &'static ids when available.
        // - If no dynamic grammar is registered for an extension, we don't fabricate
        //   a `Dynamic` with a borrowed ext (that would require 'static lifetime).
        match ext {
            // Rust / TOML / Markdown (static)
            "rs" => return LanguageId::Rust,
            "toml" => return LanguageId::Toml,
            "md" | "markdown" => return LanguageId::Markdown,

            // JavaScript / TypeScript family
            "js" | "jsx" | "mjs" | "cjs" => return LanguageId::Dynamic("javascript"),
            "ts" | "mts" | "cts" => return LanguageId::Dynamic("typescript"),
            "tsx" => return LanguageId::Dynamic("tsx"),

            // Python
            "py" | "pyi" => return LanguageId::Dynamic("python"),

            // Web / data formats
            "json" => return LanguageId::Dynamic("json"),
            "css" => return LanguageId::Dynamic("css"),
            "scss" => return LanguageId::Dynamic("scss"),
            "less" => return LanguageId::Dynamic("less"),
            "html" | "htm" | "xhtml" => return LanguageId::Dynamic("html"),
            "xml" | "xsd" | "xsl" => return LanguageId::Dynamic("xml"),
            "yaml" | "yml" => return LanguageId::Dynamic("yaml"),
            "ini" => return LanguageId::Dynamic("ini"),

            // Go / Java / Kotlin / Scala
            "go" => return LanguageId::Dynamic("go"),
            "java" => return LanguageId::Dynamic("java"),
            "kt" | "kts" => return LanguageId::Dynamic("kotlin"),
            "scala" => return LanguageId::Dynamic("scala"),

            // C family
            "c" | "h" => return LanguageId::Dynamic("c"),
            "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "ipp" => {
                return LanguageId::Dynamic("cpp");
            }
            "cs" => return LanguageId::Dynamic("c_sharp"),
            "objc" | "mm" => return LanguageId::Dynamic("objective_c"),

            // Ruby / PHP / Perl / Lua
            "rb" => return LanguageId::Dynamic("ruby"),
            "php" => return LanguageId::Dynamic("php"),
            "pl" | "pm" => return LanguageId::Dynamic("perl"),
            "lua" => return LanguageId::Dynamic("lua"),

            // Shells and scripts
            "sh" | "bash" | "zsh" | "ksh" => return LanguageId::Dynamic("bash"),
            "ps1" | "psm1" | "psd1" => return LanguageId::Dynamic("powershell"),

            // Functional / niche languages
            "hs" => return LanguageId::Dynamic("haskell"),
            "erl" | "hrl" => return LanguageId::Dynamic("erlang"),
            "ex" | "exs" => return LanguageId::Dynamic("elixir"),
            "clj" | "cljs" | "cljc" => return LanguageId::Dynamic("clojure"),
            "rsw" => return LanguageId::Dynamic("rescript"),

            // Scientific / data languages
            "r" => return LanguageId::Dynamic("r"),
            "jl" => return LanguageId::Dynamic("julia"),
            "ipynb" => return LanguageId::Dynamic("json"),

            // Other common languages
            "dart" => return LanguageId::Dynamic("dart"),
            "swift" => return LanguageId::Dynamic("swift"),
            "sql" => return LanguageId::Dynamic("sql"),
            "pp" => return LanguageId::Dynamic("puppet"),
            "gradle" | "groovy" => return LanguageId::Dynamic("groovy"),
            "make" | "mak" => return LanguageId::Dynamic("makefile"),
            "cmake" => return LanguageId::Dynamic("cmake"),
            "zig" => return LanguageId::Dynamic("zig"),
            "nix" => return LanguageId::Dynamic("nix"),

            // If we reach here, prefer to rely on the dynamic registry (checked earlier).
            // Do not fabricate a Dynamic variant from a borrowed `ext` string.
            _ => {}
        }

        // Check specific filenames.
        match name.as_str() {
            "cargo.toml"
            | "rust-toolchain.toml"
            | "clippy.toml"
            | "rustfmt.toml"
            | ".clippy.toml"
            | ".rustfmt.toml"
            | "pyproject.toml"
            | "taplo.toml" => {
                return LanguageId::Toml;
            }
            "dockerfile" => return LanguageId::Dynamic("dockerfile"),
            "cmakelists.txt" => return LanguageId::Dynamic("cmake"),
            "gemfile" | "rakefile" => return LanguageId::Dynamic("ruby"),
            _ => {}
        }

        LanguageId::PlainText
    }

    fn from_filename_dynamic(name: &str) -> Option<&'static str> {
        use crate::grammar_registry::GrammarRegistry;
        use std::collections::HashMap;
        use std::sync::OnceLock;

        static FILENAME_MAP: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
        let map = FILENAME_MAP.get_or_init(|| {
            let mut map = HashMap::new();
            let registry = GrammarRegistry::global();
            for (lang_id, info) in registry.languages() {
                for filename in &info.filenames {
                    map.insert(filename.to_lowercase(), *lang_id);
                }
            }
            map
        });

        map.get(&name.to_lowercase()).copied()
    }

    fn from_extension_dynamic(ext: &str) -> Option<&'static str> {
        use crate::grammar_registry::GrammarRegistry;
        use std::collections::HashMap;
        use std::sync::OnceLock;

        static EXTENSION_MAP: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
        let map = EXTENSION_MAP.get_or_init(|| {
            let mut map = HashMap::new();
            let registry = GrammarRegistry::global();
            for (lang_id, info) in registry.languages() {
                for ext in &info.extensions {
                    map.insert(ext.to_lowercase(), *lang_id);
                }
                for filename in &info.filenames {
                    map.insert(filename.to_lowercase(), *lang_id);
                }
            }
            map
        });

        map.get(&ext.to_lowercase()).copied()
    }

    /// Get the canonical string identifier for this language id.
    ///
    /// Returns a `'static` string for static variants and the inner id for
    /// `Dynamic` variants.
    pub fn as_str(&self) -> &str {
        match self {
            LanguageId::Rust => "rust",
            LanguageId::Toml => "toml",
            LanguageId::Markdown => "markdown",
            LanguageId::PlainText => "plaintext",
            LanguageId::Dynamic(id) => id,
        }
    }

    /// Return the Tree‑sitter language, loading dynamically via the
    /// cached dynamic loader (memoized per language_id so repeated
    /// calls on unavailable grammars do not retry the full disk path).
    pub fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        if *self == LanguageId::PlainText {
            return None;
        }
        let id = self.as_str();
        crate::dynamic_loader::load_language(id)
    }
}
