//! Runtime path resolution for Tree-sitter grammars and queries.
//!
//! Resolution is **independent of the process current working directory**.
//! A single function, [`resolve_runtime_root`], is the source of truth for the
//! canonical `runtime/treesitter` root; both grammar shared-library paths and
//! query directories derive from that same root (via [`Runtime`]).
//!
//! Resolution order (first match wins, result is canonicalized to absolute):
//! 1. Explicit env override — `ZAROXI_TREESITTER_RUNTIME_DIR` (preferred), then
//!    the legacy `ZAROXI_STUDIO_RUNTIME`.
//! 2. Executable-relative paths (packaged installs ship the runtime next to the
//!    binary, e.g. `runtime/treesitter` or `../share/.../runtime/treesitter`),
//!    plus a walk-up from the executable directory.
//! 3. The compile-time crate manifest dir of *this* crate
//!    (`CARGO_MANIFEST_DIR/runtime/treesitter`). This is the canonical dev
//!    location and is correct regardless of which binary loaded the crate or
//!    what the cwd is — `env!` is resolved at compile time against
//!    `zaroxi-core-platform-syntax`.
//! 4. cwd-relative `runtime/treesitter` (last resort).

use std::env;
use std::path::{Path, PathBuf};
use std::sync::Once;

/// Environment variables that may point directly at a runtime root, in order of
/// preference. `ZAROXI_TREESITTER_RUNTIME_DIR` is the documented override;
/// `ZAROXI_STUDIO_RUNTIME` is retained for backwards compatibility.
const RUNTIME_OVERRIDE_VARS: &[&str] = &["ZAROXI_TREESITTER_RUNTIME_DIR", "ZAROXI_STUDIO_RUNTIME"];

/// Canonicalize a path to an absolute form, falling back to the original path
/// if canonicalization fails (e.g. the path does not exist). This guarantees we
/// never leak a `./relative` path into libloading or diagnostics when the
/// directory is real.
fn canonical_or(p: PathBuf) -> PathBuf {
    std::fs::canonicalize(&p).unwrap_or(p)
}

/// Emit a one-time, always-on diagnostic when no runtime root can be located.
/// This replaces silent zero-span behavior with an actionable message.
fn warn_runtime_missing() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        eprintln!(
            "ZAROXI_RUNTIME: tree-sitter runtime root not found; syntax highlighting will be disabled. \
             Set ZAROXI_TREESITTER_RUNTIME_DIR to a directory containing 'grammars/' and 'languages/'. \
             (current_exe={:?}, current_dir={:?})",
            env::current_exe().ok(),
            env::current_dir().ok(),
        );
    });
}

/// Resolve the canonical tree-sitter runtime root, independent of cwd.
///
/// Returns `None` if no runtime directory can be located. See the module-level
/// docs for the full resolution order.
pub fn resolve_runtime_root() -> Option<PathBuf> {
    // 1. Explicit env overrides.
    for var in RUNTIME_OVERRIDE_VARS {
        if let Ok(val) = env::var(var) {
            if val.is_empty() {
                continue;
            }
            let pb = PathBuf::from(val);
            if pb.is_dir() {
                return Some(canonical_or(pb));
            }
        }
    }
    resolve_runtime_root_without_env()
}

/// Resolve the runtime root using an explicit override directory instead of the
/// process environment. Primarily intended for tests, which must avoid mutating
/// global process env state. Falls back to the env-free probes when the
/// override is `None` or is not a directory.
pub fn resolve_runtime_root_with_override(override_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = override_dir
        && dir.is_dir()
    {
        return Some(canonical_or(dir.to_path_buf()));
    }
    resolve_runtime_root_without_env()
}

