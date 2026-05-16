//! Registry of available Tree-sitter grammars and their download/compile instructions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Information needed to download and compile a Tree-sitter grammar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarInfo {
    /// Language identifier (e.g., "markdown", "rust", "python")
    pub language_id: String,
    /// Human-readable name
    pub name: String,
    /// File extensions (without dot)
    pub extensions: Vec<String>,
    /// Exact filenames that trigger this language
    pub filenames: Vec<String>,
    /// GitHub repository URL
    pub repo_url: String,
    /// Repository revision/tag (e.g., "v0.20.0")
    pub revision: String,
    /// Optional subdirectory within the repo (for mono-repos)
    pub subdirectory: Option<String>,
    /// Source files needed for compilation
    pub source_files: Vec<String>,
    /// Query files to copy
    pub query_files: Vec<String>,
    /// Whether this grammar has an external scanner
    pub has_scanner: bool,
    /// Scanner language (C or C++)
    pub scanner_lang: Option<String>,
}

/// Global grammar registry
pub struct GrammarRegistry {
    languages: HashMap<&'static str, GrammarInfo>,
}

impl GrammarRegistry {
    /// Get the global grammar registry
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<GrammarRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let mut registry = GrammarRegistry { languages: HashMap::new() };
            registry.load_defaults();
            registry
        })
    }

    fn load_defaults(&mut self) {
        // Rust
        self.add_language(GrammarInfo {
            language_id: "rust".to_string(),
            name: "Rust".to_string(),
            extensions: vec!["rs".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-rust".to_string(),
            revision: "v0.24.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string(), "src/scanner.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "injections.scm".to_string(),
                "locals.scm".to_string(),
                "language.toml".to_string(),
            ],
            has_scanner: true,
            scanner_lang: Some("c".to_string()),
        });

        // TOML - moved to tree-sitter-grammars organization
        self.add_language(GrammarInfo {
            language_id: "toml".to_string(),
            name: "TOML".to_string(),
            extensions: vec!["toml".to_string()],
            filenames: vec!["Cargo.toml".to_string(), "rust-toolchain.toml".to_string()],
            repo_url: "https://github.com/tree-sitter-grammars/tree-sitter-toml".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec!["highlights.scm".to_string(), "language.toml".to_string()],
            has_scanner: false,
            scanner_lang: None,
        });

        // Markdown - using tree-sitter-markdown-inline directory
        // Note: This is the inline-only grammar, which only handles inline elements
        // Block-level elements like headings, lists, etc. are not parsed by this grammar
        self.add_language(GrammarInfo {
            language_id: "markdown".to_string(),
            name: "Markdown".to_string(),
            extensions: vec!["md".to_string(), "markdown".to_string()],
            filenames: vec!["README.md".to_string()],
            repo_url: "https://github.com/tree-sitter-grammars/tree-sitter-markdown".to_string(),
            revision: "split_parser".to_string(),
            subdirectory: Some("tree-sitter-markdown-inline".to_string()),
            source_files: vec!["src/parser.c".to_string(), "src/scanner.c".to_string()],
            query_files: vec!["highlights.scm".to_string(), "injections.scm".to_string()],
            has_scanner: true,
            scanner_lang: Some("c".to_string()),
        });

        // JavaScript
        self.add_language(GrammarInfo {
            language_id: "javascript".to_string(),
            name: "JavaScript".to_string(),
            extensions: vec!["js".to_string(), "jsx".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-javascript".to_string(),
            revision: "v0.22.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string(), "src/scanner.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "injections.scm".to_string(),
                "locals.scm".to_string(),
            ],
            has_scanner: true,
            scanner_lang: Some("c".to_string()),
        });

        // Python
        self.add_language(GrammarInfo {
            language_id: "python".to_string(),
            name: "Python".to_string(),
            extensions: vec!["py".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-python".to_string(),
            revision: "v0.22.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string(), "src/scanner.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "injections.scm".to_string(),
                "locals.scm".to_string(),
            ],
            has_scanner: true,
            scanner_lang: Some("c".to_string()),
        });

        // JSON
        self.add_language(GrammarInfo {
            language_id: "json".to_string(),
            name: "JSON".to_string(),
            extensions: vec!["json".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-json".to_string(),
            revision: "v0.22.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec!["highlights.scm".to_string()],
            has_scanner: false,
            scanner_lang: None,
        });

        // CSS
        self.add_language(GrammarInfo {
            language_id: "css".to_string(),
            name: "CSS".to_string(),
            extensions: vec!["css".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-css".to_string(),
            revision: "v0.21.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // HTML
        self.add_language(GrammarInfo {
            language_id: "html".to_string(),
            name: "HTML".to_string(),
            extensions: vec!["html".to_string(), "htm".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-html".to_string(),
            revision: "v0.21.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // Go
        self.add_language(GrammarInfo {
            language_id: "go".to_string(),
            name: "Go".to_string(),
            extensions: vec!["go".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-go".to_string(),
            revision: "v0.21.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // Java
        self.add_language(GrammarInfo {
            language_id: "java".to_string(),
            name: "Java".to_string(),
            extensions: vec!["java".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-java".to_string(),
            revision: "v0.21.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // Bash
        self.add_language(GrammarInfo {
            language_id: "bash".to_string(),
            name: "Bash".to_string(),
            extensions: vec!["sh".to_string(), "bash".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-bash".to_string(),
            revision: "v0.21.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // C
        self.add_language(GrammarInfo {
            language_id: "c".to_string(),
            name: "C".to_string(),
            extensions: vec!["c".to_string(), "h".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-c".to_string(),
            revision: "v0.21.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // C++
        self.add_language(GrammarInfo {
            language_id: "cpp".to_string(),
            name: "C++".to_string(),
            extensions: vec![
                "cpp".to_string(),
                "cc".to_string(),
                "cxx".to_string(),
                "hpp".to_string(),
                "hh".to_string(),
                "hxx".to_string(),
            ],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-cpp".to_string(),
            revision: "v0.21.0".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // C#
        self.add_language(GrammarInfo {
            language_id: "c_sharp".to_string(),
            name: "C#".to_string(),
            extensions: vec!["cs".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-c-sharp".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // Ruby
        self.add_language(GrammarInfo {
            language_id: "ruby".to_string(),
            name: "Ruby".to_string(),
            extensions: vec!["rb".to_string()],
            filenames: vec!["Gemfile".to_string(), "Rakefile".to_string()],
            repo_url: "https://github.com/tree-sitter/tree-sitter-ruby".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // TypeScript
        self.add_language(GrammarInfo {
            language_id: "typescript".to_string(),
            name: "TypeScript".to_string(),
            extensions: vec!["ts".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-typescript".to_string(),
            revision: "master".to_string(),
            subdirectory: Some("typescript".to_string()),
            source_files: vec!["src/parser.c".to_string(), "src/scanner.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: true,
            scanner_lang: Some("c".to_string()),
        });

        // TSX
        self.add_language(GrammarInfo {
            language_id: "tsx".to_string(),
            name: "TSX".to_string(),
            extensions: vec!["tsx".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter/tree-sitter-typescript".to_string(),
            revision: "master".to_string(),
            subdirectory: Some("tsx".to_string()),
            source_files: vec!["src/parser.c".to_string(), "src/scanner.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: true,
            scanner_lang: Some("c".to_string()),
        });

        // Lua
        self.add_language(GrammarInfo {
            language_id: "lua".to_string(),
            name: "Lua".to_string(),
            extensions: vec!["lua".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter-grammars/tree-sitter-lua".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // YAML
        self.add_language(GrammarInfo {
            language_id: "yaml".to_string(),
            name: "YAML".to_string(),
            extensions: vec!["yaml".to_string(), "yml".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter-grammars/tree-sitter-yaml".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // Zig
        self.add_language(GrammarInfo {
            language_id: "zig".to_string(),
            name: "Zig".to_string(),
            extensions: vec!["zig".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/tree-sitter-grammars/tree-sitter-zig".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // CMake
        self.add_language(GrammarInfo {
            language_id: "cmake".to_string(),
            name: "CMake".to_string(),
            extensions: vec!["cmake".to_string()],
            filenames: vec!["CMakeLists.txt".to_string()],
            repo_url: "https://github.com/uyha/tree-sitter-cmake".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // Dockerfile
        self.add_language(GrammarInfo {
            language_id: "dockerfile".to_string(),
            name: "Dockerfile".to_string(),
            extensions: vec![],
            filenames: vec!["Dockerfile".to_string()],
            repo_url: "https://github.com/camdencheek/tree-sitter-dockerfile".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // Elixir
        self.add_language(GrammarInfo {
            language_id: "elixir".to_string(),
            name: "Elixir".to_string(),
            extensions: vec!["ex".to_string(), "exs".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/elixir-lang/tree-sitter-elixir".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });

        // Nix
        self.add_language(GrammarInfo {
            language_id: "nix".to_string(),
            name: "Nix".to_string(),
            extensions: vec!["nix".to_string()],
            filenames: vec![],
            repo_url: "https://github.com/nix-community/tree-sitter-nix".to_string(),
            revision: "master".to_string(),
            subdirectory: None,
            source_files: vec!["src/parser.c".to_string()],
            query_files: vec![
                "highlights.scm".to_string(),
                "locals.scm".to_string(),
                "injections.scm".to_string(),
            ],
            has_scanner: false,
            scanner_lang: None,
        });
    }

    fn add_language(&mut self, info: GrammarInfo) {
        self.languages.insert(Box::leak(info.language_id.clone().into_boxed_str()), info);
    }

    /// Get information for a specific language
    pub fn get(&self, language_id: &str) -> Option<&GrammarInfo> {
        self.languages.get(language_id)
    }

    /// Check if a language is in the registry
    pub fn contains_language(&self, language_id: &str) -> bool {
        self.languages.contains_key(language_id)
    }

    /// Get all language IDs
    pub fn language_ids(&self) -> Vec<&str> {
        self.languages.keys().copied().collect()
    }

    /// Get all languages
    pub fn languages(&self) -> &HashMap<&'static str, GrammarInfo> {
        &self.languages
    }
}

/// Get the grammar info for a language, if available
pub fn for_language(language_id: &str) -> Option<GrammarInfo> {
    GrammarRegistry::global().get(language_id).cloned()
}

/// Get all available language IDs
pub fn available_languages() -> Vec<String> {
    GrammarRegistry::global().language_ids().iter().map(|s| s.to_string()).collect()
}

/// Check if a grammar is installed in the runtime directory.
pub fn is_grammar_installed(language_id: &str) -> bool {
    let runtime = crate::runtime::Runtime::new();
    let lib_path = runtime.grammar_library_path(language_id);
    lib_path.exists()
}

/// Download and compile a missing grammar into the runtime directory.
///
/// This function:
/// 1. Clones the grammar repository
/// 2. Compiles the grammar using the C compiler
/// 3. Copies the resulting shared library to the runtime directory
/// 4. Copies query files to the runtime directory
///
/// Returns an error if the grammar cannot be downloaded or compiled.
pub fn download_and_install_grammar(language_id: &str) -> Result<(), String> {
    let info = GrammarRegistry::global()
        .get(language_id)
        .ok_or_else(|| format!("Unknown language: {}", language_id))?;

    let runtime = crate::runtime::Runtime::new();
    let grammars_dir = runtime.grammar_dir();
    let languages_dir = runtime.language_dir(language_id);

    // Create directories if they don't exist
    std::fs::create_dir_all(&grammars_dir)
        .map_err(|e| format!("Failed to create grammars dir: {}", e))?;
    std::fs::create_dir_all(&languages_dir)
        .map_err(|e| format!("Failed to create languages dir: {}", e))?;

    // Create a unique temporary directory for cloning and compiling
    let temp_dir = std::env::temp_dir().join(format!("zaroxi-grammar-{}-{}", language_id, std::process::id()));
    if temp_dir.exists() {
        // Remove the entire directory to ensure a clean clone
        std::fs::remove_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to clean temp dir: {}", e))?;
    }

    // Clone the repository
    let repo_url = &info.repo_url;
    let revision = &info.revision;

    // Use git clone with --force to overwrite existing directory
    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", "--branch", revision, "--force", repo_url, temp_dir.to_str().unwrap()])
        .status()
        .map_err(|e| format!("Failed to run git clone: {}", e))?;

    if !status.success() {
        // Clean up temp directory on failure
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!("git clone failed for {}", language_id));
    }

    // Determine the source directory (handle subdirectories)
    let source_dir = if let Some(subdir) = &info.subdirectory {
        temp_dir.join(subdir)
    } else {
        temp_dir.clone()
    };

    // Compile the grammar
    let output_lib = if cfg!(windows) {
        grammars_dir.join(format!("tree-sitter-{}.dll", language_id))
    } else if cfg!(target_os = "macos") {
        grammars_dir.join(format!("libtree-sitter-{}.dylib", language_id))
    } else {
        grammars_dir.join(format!("libtree-sitter-{}.so", language_id))
    };

    // Check if we're in a cargo build environment (TARGET env var is set)
    let in_cargo_build = std::env::var("TARGET").is_ok();

    if in_cargo_build {
        // Build the C source files into a shared library using the cc crate
        let mut build = cc::Build::new();
        build.opt_level(2);
        build.cpp(false); // C language, not C++

        for src_file in &info.source_files {
            let src_path = source_dir.join(src_file);
            if src_path.exists() {
                build.file(&src_path);
            } else {
                eprintln!("Warning: Source file not found: {}", src_path.display());
            }
        }

        // The cc crate outputs to OUT_DIR, we need to find and copy the library
        let lib_name = format!("tree_sitter_{}", language_id);
        build.compile(&lib_name);

        // Find the compiled library in OUT_DIR
        let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "/tmp".to_string());
        let out_path = std::path::PathBuf::from(&out_dir);

        // The cc crate creates a library with a specific naming pattern
        let built_lib = if cfg!(windows) {
            out_path.join(format!("{}.dll", lib_name))
        } else if cfg!(target_os = "macos") {
            out_path.join(format!("lib{}.dylib", lib_name))
        } else {
            out_path.join(format!("lib{}.so", lib_name))
        };

        if built_lib.exists() {
            std::fs::copy(&built_lib, &output_lib)
                .map_err(|e| format!("Failed to copy library: {}", e))?;
            eprintln!("Copied grammar library to {}", output_lib.display());
        } else {
            // Try to find the library with alternative naming (static library)
            let alt_lib = if cfg!(windows) {
                out_path.join(format!("{}.lib", lib_name))
            } else if cfg!(target_os = "macos") {
                out_path.join(format!("lib{}.a", lib_name))
            } else {
                out_path.join(format!("lib{}.a", lib_name))
            };

            if alt_lib.exists() {
                std::fs::copy(&alt_lib, &output_lib)
                    .map_err(|e| format!("Failed to copy static library: {}", e))?;
                eprintln!("Copied static grammar library to {}", output_lib.display());
            } else {
                // Clean up temp directory
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!(
                    "Could not find compiled library for {} in {}",
                    language_id,
                    out_dir
                ));
            }
        }
    } else {
        // Not in cargo build environment, try to use system cc directly
        eprintln!("DEBUG: Not in cargo build environment, trying system cc for {}", language_id);

        // Build using system cc
        let mut cc_cmd = std::process::Command::new("cc");
        cc_cmd.arg("-shared").arg("-fPIC").arg("-O2");

        for src_file in &info.source_files {
            let src_path = source_dir.join(src_file);
            if src_path.exists() {
                cc_cmd.arg(src_path.to_str().unwrap());
            } else {
                eprintln!("Warning: Source file not found: {}", src_path.display());
            }
        }

        cc_cmd.arg("-o").arg(output_lib.to_str().unwrap());

        let status = cc_cmd.status()
            .map_err(|e| format!("Failed to run system cc: {}", e))?;

        if !status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(format!("System cc compilation failed for {}", language_id));
        }

        eprintln!("Copied grammar library to {}", output_lib.display());
    }

    // Copy query files
    let queries_dir = languages_dir.join("queries");
    std::fs::create_dir_all(&queries_dir)
        .map_err(|e| format!("Failed to create queries dir: {}", e))?;

    for query_file in &info.query_files {
        let src_path = source_dir.join("queries").join(query_file);
        if src_path.exists() {
            let dst_path = queries_dir.join(query_file);
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy query file {}: {}", query_file, e))?;
        }
    }

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(())
}

/// Install all missing grammars.
///
/// This function checks which grammars are not yet installed in the runtime
/// directory and downloads/compiles them.
pub fn install_missing_grammars() -> Vec<String> {
    let mut installed = Vec::new();
    let registry = GrammarRegistry::global();

    for language_id in registry.language_ids() {
        if !is_grammar_installed(language_id) {
            match download_and_install_grammar(language_id) {
                Ok(()) => {
                    eprintln!("Installed grammar for {}", language_id);
                    installed.push(language_id.to_string());
                }
                Err(e) => {
                    eprintln!("Failed to install grammar for {}: {}", language_id, e);
                }
            }
        }
    }

    installed
}
