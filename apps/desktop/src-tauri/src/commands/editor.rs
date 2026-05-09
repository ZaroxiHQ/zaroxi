//! Tauri commands that serve the editor front‑end.
//!
//! This file handles **opening, editing, saving, and line‑fetching** of
//! documents.  Syntax highlighting is delegated to the
//! `zaroxi_lang_syntax::cache` module, which owns the per‑document
//! tree‑sitter cache.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::command;
use zaroxi_domain_editor::document_cache::BufferManager;
use zaroxi_domain_editor::FileClass;
use zaroxi_lang_syntax::language::LanguageId;
use zaroxi_lang_syntax::parser::ParserPool;
use zaroxi_lang_syntax::highlight::{HighlightEngine, Highlight};
use zaroxi_lang_syntax::cache;
use zaroxi_theme::theme::SemanticColors;
use zaroxi_theme::colors::Color;

/// Shared buffer manager instance.
static BUFFER_MANAGER: once_cell::sync::Lazy<Arc<BufferManager>> =
    once_cell::sync::Lazy::new(|| Arc::new(BufferManager::new()));

/// Shared parser pool – used for all highlight requests.
static PARSER_POOL: once_cell::sync::Lazy<Arc<ParserPool>> =
    once_cell::sync::Lazy::new(|| Arc::new(ParserPool::new()));

/// Per-document deterministic language resolution cache.
///
/// We persist the resolved language for each document id (normally the file
/// path) at open time and on first highlight_text requests. This prevents
/// transient "PlainText" fallbacks when a request races with a metadata update.
static LANGUAGE_MAP: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashMap<String, LanguageId>>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

// ── OpenDocument response ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDocumentResponse {
    pub document_id: String,
    pub path: String,
    pub line_count: usize,
    pub char_count: usize,
    pub file_class: String,
    pub is_read_only: bool,
    pub content: String,
    pub content_truncated: bool,
    pub version: u64,
    /// Optional detected language identifier (e.g. "rust", "toml").
    /// Present when the backend can infer a language from the path.
    pub language: Option<String>,
    /// Optional compact highlight snapshot to use for the first paint.
    /// When present the frontend MUST render syntax from this snapshot
    /// immediately to avoid a visible second-phase highlight pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_highlight: Option<HighlightResponse>,
}

