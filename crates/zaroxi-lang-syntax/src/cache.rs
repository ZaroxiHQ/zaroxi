//! Global document-version‑aware highlight cache.
//!
//! This module stores a parsed `SyntaxTree` and the resulting
//! `HighlightSpan`s for each open document, keyed by file path and
//! document version.  It is the single source of truth for the syntax
//! layer and is consumed directly by the Tauri editor commands.
//!
//! Only document content changes (edits) cause cache invalidation;
//! scrolling or viewport changes never repopulate the cache.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;

use crate::highlight::{HighlightEngine, HighlightSpan};
use crate::language::LanguageId;
use crate::parser::{ParserPool, SyntaxTree};

// ── Global cache ──────────────────────────────────────────────────

static CACHE: LazyLock<Mutex<HashMap<PathBuf, DocumentSyntaxState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ── Per‑document state ────────────────────────────────────────────

#[allow(dead_code)]
struct DocumentSyntaxState {
    /// The document version for which `spans` is valid.
    version: u64,
    /// Parsed syntax tree, or `None` for plain‑text.
    syntax: Option<SyntaxTree>,
    /// Highlight spans covering the whole document.
    spans: Vec<HighlightSpan>,
    /// Language of the document.
    language: LanguageId,
    /// Parser pool shared with all requesters.
    pool: Arc<ParserPool>,
}

// ── Public API ────────────────────────────────────────────────────

/// Ensure a syntax tree and highlight spans exist for the given
/// document, returning the spans.
///
/// * If the cached version matches `version`, the stored spans are
///   returned immediately (no work).
/// * Otherwise the text is re‑parsed and re‑highlighted.
pub fn get_or_compute(
    path: &PathBuf,
    version: u64,
    text: &str,
    language: LanguageId,
    pool: Arc<ParserPool>,
    engine: &HighlightEngine,
) -> Result<Vec<HighlightSpan>, String> {
    let mut guard = CACHE.lock();

    // Fast‑path: existing state with same version → return cached spans.
    if let Some(state) = guard.get_mut(path) {
        if state.version == version && state.language == language {
            return Ok(state.spans.clone());
        }
    }

    // ── Compute fresh ──────────────────────────────────────────────
    let syntax = if language == LanguageId::PlainText {
        None
    } else {
        match SyntaxTree::new(pool.clone(), text, language) {
            Ok(tree) => Some(tree),
            Err(e) => {
                eprintln!("[syntax-cache] parse error: {e}");
                None
            }
        }
    };

    let spans = if let Some(ref tree) = syntax {
        match engine.highlight(language, text, tree.tree()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[syntax-cache] highlight error: {e}");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let state = DocumentSyntaxState {
        version,
        syntax,
        spans: spans.clone(),
        language,
        pool,
    };

    guard.insert(path.clone(), state);
    Ok(spans)
}

/// Invalidate the cache entry for `path` (usually after an edit).
pub fn invalidate(path: &PathBuf) {
    CACHE.lock().remove(path);
}

/// Clear all cached entries.
pub fn clear() {
    CACHE.lock().clear();
}

/// Number of documents currently cached.  Mostly useful for
/// diagnostics.
pub fn len() -> usize {
    CACHE.lock().len()
}
