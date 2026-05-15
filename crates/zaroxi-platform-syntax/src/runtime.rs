//! Runtime path resolution for Tree-sitter grammars and queries.

use std::env;
use std::path::PathBuf;

/// Runtime environment for locating Tree-sitter assets.
#[derive(Debug, Clone)]
pub struct Runtime {
    /// Root directory of the Tree-sitter runtime (e.g., .../runtime/treesitter).
    root: PathBuf,
}

impl Runtime {
    /// Attempt to locate the runtime directory.
    ///
    /// Searches in the following order:
    /// 1. `QYZER_STUDIO_RUNTIME` environment variable (for compatibility)
    /// 2. `NEOTE_RUNTIME` environment variable.
    /// 3. A directory `runtime/treesitter` sibling to the current executable.
    /// 4. The current working directory `./runtime/treesitter`.
    /// 5. Bundled resources directory for packaged applications.
    ///
    /// Returns a `Runtime` even if the directory does not exist; operations will
    /// fail later with appropriate errors.
    pub fn new() -> Self {
        let root = Self::locate_root().unwrap_or_else(|| {
            // Fallback to a placeholder path; errors will be reported when trying to load.
            PathBuf::from("./runtime/treesitter")
        });
        let runtime = Self { root };

        // Try to fix nested structure if it exists
        let _ = runtime.fix_nested_structure();

        runtime
    }