const TRUNCATE_CHARS: usize = 50_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleLinesRequest {
    pub document_id: String,
    pub start_line: usize,
    pub count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleLinesResponse {
    pub lines: Vec<LineDto>,
    pub total_lines: usize,
}

#[derive(Debug, Serialize)]
pub struct LineDto {
    pub index: usize,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditRequest {
    pub document_id: String,
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_text: String,
}

/// Request to save a full file contents.  The frontend uses this to atomically
/// write the provided content to disk (and update any in‑memory cache).
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFileRequest {
    pub path: String,
    pub content: String,
}

// ── Highlight request / response DTOs ─────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighlightRequest {
    pub document_id: String,
    pub start_line: usize,
    pub count: usize,
    /// Optional theme name: "dark" or "light".  If omitted, dark is used.
    pub theme: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighlightTextRequest {
    /// Document identifier (path or document id)
    pub document_id: String,
    /// Full text to highlight (UTF-8)
    pub text: String,
    /// Optional theme name: "dark" or "light".  If omitted, dark is used.
    pub theme: Option<String>,
    /// Optional language hint (e.g. "rust", "toml", "js") provided by the frontend.
    /// When present, the backend will prefer this to infer the Tree-sitter language.
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighlightResponse {
    /// Document id the highlights correspond to (echoed for safety).
    pub document_id: String,
    /// Resolved language id used to compute these spans (echoed for safety).
    pub language: String,
    pub lines: Vec<HighlightedLine>,
    pub version: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HighlightedLine {
    pub index: usize,
    pub text: String,
    pub spans: Vec<HighlightSpanDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HighlightSpanDto {
    pub start: usize,
    pub end: usize,
    pub token_type: String,
    /// Colour hex string (e.g. "#FF6B6B"). Absent when theme information is unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

// ── open_document ─────────────────────────────────────────────────

#[command]
pub async fn open_document(path: String) -> Result<OpenDocumentResponse, String> {
    let path_buf = std::path::PathBuf::from(&path);
    // Determine language early so we can consult the syntax cache during open.
    let lang = LanguageId::from_path(path_buf.as_path());
    let cached_arc = BUFFER_MANAGER
        .open_document(&path_buf, &zaroxi_ops_file::FileLoader)
        .await
        .map_err(|e| format!("Failed to open document: {}", e))?;

    let guard = cached_arc.lock();
    let document = &guard.document;
    let file_class = document.file_class();
    let content_truncated = file_class == FileClass::Large;
    let is_read_only = content_truncated;

    let content: String = if content_truncated {
        document.text().chars().take(TRUNCATE_CHARS).collect()
    } else {
        document.text()
    };

    let (line_count, char_count) = if content_truncated {
        let preview_lines = content.lines().count();
        (preview_lines, content.len())
    } else {
        (document.len_lines(), document.len_chars())
    };

    // Attempt to seed a compact initial highlight snapshot from the server-side cache.
    // We only use the cached spans (no expensive re-computation) to ensure the open path
    // can present syntax immediately when available. If no cached spans exist we return None
    // and let the frontend fall back to a cheap local rendering while an async highlight
    // may run later.
    let initial_highlight = (|| -> Result<Option<HighlightResponse>, String> {
        if let Some(cached_spans) = cache::get_cached(&path_buf, document.version(), lang) {
            use std::borrow::Cow;

            let line_count = content.lines().count();
            let mut line_offsets = Vec::with_capacity(line_count + 1);
            line_offsets.push(0usize);
            for (pos, b) in content.bytes().enumerate() {
                if b == b'\n' {
                    line_offsets.push(pos + 1);
                }
            }

            let mut response_lines = Vec::with_capacity(line_count);

            for idx in 0..line_count {
                let line_start = *line_offsets.get(idx).unwrap_or(&content.len());
                let line_end = *line_offsets.get(idx + 1).unwrap_or(&content.len());

                let raw = if line_start <= content.len() && line_end <= content.len() {
                    &content[line_start..line_end]
                } else {
                    ""
                };

                let display = if raw.ends_with('\n') {
                    Cow::Owned(raw[..raw.len() - 1].to_owned())
                } else {
                    Cow::Borrowed(raw)
                };

                // Convert byte offsets to character offsets using the loaded document.
                let line_start_char = document.byte_to_char(line_start);
                let line_end_char = document.byte_to_char(line_end);
                let line_len_chars = line_end_char.saturating_sub(line_start_char);

                let mut line_spans: Vec<HighlightSpanDto> = Vec::new();
                for sp in &cached_spans {
                    if sp.end <= line_start || sp.start >= line_end {
                        continue;
                    }

                    let span_start_char = document.byte_to_char(sp.start);
                    let span_end_char = document.byte_to_char(sp.end);

                    let rel_start = span_start_char.saturating_sub(line_start_char);
                    let rel_end = span_end_char
                        .saturating_sub(line_start_char)
                        .min(line_len_chars);

                    let token_type = highlight_tag_to_string(sp.highlight);
                    // Omit color in the compact snapshot; frontend can map token types to theme.
                    line_spans.push(HighlightSpanDto {
                        start: rel_start,
                        end: rel_end,
                        token_type,
                        color: None,
                    });
                }
                line_spans.sort_by_key(|s| s.start);
                response_lines.push(HighlightedLine {
                    index: idx,
                    text: display.into_owned(),
                    spans: line_spans,
                });
            }

            Ok(Some(HighlightResponse {
                document_id: path.clone(),
                language: lang.as_str().to_string(),
                lines: response_lines,
                version: document.version(),
            }))
        } else {
            Ok(None)
        }
    })()
    .map_err(|e| format!("Failed to build initial highlight: {}", e))?;

    Ok(OpenDocumentResponse {
        document_id: path.clone(),
        path,
        line_count,
        char_count,
        file_class: format!("{:?}", file_class),
        is_read_only,
        content,
        content_truncated,
        version: document.version(),
        // Detect language from the requested path so the frontend can request
        // highlighting using an explicit language hint when available.
        language: Some(LanguageId::from_path(path_buf.as_path()).as_str().to_string()),
        initial_highlight,
    })
}

// ── get_visible_lines ─────────────────────────────────────────────

#[command]
pub async fn get_visible_lines(
    request: VisibleLinesRequest,
) -> Result<VisibleLinesResponse, String> {
    let path = std::path::PathBuf::from(&request.document_id);
    let cached_arc = BUFFER_MANAGER
        .get_cached(&path)
        .await
        .ok_or_else(|| "Document not found in cache".to_string())?;
    let guard = cached_arc.lock();
    let document = &guard.document;
    // Record the document version early so early-return paths can include it
    // in the HighlightResponse (prevents constructing responses missing `version`).
    let _version = document.version();
    let total_lines = document.len_lines();
    let mut lines = Vec::new();
    let start_line = request.start_line.min(total_lines);
    let end_line = (start_line + request.count).min(total_lines);

    for line_idx in start_line..end_line {
        if let Some(text) = document.line(line_idx) {
            lines.push(LineDto {
                index: line_idx,
                text,
            });
        }
    }
    Ok(VisibleLinesResponse { lines, total_lines })
}

// ── apply_edit ────────────────────────────────────────────────────

#[command]
pub async fn apply_edit(request: EditRequest) -> Result<(), String> {
    let path = std::path::PathBuf::from(&request.document_id);
    let cached_arc = BUFFER_MANAGER
        .get_cached(&path)
        .await
        .ok_or_else(|| "Document not found in cache".to_string())?;

    {
        let mut guard = cached_arc.lock();
        let document = &mut guard.document;
        if document.file_class().is_read_only() {
            return Err("Document is read‑only (very large file)".to_string());
        }

        let start_char = document.byte_to_char(request.start_byte);
        let old_end_char = document.byte_to_char(request.old_end_byte);
        let (delete_start, delete_end) = if start_char <= old_end_char {
            (start_char, old_end_char)
        } else {
            (old_end_char, start_char)
        };

        if delete_start < delete_end {
            document.delete(delete_start, delete_end)?;
        }
        if !request.new_text.is_empty() {
            document.insert(delete_start, &request.new_text)?;
        }
    }

    // Invalidate syntax cache (version changed)
    cache::invalidate(&path);

    BUFFER_MANAGER.mark_dirty(&path).await;
    Ok(())
}

// ── save_document ─────────────────────────────────────────────────

#[command]
pub async fn save_document(document_id: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&document_id);
    let cached_arc = BUFFER_MANAGER
        .get_cached(&path)
        .await
        .ok_or_else(|| "Document not found in cache".to_string())?;
    {
        let guard = cached_arc.lock();
        let text = guard.document.text();
        std::fs::write(&path, &text).map_err(|e| format!("Failed to save file: {}", e))?;
    }
    BUFFER_MANAGER.mark_clean(&path).await;
    Ok(())
}

// ── line‑count / content ──────────────────────────────────────────

#[command]
pub async fn get_line_count(document_id: String) -> Result<usize, String> {
    let path = std::path::PathBuf::from(&document_id);
    let cached_arc = BUFFER_MANAGER
        .get_cached(&path)
        .await
        .ok_or_else(|| "Document not found in cache".to_string())?;
    let guard = cached_arc.lock();
    Ok(guard.document.len_lines())
}

#[allow(dead_code)]
#[command]
pub async fn get_document_content(document_id: String) -> Result<String, String> {
    let path = std::path::PathBuf::from(&document_id);
    let cached_arc = BUFFER_MANAGER
        .get_cached(&path)
        .await
        .ok_or_else(|| "Document not found in cache".to_string())?;
    let guard = cached_arc.lock();
    Ok(guard.document.text())
}

// ── highlight_document ────────────────────────────────────────────
// Accepts a single `HighlightRequest` struct, just like `get_visible_lines`.

#[command]
pub async fn highlight_document(
    request: HighlightRequest,
) -> Result<HighlightResponse, String> {
    eprintln!("[highlight_document] request: {:?}", request);

    let path = std::path::PathBuf::from(&request.document_id);
    let cached_arc = BUFFER_MANAGER
        .get_cached(&path)
        .await
        .ok_or_else(|| "Document not found in cache".to_string())?;
    let guard = cached_arc.lock();
    let document = &guard.document;
    let version = document.version();

    eprintln!("[highlight_document] document file_class={:?}", document.file_class());

    if document.file_class() == FileClass::Large {
        eprintln!("[highlight_document] document is Large -> returning empty");
        return Ok(HighlightResponse {
            document_id: request.document_id.clone(),
            language: LanguageId::PlainText.as_str().to_string(),
            lines: vec![],
            version,
        });
    }

    // Resolve language deterministically: prefer a cached per-document value
    // (populated at open_document or on first highlight_text request). This
    // prevents transient PlainText fallbacks when metadata is missing briefly.
    let lang = {
        let key = request.document_id.clone();
        let mut map = LANGUAGE_MAP.lock().unwrap();
        if let Some(cached) = map.get(&key) {
            eprintln!("[highlight_document] using cached language for {}: {:?}", key, cached);
            cached.clone()
        } else {
            let resolved = LanguageId::from_path(document.path().unwrap_or(std::path::Path::new("")));
            eprintln!("[highlight_document] resolved language for {}: {:?}", key, resolved);
            map.insert(key.clone(), resolved.clone());
            resolved
        }
    };
    eprintln!("[highlight_document] detected language: {:?}", lang);

    if lang == LanguageId::PlainText {
        eprintln!("[highlight_document] PlainText -> returning empty");
        return Ok(HighlightResponse {
            document_id: request.document_id.clone(),
            language: lang.as_str().to_string(),
            lines: vec![],
            version,
        });
    }

    let full_text = document.text();
    eprintln!("[highlight_document] document version={}, text len={}", version, full_text.len());

    let engine = HighlightEngine::new();

    // ── Resolve theme colours ────────────────────────────────────
    let theme_colors = match request.theme.as_deref() {
        Some("light") => SemanticColors::light(),
        _ => SemanticColors::dark(),
    };

    eprintln!("[highlight_document] checking local cache for version={}", version);
    let spans = if let Some(s) = cache::get_cached(&path, version, lang) {
        eprintln!("[highlight_document] cache hit for version={}", version);
        s
    } else {
        eprintln!("[highlight_document] cache miss - computing spans...");
        cache::get_or_compute(
            &path,
            version,
            &full_text,
            lang,
            PARSER_POOL.clone(),
            &engine,
        )
        .map_err(|e| format!("Highlight error: {}", e))?
    };

    eprintln!("[highlight_document] got {} total spans", spans.len());

    // ── Map spans to requested line range ──
    use std::borrow::Cow;
    let line_count = full_text.lines().count();
    let end_line = request.start_line.saturating_add(request.count).min(line_count);
    // Prevent underflow when start_line > end_line (e.g. scrolling past EOF)
    let desired_capacity = end_line.saturating_sub(request.start_line);
    let mut response_lines = Vec::with_capacity(desired_capacity);

    let mut line_offsets = Vec::with_capacity(line_count + 1);
    line_offsets.push(0usize);
    for (pos, b) in full_text.bytes().enumerate() {
        if b == b'\n' {
            line_offsets.push(pos + 1);
        }
    }

    for idx in request.start_line..end_line {
        let line_start = *line_offsets.get(idx).unwrap_or(&full_text.len());
        let line_end = *line_offsets.get(idx + 1).unwrap_or(&full_text.len());

        // Guard against degenerate offsets.
        // Keep the byte length calculation but prefix with an underscore to avoid
        // "unused variable" warnings while preserving the original intent.
        let _line_len_bytes = line_end.saturating_sub(line_start);
        let raw = if line_start <= full_text.len() && line_end <= full_text.len() {
            &full_text[line_start..line_end]
        } else {
            ""
        };

        let display = if raw.ends_with('\n') {
            Cow::Owned(raw[..raw.len() - 1].to_owned())
        } else {
            Cow::Borrowed(raw)
        };

        // Convert the line byte-range into character offsets so the frontend receives
        // character indices (not raw byte offsets). This keeps the highlight overlay
        // aligned with the editor's text slicing logic.
        let line_start_char = document.byte_to_char(line_start);
        let line_end_char = document.byte_to_char(line_end);
        let line_len_chars = line_end_char.saturating_sub(line_start_char);

        let mut line_spans: Vec<HighlightSpanDto> = Vec::new();
        for sp in &spans {
            if sp.end <= line_start || sp.start >= line_end {
                continue;
            }

            // Convert span byte offsets to character offsets, then make them relative
            // to the start of the current line. Clamp to the visible character length
            // of the line to avoid out-of-bounds indices.
            let span_start_char = document.byte_to_char(sp.start);
            let span_end_char = document.byte_to_char(sp.end);

            let rel_start = span_start_char.saturating_sub(line_start_char);
            let rel_end = span_end_char
                .saturating_sub(line_start_char)
                .min(line_len_chars);

            let token_type = highlight_tag_to_string(sp.highlight);
            let color = tag_to_color(sp.highlight, &theme_colors).map(color_to_hex);
            line_spans.push(HighlightSpanDto {
                start: rel_start,
                end: rel_end,
                token_type,
                color,
            });
        }
        line_spans.sort_by_key(|s| s.start);
        response_lines.push(HighlightedLine {
            index: idx,
            text: display.into_owned(),
            spans: line_spans,
        });
    }

    eprintln!("[highlight_document] returning {} lines", response_lines.len());
    Ok(HighlightResponse {
        document_id: request.document_id.clone(),
        language: lang.as_str().to_string(),
        lines: response_lines,
        version,
    })
}

/// Highlight arbitrary text supplied by the frontend and return the same
/// DTO as `highlight_document`.
///
/// This command is used by the editor to request highlighting for the exact
/// in-memory text the user is editing (single source of truth).  It computes
/// a stable hash of the text and passes it to the cache helper so that the
/// highlighting pipeline can reuse work for identical inputs while never
/// using highlights computed for a different text.
///
/// Important: this does NOT mutate the server-side buffer cache and is safe
/// to call on every edited frame (the frontend will debounce).
#[command]
pub async fn highlight_text(
    request: HighlightTextRequest,
) -> Result<HighlightResponse, String> {
    eprintln!("[highlight_text] request for document_id={}", request.document_id);

    let path = std::path::PathBuf::from(&request.document_id);
    let full_text = request.text;
    // Compute a stable version/hash for this exact text up-front so early-return
    // paths and the rest of the function can refer to the same `version`.
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    full_text.hash(&mut hasher);
    let version = hasher.finish();

    let theme_colors = match request.theme.as_deref() {
        Some("light") => SemanticColors::light(),
        _ => SemanticColors::dark(),
    };

    // Resolve language deterministically for this document id.
    // Prefer a previously cached per-document resolution (set at open_document
    // time), then an explicit frontend hint, then derive from the path.
    // Persist the final decision in LANGUAGE_MAP so future requests are stable.
    let mut lang = {
        let key = request.document_id.clone();
        let mut map = LANGUAGE_MAP.lock().unwrap();
        if let Some(cached) = map.get(&key) {
            eprintln!("[highlight_text] using cached language for {}: {:?}", key, cached);
            cached.clone()
        } else {
            // prefer explicit hint
            let resolved = if let Some(lang_hint) = request.language.as_deref() {
                let fake = std::path::PathBuf::from(format!("file.{}", lang_hint));
                LanguageId::from_path(fake.as_path())
            } else {
                LanguageId::from_path(path.as_path())
            };
            eprintln!("[highlight_text] initial resolved language for {}: {:?}", key, resolved);
            map.insert(key.clone(), resolved.clone());
            resolved
        }
    };

    // If we still have PlainText, attempt lightweight content heuristics (shebangs, markers).
    if lang == LanguageId::PlainText {
        let txt = full_text.as_str();

        // Shebang detection
        if txt.starts_with("#!") {
            let ext_hint = if txt.contains("bash") || txt.contains("sh") {
                "sh"
            } else if txt.contains("python") {
                "py"
            } else if txt.contains("node") || txt.contains("nodejs") {
                "js"
            } else {
                ""
            };
            if !ext_hint.is_empty() {
                let fake = std::path::PathBuf::from(format!("file.{}", ext_hint));
                lang = LanguageId::from_path(fake.as_path());
            }
        }

        // TOML detection
        if lang == LanguageId::PlainText {
            if txt.contains("[package]") || (txt.contains("=") && txt.contains("version")) {
                let fake = std::path::PathBuf::from("file.toml");
                lang = LanguageId::from_path(fake.as_path());
            }
        }

        // Rust heuristics
        if lang == LanguageId::PlainText {
            if txt.contains("fn ") || (txt.contains("pub ") && txt.contains("crate")) {
                let fake = std::path::PathBuf::from("file.rs");
                lang = LanguageId::from_path(fake.as_path());
            }
        }

        // Markdown heuristics
        if lang == LanguageId::PlainText {
            if txt.contains("\n# ") || txt.starts_with("# ") || txt.contains("```") {
                let fake = std::path::PathBuf::from("file.md");
                lang = LanguageId::from_path(fake.as_path());
            }
        }

        // Persist any corrected resolution so future requests don't flip to PlainText.
        let key = request.document_id.clone();
        LANGUAGE_MAP.lock().unwrap().insert(key, lang.clone());
    }

    // If still PlainText after heuristics, give a clear log and return empty spans.
    if lang == LanguageId::PlainText {
        eprintln!("[highlight_text] PlainText -> returning empty for {}", request.document_id);
        return Ok(HighlightResponse {
            document_id: request.document_id.clone(),
            language: lang.as_str().to_string(),
            lines: vec![],
            version,
        });
    }

    let engine = HighlightEngine::new();

    // version hash already computed above

    eprintln!("[highlight_text] checking local cache for version={}", version);

    let spans = if let Some(s) = cache::get_cached(&path, version, lang) {
        eprintln!("[highlight_text] cache hit for version={}", version);
        s
    } else {
        eprintln!("[highlight_text] cache miss - computing spans...");
        cache::get_or_compute(
            &path,
            version,
            &full_text,
            lang,
            PARSER_POOL.clone(),
            &engine,
        )
        .map_err(|e| format!("Highlight error: {}", e))?
    };

    eprintln!("[highlight_text] got {} total spans", spans.len());

    // Map spans into requested line range covering the whole document text.
    use std::borrow::Cow;
    let line_count = full_text.lines().count();

    let mut line_offsets = Vec::with_capacity(line_count + 1);
    line_offsets.push(0usize);
    for (pos, b) in full_text.bytes().enumerate() {
        if b == b'\n' {
            line_offsets.push(pos + 1);
        }
    }

    let mut response_lines = Vec::with_capacity(line_count);

    for idx in 0..line_count {
        let line_start = *line_offsets.get(idx).unwrap_or(&full_text.len());
        let line_end = *line_offsets.get(idx + 1).unwrap_or(&full_text.len());

        // Guard against degenerate offsets.
        let raw = if line_start <= full_text.len() && line_end <= full_text.len() {
            &full_text[line_start..line_end]
        } else {
            ""
        };

        let display = if raw.ends_with('\n') {
            Cow::Owned(raw[..raw.len() - 1].to_owned())
        } else {
            Cow::Borrowed(raw)
        };

        // Convert byte offsets to character offsets by counting chars up to the byte position.
        let line_start_char = full_text[..line_start].chars().count();
        let line_end_char = full_text[..line_end].chars().count();
        let line_len_chars = line_end_char.saturating_sub(line_start_char);

        let mut line_spans: Vec<HighlightSpanDto> = Vec::new();
        for sp in &spans {
            if sp.end <= line_start || sp.start >= line_end {
                continue;
            }

            // Convert span byte offsets to character offsets.
            let span_start_char = full_text[..sp.start.min(full_text.len())].chars().count();
            let span_end_char = full_text[..sp.end.min(full_text.len())].chars().count();

            let rel_start = span_start_char.saturating_sub(line_start_char);
            let rel_end = span_end_char
                .saturating_sub(line_start_char)
                .min(line_len_chars);

            let token_type = highlight_tag_to_string(sp.highlight);
            let color = tag_to_color(sp.highlight, &theme_colors).map(color_to_hex);
            line_spans.push(HighlightSpanDto {
                start: rel_start,
                end: rel_end,
                token_type,
                color,
            });
        }
        line_spans.sort_by_key(|s| s.start);
        response_lines.push(HighlightedLine {
            index: idx,
            text: display.into_owned(),
            spans: line_spans,
        });
    }

    eprintln!("[highlight_text] returning {} lines", response_lines.len());
    Ok(HighlightResponse {
        document_id: request.document_id.clone(),
        language: lang.as_str().to_string(),
        lines: response_lines,
        version,
    })
}

/// Save a file by writing the provided content to disk.
///
/// The frontend calls this command when it has the authoritative current text
/// and wishes to persist it.  The command also attempts to update the server-side
/// cached document (if present) so subsequent read/save operations remain coherent.
#[allow(dead_code)]
#[command]
pub async fn save_file(request: SaveFileRequest) -> Result<(), String> {
    eprintln!("[save_file] writing path={}", request.path);

    let path = std::path::PathBuf::from(&request.path);

    // Write to disk first.  This mirrors typical editor semantics where the
    // frontend sends final content for atomic overwrite.
    std::fs::write(&path, &request.content)
        .map_err(|e| format!("Failed to write file {}: {}", request.path, e))?;

    // If the BufferManager has a cached document for this path, update it so
    // backend-side highlights / saves remain consistent.
    if let Some(cached_arc) = BUFFER_MANAGER.get_cached(&path).await {
        // Phase 1: Mutate document under lock and then drop the lock to avoid
        // holding simultaneous mutable borrows of different fields of the guard.
        {
            let mut guard = cached_arc.lock();
            // Replace whole document contents: delete existing chars, insert new content.
            let len_chars = guard.document.len_chars();
            if let Err(e) = guard.document.delete_range(0, len_chars) {
                eprintln!("[save_file] failed to clear existing cached document: {}", e);
            }
            if let Err(e) = guard.document.insert(0, &request.content) {
                eprintln!("[save_file] failed to insert new content into cached document: {}", e);
            }
            // Drop guard at end of this scope so we can re-lock to update metadata.
        }

        // Phase 2: Update metadata under a fresh lock to avoid overlapping mutable borrows.
        if let Ok(meta) = std::fs::metadata(&path) {
            let mut guard = cached_arc.lock();
            guard.meta.file_size = meta.len();
            // Compute modification time in seconds since UNIX_EPOCH without relying
            // on a helper function present elsewhere.
            guard.meta.mtime_secs = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Read version while holding the lock (safe) and assign to metadata.
            guard.meta.version = guard.document.version();
            guard.meta.is_dirty = false;
        }
    }

    Ok(())
}

// ── stub ──────────────────────────────────────────────────────────

#[command]
pub async fn get_styled_spans() -> Result<(), String> {
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────

fn highlight_tag_to_string(tag: Highlight) -> String {
    match tag {
        Highlight::Keyword => "keyword".to_string(),
        Highlight::String => "string".to_string(),
        Highlight::Comment => "comment".to_string(),
        Highlight::Function => "function".to_string(),
        Highlight::Type => "type".to_string(),
        Highlight::Variable => "variable".to_string(),
        Highlight::Constant => "constant".to_string(),
        Highlight::Number => "number".to_string(),
        Highlight::Operator => "operator".to_string(),
        other => format!("{:?}", other),
    }
}

/// Convert a semantic `Highlight` into the corresponding theme colour.
fn tag_to_color(tag: Highlight, colors: &SemanticColors) -> Option<Color> {
    // Plain text should not be assigned a semantic color. Returning `None`
    // indicates the frontend should use the editor's default foreground color.
    if tag == Highlight::Plain {
        return None;
    }

    use zaroxi_lang_syntax::theme_map::SemanticTokenType;
    let token_type = SemanticTokenType::from_highlight(tag);
    Some(token_type.theme_color(colors))
}

/// Convert a `Color` to a hex string suitable for CSS.
fn color_to_hex(c: Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}