/// Env-independent portion of runtime resolution (steps 2–4).
fn resolve_runtime_root_without_env() -> Option<PathBuf> {
    // 2. Executable-relative (packaged installs).
    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for rel in
            ["runtime/treesitter", "../runtime/treesitter", "../share/zaroxi/runtime/treesitter"]
        {
            let cand = dir.join(rel);
            if cand.is_dir() {
                return Some(canonical_or(cand));
            }
        }

        // Walk up from the executable directory looking for runtime/treesitter.
        let mut cur = dir.to_path_buf();
        while let Some(parent) = cur.parent() {
            let cand = cur.join("runtime/treesitter");
            if cand.is_dir() {
                return Some(canonical_or(cand));
            }
            cur = parent.to_path_buf();
        }
    }

    // 3. Compile-time crate manifest dir: canonical dev location of the bundled
    //    runtime. `env!` resolves at compile time against this crate, so it is
    //    correct regardless of cwd or which binary loaded the crate.
    let crate_runtime =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime").join("treesitter");
    if crate_runtime.is_dir() {
        return Some(canonical_or(crate_runtime));
    }

    // 4. cwd-relative (last resort).
    if let Ok(cwd) = env::current_dir() {
        let cand = cwd.join("runtime").join("treesitter");
        if cand.is_dir() {
            return Some(canonical_or(cand));
        }
    }

    None
}

/// Runtime environment for locating Tree-sitter assets.
#[derive(Debug, Clone)]
pub struct Runtime {
    /// Root directory of the Tree-sitter runtime (e.g., .../runtime/treesitter).
    root: PathBuf,
}

impl Runtime {
    /// Create a `Runtime` by resolving the canonical runtime root.
    ///
    /// If no runtime can be located a one-time diagnostic is emitted and a
    /// relative placeholder is stored so subsequent `.exists()`/path checks fail
    /// loudly (rather than silently producing zero highlight spans).
    pub fn new() -> Self {
        let root = resolve_runtime_root().unwrap_or_else(|| {
            warn_runtime_missing();
            PathBuf::from("./runtime/treesitter")
        });
        let runtime = Self { root };

        // Try to fix nested structure if it exists.
        let _ = runtime.fix_nested_structure();

        runtime
    }

    /// Construct a `Runtime` rooted at an explicit directory (canonicalized).
    /// Used by tests and by callers that resolve the root themselves.
    pub fn with_root(root: PathBuf) -> Self {
        Self { root: canonical_or(root) }
    }

    /// Get the path to the directory containing grammar shared libraries
    /// (flat directory, no platform subdirectory).
    pub fn grammar_dir(&self) -> PathBuf {
        self.root.join("grammars")
    }

    /// Get the path to the language metadata and queries directory for a language.
    pub fn language_dir(&self, language_id: &str) -> PathBuf {
        self.root.join("languages").join(language_id)
    }

    /// Construct the full path to a grammar shared library.
    ///
    /// The library filename is expected to follow the pattern
    /// `libtree-sitter-{language}.{ext}` on Unix and `tree-sitter-{language}.dll` on Windows.
    ///
    /// First, the flat `grammars/` directory is tried; if the library is not found there,
    /// the platform‑specific subdirectory (`grammars/<os>-<arch>/`) is used as a fallback
    /// to support existing installations.
    pub fn grammar_library_path(&self, language_id: &str) -> PathBuf {
        let prefix = if cfg!(windows) { "" } else { "lib" };
        let extension = if cfg!(windows) {
            ".dll"
        } else if cfg!(target_os = "macos") {
            ".dylib"
        } else {
            ".so"
        };
        // Some language IDs use underscores but the library uses hyphens.
        let lib_name = match language_id {
            "c_sharp" => "c-sharp",
            _ => language_id,
        };
        let lib_name = format!("{}tree-sitter-{}{}", prefix, lib_name, extension);

        // First try the flat grammars directory.
        let flat_path = self.root.join("grammars").join(&lib_name);
        if flat_path.exists() {
            return flat_path;
        }

        // Fallback to platform-specific subdirectory.
        let target = env::consts::ARCH;
        let os = env::consts::OS;
        let subdir = format!("{}-{}", os, target);
        self.root.join("grammars").join(&subdir).join(&lib_name)
    }