    fn locate_root() -> Option<PathBuf> {
        // 0. First priority: runtime directory relative to the zaroxi-lang-syntax crate source directory
        // This is the most reliable location for development
        // We need to find the crate's Cargo.toml, not the desktop app's
        // Try to find the crate by looking for its Cargo.toml relative to the current executable
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Walk up from executable to find the workspace root
                let mut current = exe_dir.to_path_buf();
                while current.parent().is_some() {
                    let cargo_toml = current.join("Cargo.toml");
                    if cargo_toml.exists() {
                        // Found workspace root, look for crates/zaroxi-lang-syntax/runtime/treesitter
                        let candidate = current.join("crates/zaroxi-lang-syntax/runtime/treesitter");
                        if candidate.is_dir() {
                            return Some(candidate);
                        }
                        let runtime_dir = current.join("crates/zaroxi-lang-syntax/runtime");
                        if runtime_dir.is_dir() {
                            let ts_dir = runtime_dir.join("treesitter");
                            if ts_dir.is_dir() {
                                return Some(ts_dir);
                            }
                            return Some(runtime_dir);
                        }
                        break;
                    }
                    current = current.parent().unwrap().to_path_buf();
                }
            }
        }

        // 1. ZAROXI_STUDIO_RUNTIME environment variable (new)
        if let Ok(env_path) = env::var("ZAROXI_STUDIO_RUNTIME") {
            let p = PathBuf::from(env_path);
            if p.is_dir() {
                return Some(p);
            }
        }

        // 2. QYZER_STUDIO_RUNTIME environment variable (for backward compatibility)
        if let Ok(env_path) = env::var("QYZER_STUDIO_RUNTIME") {
            let p = PathBuf::from(env_path);
            if p.is_dir() {
                return Some(p);
            }
        }

        // 3. NEOTE_RUNTIME environment variable.
        if let Ok(env_path) = env::var("NEOTE_RUNTIME") {
            let p = PathBuf::from(env_path);
            if p.is_dir() {
                return Some(p);
            }
        }

        // 4. Check for the correct structure: look for runtime/treesitter directory
        // First, try current working directory
        if let Ok(cwd) = env::current_dir() {
            // Check for runtime/treesitter directly
            let candidate = cwd.join("runtime/treesitter");
            if candidate.is_dir() {
                return Some(candidate);
            }

            // Check for nested runtime/treesitter/runtime/treesitter (incorrect structure)
            let nested_candidate = candidate.join("runtime/treesitter");
            if nested_candidate.is_dir() {
                return Some(candidate);
            }

            // Try to find the runtime directory by walking up
            let mut current = cwd.clone();
            while current.parent().is_some() {
                let candidate = current.join("runtime/treesitter");
                if candidate.is_dir() {
                    return Some(candidate);
                }
                current = current.parent().unwrap().to_path_buf();
            }
        }

        // 5. Sibling to executable (development mode)
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // Try development layout: ../runtime/treesitter
                let candidate = exe_dir.join("../runtime/treesitter");
                if candidate.is_dir() {
                    return Some(candidate);
                }

                // Try walking up from executable
                let mut current = exe_dir.to_path_buf();
                while current.parent().is_some() {
                    let candidate = current.join("runtime/treesitter");
                    if candidate.is_dir() {
                        return Some(candidate);
                    }
                    current = current.parent().unwrap().to_path_buf();
                }
            }
        }

        // 6. Fallback: look for runtime directory relative to the project root
        // This handles the case where the runtime directory is at the workspace root
        if let Ok(cwd) = env::current_dir() {
            // Try to find the project root by looking for Cargo.toml
            let mut current = cwd.clone();
            while current.parent().is_some() {
                let cargo_toml = current.join("Cargo.toml");
                if cargo_toml.exists() {
                    // Found project root, look for runtime/treesitter relative to it
                    let candidate = current.join("runtime/treesitter");
                    if candidate.is_dir() {
                        return Some(candidate);
                    }
                    // Also try just "runtime" directory
                    let runtime_dir = current.join("runtime");
                    if runtime_dir.is_dir() {
                        // Check if runtime contains treesitter subdirectory
                        let ts_dir = runtime_dir.join("treesitter");
                        if ts_dir.is_dir() {
                            return Some(ts_dir);
                        }
                        // If runtime exists but no treesitter subdir, use runtime itself
                        return Some(runtime_dir);
                    }
                    break;
                }
                current = current.parent().unwrap().to_path_buf();
            }
        }

        // 7. Last resort: check if runtime directory exists in the current directory
        if let Ok(cwd) = env::current_dir() {
            let runtime_dir = cwd.join("runtime");
            if runtime_dir.is_dir() {
                return Some(runtime_dir);
            }
        }

        // 8. Try to find runtime directory relative to the CARGO_MANIFEST_DIR
        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir);
            // Walk up to find workspace root
            let mut current = manifest_path.clone();
            while current.parent().is_some() {
                let workspace_toml = current.join("Cargo.toml");
                if workspace_toml.exists() {
                    // Found workspace root, look for runtime/treesitter
                    let candidate = current.join("runtime/treesitter");
                    if candidate.is_dir() {
                        return Some(candidate);
                    }
                    let runtime_dir = current.join("runtime");
                    if runtime_dir.is_dir() {
                        let ts_dir = runtime_dir.join("treesitter");
                        if ts_dir.is_dir() {
                            return Some(ts_dir);
                        }
                        return Some(runtime_dir);
                    }
                    break;
                }
                current = current.parent().unwrap().to_path_buf();
            }
        }

        // 9. Try to find runtime directory relative to the crate source directory (fallback)
        // This handles the case where the runtime directory is inside the crate itself
        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            let manifest_path = PathBuf::from(manifest_dir);
            // Check if runtime/treesitter exists directly in the crate directory
            let candidate = manifest_path.join("runtime/treesitter");
            if candidate.is_dir() {
                return Some(candidate);
            }
            // Check if runtime exists directly in the crate directory
            let runtime_dir = manifest_path.join("runtime");
            if runtime_dir.is_dir() {
                let ts_dir = runtime_dir.join("treesitter");
                if ts_dir.is_dir() {
                    return Some(ts_dir);
                }
                return Some(runtime_dir);
            }
        }

        None
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
    /// the old platform‑specific subdirectory (`grammars/<os>-<arch>/`) is used as a fallback
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
        // Some language IDs use underscores but the library uses hyphens
        let lib_name = match language_id {
            "c_sharp" => "c-sharp",
            _ => language_id,
        };
        let lib_name = format!("{}tree-sitter-{}{}", prefix, lib_name, extension);

        // First try the flat grammars directory
        let flat_path = self.root.join("grammars").join(&lib_name);
        if flat_path.exists() {
            return flat_path;
        }

        // Fallback to platform-specific subdirectory
        let target = env::consts::ARCH;
        let os = env::consts::OS;
        let subdir = format!("{}-{}", os, target);
        self.root.join("grammars").join(&subdir).join(&lib_name)
    }

    /// Load a Tree-sitter language from a shared library in the runtime directory.
    ///
    /// This uses `libloading` to dynamically load the grammar library and retrieve
    /// the `tree_sitter_{language}` function.
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

            let get_symbol = |sym: &str| -> Result<Symbol<unsafe extern "C" fn() -> tree_sitter::Language>, String> {
                lib.get(sym.as_bytes())
                    .map_err(|e| format!("Failed to get symbol {}: {}", sym, e))
            };

            let symbol = match get_symbol(&standard_symbol) {
                Ok(s) => Some(s),
                Err(e) => {
                    if language_id == "markdown" {
                        match get_symbol(&markdown_inline_symbol.unwrap()) {
                            Ok(s) => {
                                eprintln!("[runtime] loaded markdown inline symbol instead of standard");
                                Some(s)
                            }
                            Err(_) => return Err(format!("Failed to get symbol {}: {}", standard_symbol, e)),
                        }
                    } else {
                        return Err(e);
                    }
                }
            };

            // Call the function to get the language
            let language_fn = symbol.unwrap();
            let language = language_fn();

            // The library must not be unloaded while the language is in use.
            // We leak the library handle to keep it loaded for the lifetime of the program.
            std::mem::forget(lib);

            Ok(language)
        }
    }

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

    /// Fix nested runtime directory structure if found
    pub fn fix_nested_structure(&self) -> std::io::Result<()> {
        let nested_path = self.root.join("runtime/treesitter");
        if nested_path.is_dir() {
            // Move contents from nested to parent
            let grammars_nested = nested_path.join("grammars");
            let languages_nested = nested_path.join("languages");

            let grammars_target = self.root.join("grammars");
            let languages_target = self.root.join("languages");

            // Move grammars if they exist
            if grammars_nested.exists() {
                if !grammars_target.exists() {
                    std::fs::create_dir_all(&grammars_target)?;
                }
                move_dir_contents(&grammars_nested, &grammars_target)?;
            }

            // Move languages if they exist
            if languages_nested.exists() {
                if !languages_target.exists() {
                    std::fs::create_dir_all(&languages_target)?;
                }
                move_dir_contents(&languages_nested, &languages_target)?;
            }

            // Try to remove the now-empty nested directory
            let _ = std::fs::remove_dir_all(&nested_path);
        }
        Ok(())
    }
}

/// Helper to move directory contents
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
            // Try to remove the now-empty source directory
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
