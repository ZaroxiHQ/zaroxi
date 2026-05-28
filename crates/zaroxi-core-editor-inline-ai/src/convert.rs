#![allow(dead_code)]
// Minimal deterministic conversion helpers for AI proposal text -> editor transaction.
// Phase 11 initial strategy: when provider returns a full-file replacement we produce
// a single Replace transaction replacing the whole buffer content. Future commits may
// implement smarter diffing.

/// Convert provider proposal text into a deterministic text edit that replaces the
/// full buffer contents. Returns (start_char_index, end_char_index, replacement_text).
pub fn full_replace_edit(original_content: &str, proposal_text: &str) -> (usize, usize, String) {
    let start = 0usize;
    // end uses character count for the editor transaction helpers that operate on char indices.
    let end = original_content.chars().count();
    (start, end, proposal_text.to_string())
}