    /// Load a Tree-sitter language by dynamically opening its shared library.
    ///
    /// Attempts to locate and open the platform shared library for `language_id`
    /// in the runtime directory, looks up the `tree_sitter_{language}` symbol,
    /// and returns the produced `tree_sitter::Language`. Errors are returned as
    /// human-readable strings.
    ///
    /// For the `markdown` language the library may export `tree_sitter_markdown_inline`
    /// instead of `tree_sitter_markdown`.  We try the alternative symbol first if
    /// `language_id == "markdown"`.
    #[cfg(feature = "dynamic-loading")]
    pub fn load_language(&self, language_id: &str) -> Result<tree_sitter::Language, String> {
        use libloading::{Library, Symbol};

        let library_path = self.grammar_library_path(language_id);
        if !library_path.exists() {
            return Err(format!(
                "Grammar library not found at {}\nRun: cargo run --bin download_grammars -- install {}",
                library_path.display(),
                language_id
            ));
        }

        // Safety: We're loading a shared library that we expect to be a valid
        // Tree-sitter grammar. The library should export a function named
        // `tree_sitter_{language}`.
        unsafe {
            let lib = Library::new(&library_path)
                .map_err(|e| format!("Failed to load library {}: {}", library_path.display(), e))?;

            // Try the standard symbol first, then the inline variant for markdown.
            let standard_symbol = format!("tree_sitter_{}", language_id);
            let markdown_inline_symbol = if language_id == "markdown" {
                Some("tree_sitter_markdown_inline".to_string())
            } else {
                None
            };

            let get_symbol = |sym: &str| -> Result<
                Symbol<unsafe extern "C" fn() -> tree_sitter::Language>,
                String,
            > {
                lib.get(sym.as_bytes()).map_err(|e| format!("Failed to get symbol {}: {}", sym, e))
            };

            let symbol = match get_symbol(&standard_symbol) {
                Ok(s) => Some(s),
                Err(e) => {
                    if language_id == "markdown" {
                        match get_symbol(&markdown_inline_symbol.unwrap()) {
                            Ok(s) => Some(s),
                            Err(_) => {
                                return Err(format!(
                                    "Failed to get symbol {}: {}",
                                    standard_symbol, e
                                ));
                            }
                        }
                    } else {
                        return Err(e);
                    }
                }
            };

            // Call the function to get the language.
            let language_fn = symbol.unwrap();
            let language = language_fn();

            // The library must not be unloaded while the language is in use.
            // We leak the library handle to keep it loaded for the lifetime of the program.
            std::mem::forget(lib);

            Ok(language)
        }
    }

    /// Load a Tree-sitter language when dynamic-loading is not enabled.
    ///
    /// Returns an error indicating that the `dynamic-loading` feature is
    /// required to perform runtime library loading.
    #[cfg(not(feature = "dynamic-loading"))]
    pub fn load_language(&self, language_id: &str) -> Result<tree_sitter::Language, String> {
        Err(format!(
            "Dynamic loading not enabled (feature 'dynamic-loading' required) for language {}",
            language_id
        ))
    }

    /// Get a reference to the runtime root directory.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Check whether the runtime root directory exists.
    pub fn exists(&self) -> bool {
        self.root.is_dir()
    }

    /// Fix nested runtime directory structure if found.
    pub fn fix_nested_structure(&self) -> std::io::Result<()> {
        let nested_path = self.root.join("runtime/treesitter");
        if nested_path.is_dir() {
            // Move contents from nested to parent.
            let grammars_nested = nested_path.join("grammars");
            let languages_nested = nested_path.join("languages");

            let grammars_target = self.root.join("grammars");
            let languages_target = self.root.join("languages");

            // Move grammars if they exist.
            if grammars_nested.exists() {
                if !grammars_target.exists() {
                    std::fs::create_dir_all(&grammars_target)?;
                }
                move_dir_contents(&grammars_nested, &grammars_target)?;
            }

            // Move languages if they exist.
            if languages_nested.exists() {
                if !languages_target.exists() {
                    std::fs::create_dir_all(&languages_target)?;
                }
                move_dir_contents(&languages_nested, &languages_target)?;
            }

            // Try to remove the now-empty nested directory.
            let _ = std::fs::remove_dir_all(&nested_path);
        }
        Ok(())
    }
}

/// Helper to move directory contents.
/// Recursively move contents from `src` into `dst`.
///
/// Creates `dst` if necessary and moves files/directories one-by-one.
/// This helper is internal to the runtime and intentionally not exported.
fn move_dir_contents(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            move_dir_contents(&src_path, &dst_path)?;
            // Try to remove the now-empty source directory.
            let _ = std::fs::remove_dir(&src_path);
        } else {
            std::fs::rename(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
