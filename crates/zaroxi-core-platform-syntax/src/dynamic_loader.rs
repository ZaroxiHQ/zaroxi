//! Dynamic loading of Tree-sitter grammars from bundled runtime.
//
// This file provides two builds:
//  - When the "dynamic-loading" feature is enabled we use `libloading` to open
//    runtime grammar libraries. (The heavy implementation lives under the
//    feature guard.)
//  - When the feature is disabled we expose lightweight stubs so the crate can
//    compile without optional native loading support.
//
// This keeps the crate small and allows consumers to opt into dynamic loading.

use tree_sitter;

#[cfg(not(feature = "dynamic-loading"))]
mod no_dynamic {
    use super::tree_sitter;

    /// Stubs when dynamic loading is disabled.
    ///
    /// These functions allow callers to compile against the optional API
    /// without enabling the heavy native-loading feature.
    pub fn load_language(_language_id: &str) -> Option<tree_sitter::Language> {
        // Dynamic loading disabled at compile time.
        None
    }

    /// No-op placeholder used when dynamic loading is disabled.
    pub fn preload_available_grammars() {
        // Nothing to do when dynamic loading is disabled.
    }

    /// Always returns false when dynamic loading is disabled.
    pub fn is_grammar_available(_language_id: &str) -> bool {
        false
    }

    /// Lightweight shim type so callers can refer to `DynamicGrammarLoader`
    /// even when dynamic loading is disabled at compile time.
    pub struct DynamicGrammarLoader;

    impl DynamicGrammarLoader {
        /// Attempt to load a language; always returns None in stub build.
        pub fn load(_language_id: &str) -> Option<tree_sitter::Language> {
            None
        }

        /// Check availability; always false in stub build.
        pub fn is_available(_language_id: &str) -> bool {
            false
        }

        /// Preload all grammars; noop in stub build.
        pub fn preload_all() {
            // noop
        }
    }
}

#[cfg(not(feature = "dynamic-loading"))]
pub use no_dynamic::*;

#[cfg(feature = "dynamic-loading")]
mod enabled {
    //! Enabled implementation that depends on libloading.
    use libloading::Library;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    use tree_sitter;

    use crate::grammar_registry::GrammarRegistry;
    use crate::runtime::Runtime;

    /// Global cache for loaded languages
    static LANGUAGE_CACHE: OnceLock<Mutex<HashMap<String, Option<tree_sitter::Language>>>> =
        OnceLock::new();

    /// Load a Tree-sitter language dynamically from the runtime directory
    pub fn load_language(language_id: &str) -> Option<tree_sitter::Language> {
        let cache = LANGUAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        // Check cache first
        {
            let cache_guard = cache.lock().unwrap();
            if let Some(cached) = cache_guard.get(language_id) {
                return cached.clone();
            }
        }

        // Not in cache, try to load
        let result = load_language_impl(language_id);

        // Store in cache
        let mut cache_guard = cache.lock().unwrap();
        cache_guard.insert(language_id.to_string(), result.clone());

        result
    }

    fn load_language_impl(language_id: &str) -> Option<tree_sitter::Language> {
        // Check if the language is in the registry
        let registry = GrammarRegistry::global();
        if !registry.contains_language(language_id) {
            eprintln!("DEBUG: Language {} not in registry", language_id);
            return None;
        }

        let runtime = Runtime::new();

        // Check if the grammar library exists
        let library_path = runtime.grammar_library_path(language_id);
        eprintln!("DEBUG: load_language_impl: checking path {:?}", library_path);
        if !library_path.exists() {
            eprintln!("DEBUG: load_language_impl: Library path doesn't exist: {}", library_path.display());
            return None;
        }

        eprintln!("DEBUG: load_language_impl: Loading language {} from {}", language_id, library_path.display());

        // Load the library
        unsafe {
            match Library::new(&library_path) {
                Ok(lib) => {
                    // For markdown, try multiple symbol names in order
                    let symbol_names: Vec<String> = if language_id == "markdown" {
                        vec![
                            "tree_sitter_markdown".to_string(), // Try non-inline first
                            "tree_sitter_markdown_inline".to_string(),
                            format!("tree_sitter_{}", language_id),
                        ]
                    } else {
                        vec![format!("tree_sitter_{}", language_id)]
                    };

                    let mut last_error = None;

                    for symbol_name in symbol_names {
                        eprintln!("DEBUG: load_language_impl: Looking for symbol: {}", symbol_name);

                        // Use explicit generic to help type inference across libloading versions.
                        let language_fn = lib.get::<unsafe extern "C" fn() -> tree_sitter::Language>(symbol_name.as_bytes());

                        match language_fn {
                            Ok(func) => {
                                eprintln!("DEBUG: load_language_impl: Found symbol {} for {}", symbol_name, language_id);
                                let language = func();
                                // Leak the library to keep it loaded
                                std::mem::forget(lib);
                                // Print some info about the language
                                eprintln!(
                                    "DEBUG: load_language_impl: Language {} loaded successfully via {}, node count: {}",
                                    language_id,
                                    symbol_name,
                                    language.node_kind_count()
                                );
                                // Print node types for debugging
                                if language_id == "markdown" {
                                    for i in 0..language.node_kind_count() {
                                        let kind = language.node_kind_for_id(i as u16);
                                        if let Some(kind) = kind {
                                            eprintln!("DEBUG: Node type {}: {}", i, kind);
                                        }
                                    }
                                }
                                return Some(language);
                            }
                            Err(e) => {
                                // Store error message string instead of the error itself
                                let error_msg = format!("{}", e);
                                last_error = Some(error_msg);
                                eprintln!("DEBUG: load_language_impl: Failed to get symbol {}: {}", symbol_name, e);
                                // Try next symbol
                            }
                        }
                    }

                    // If we get here, all symbols failed
                    if let Some(e) = last_error {
                        eprintln!("DEBUG: load_language_impl: All symbols failed for {}: {}", language_id, e);
                    }
                    None
                }
                Err(e) => {
                    eprintln!("DEBUG: load_language_impl: Failed to load library {}: {}", library_path.display(), e);
                    None
                }
            }
        }
    }

    /// Preload all available grammars to warm up the cache.
    ///
    /// This performs a best-effort load of every language registered in the
    /// global registry; failures are logged but not returned.
    pub fn preload_available_grammars() {
        let registry = GrammarRegistry::global();
        for language_id in registry.language_ids() {
            // Try to load each language
            load_language(language_id);
        }
    }

    /// Check if a grammar shared library exists in the runtime directory.
    ///
    /// This checks the runtime layout for the platform-specific shared library
    /// corresponding to `language_id`.
    pub fn is_grammar_available(language_id: &str) -> bool {
        let runtime = Runtime::new();
        let library_path = runtime.grammar_library_path(language_id);
        library_path.exists()
    }

    /// High-level dynamic grammar loader facade exported by the crate.
    ///
    /// Consumers can use this stable facade to attempt language loads,
    /// check availability, or warm caches without depending on internal details.
    pub struct DynamicGrammarLoader;

    impl DynamicGrammarLoader {
        /// Load a language by id using the runtime.
        pub fn load(language_id: &str) -> Option<tree_sitter::Language> {
            load_language(language_id)
        }

        /// Return whether the language is available.
        pub fn is_available(language_id: &str) -> bool {
            is_grammar_available(language_id)
        }

        /// Preload all grammars to warm up caches.
        pub fn preload_all() {
            preload_available_grammars();
        }
    }
}

#[cfg(feature = "dynamic-loading")]
pub use enabled::*;
