use crate::Document;

/// Small domain invariants and validators for buffers.
///
/// Keep rules tiny and focused for Phase 4. These are pure functions with
/// deterministic behavior and no IO, owned by the domain crate.

/// Validate a document according to minimal domain policies:
/// - display_name must be non-empty
/// - text length must be <= 1_000_000 characters (defensive guard)
pub fn validate_document(doc: &Document) -> Result<(), String> {
    if doc.display_name.trim().is_empty() {
        return Err("display_name must not be empty".to_string());
    }

    if doc.text.chars().count() > 1_000_000 {
        return Err("text exceeds maximum allowed length".to_string());
    }

    Ok(())
}

/// Validate a raw content snapshot for buffer mutation.
/// - content must be non-empty (for Phase 4)
/// - content length must be reasonable
pub fn validate_content(content: &str) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err("content must not be empty".to_string());
    }
    if content.chars().count() > 1_000_000 {
        return Err("content exceeds maximum allowed length".to_string());
    }
    Ok(())
}

/// Validate buffer identifier shape for the simple slice.
/// We expect buffer ids produced by core/infra to follow `buf:<path>` convention.
pub fn validate_buffer_id(id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        return Err("buffer id must not be empty".to_string());
    }
    if !id.starts_with("buf:") {
        return Err("buffer id must start with 'buf:'".to_string());
    }
    // require at least one character after the prefix
    if id.len() <= 4 {
        return Err("buffer id missing identifier after 'buf:'".to_string());
    }
    Ok(())
}
