use crate::Document;

/// Small domain invariants and validators for buffers.
///
/// Keep rules tiny and focused for Phase 3. These are pure functions with
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
