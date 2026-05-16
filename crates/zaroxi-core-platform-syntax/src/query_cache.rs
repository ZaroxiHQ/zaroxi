//! Cache for compiled Tree-sitter queries loaded from bundled files.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tree_sitter::Query;

use crate::grammar_registry::GrammarRegistry;
use crate::runtime::Runtime;

/// Global cache for compiled queries
static QUERY_CACHE: OnceLock<Mutex<HashMap<String, Result<&'static Query, String>>>> =
    OnceLock::new();

/// Get a compiled query for a language
pub fn get_query(language_id: &str, query_type: &str) -> Option<&'static Query> {
    // For markdown, always reload the query to avoid caching issues with development
    if language_id == "markdown" {
        match load_query_from_file(language_id, query_type) {
            Ok(query) => {
                // Box and leak the query to get a static reference
                let query_ptr = Box::leak(Box::new(query));
                return Some(query_ptr);
            }
            Err(e) => {
                eprintln!("DEBUG: Failed to load markdown query: {}", e);
                return None;
            }
        }
    }

    let cache_key = format!("{}:{}", language_id, query_type);
    let cache = QUERY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    // Check cache first
    {
        let cache_guard = cache.lock().unwrap();
        if let Some(result) = cache_guard.get(&cache_key) {
            return match result {
                Ok(query) => Some(*query),
                Err(_) => None,
            };
        }
    }

    // Not in cache, load from file
    let result = load_query_from_file(language_id, query_type);

    match result {
        Ok(query) => {
            // Box and leak the query to get a static reference
            let query_ptr = Box::leak(Box::new(query));

            // Store in cache
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert(cache_key, Ok(query_ptr));

            Some(query_ptr)
        }
        Err(e) => {
            eprintln!("DEBUG: Query loading failed for {}:{}: {}", language_id, query_type, e);
            // Store error in cache
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert(cache_key, Err(e));
            None
        }
    }
}

fn load_query_from_file(language_id: &str, query_type: &str) -> Result<Query, String> {
    // Get the language
    let language = crate::dynamic_loader::load_language(language_id)
        .ok_or_else(|| format!("Language '{}' not loaded", language_id))?;

    // Get query file path
    let runtime = Runtime::new();
    let query_dir = runtime.language_dir(language_id).join("queries");
    let query_path = query_dir.join(format!("{}.scm", query_type));

    // Read query file
    let query_text = match std::fs::read_to_string(&query_path) {
        Ok(text) => text,
        Err(e) => {
            // If we can't read the query file, return an empty query
            eprintln!("DEBUG: Failed to read query file {}: {}", query_path.display(), e);
            return Ok(Query::new(&language, "")
                .map_err(|e| format!("Empty query compilation failed: {}", e))?);
        }
    };

    // Try to compile query, but if it fails, return an empty query
    match Query::new(&language, &query_text) {
        Ok(query) => Ok(query),
        Err(e) => {
            eprintln!(
                "DEBUG: Query compilation failed for {} ({}): {}",
                language_id, query_type, e
            );
            // Return an empty query instead of failing
            Query::new(&language, "").map_err(|e| format!("Empty query compilation failed: {}", e))
        }
    }
}

/// Preload common queries for all available languages
pub fn preload_queries() {
    let registry = GrammarRegistry::global();

    for language_id in registry.language_ids() {
        // Try to load highlights query
        let _ = get_query(language_id, "highlights");

        // Try to load injections query if it exists
        let _ = get_query(language_id, "injections");
    }
}

/// Query cache struct (for re-export)
pub struct QueryCache;

impl QueryCache {
    /// Get a query
    pub fn get(language_id: &str, query_type: &str) -> Option<&'static Query> {
        get_query(language_id, query_type)
    }

    /// Preload queries
    pub fn preload() {
        preload_queries();
    }
}
