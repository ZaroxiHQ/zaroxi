//! Diff applier — takes structured `DiffResult`s and produces `TextEdit` operations
//! for the core editor transaction pipeline.
//!
//! Phase 2: bridges AI-generated diffs to the buffer mutation layer.

use zaroxi_core_editor_buffer::ports::TextEdit;
use zaroxi_domain_ai::actions::{DiffChange, DiffResult};

/// Convert a `DiffResult` into a sequence of `TextEdit` operations.
/// Returns `None` if there are no changes to apply.
///
/// Note: each `DiffChange` maps directly to a `TextEdit` variant:
/// - `Insert` → `TextEdit::Insert`
/// - `Delete` → `TextEdit::Delete`
/// - `Replace` → `TextEdit::Replace`
pub fn diff_to_text_edits(diff: &DiffResult) -> Option<Vec<TextEdit>> {
    if !diff.has_changes() {
        return None;
    }

    // Full replacement is one big Replace
    if diff.full_replacement.is_some() {
        // Handled by the caller reading the full text; no individual edits needed
        return None;
    }

    let edits: Vec<TextEdit> = diff
        .changes
        .iter()
        .map(|change| match change {
            DiffChange::Insert { index, text } => {
                TextEdit::Insert { index: *index, text: text.clone() }
            }
            DiffChange::Delete { start, end } => TextEdit::Delete { start: *start, end: *end },
            DiffChange::Replace { start, end, text } => {
                TextEdit::Replace { start: *start, end: *end, text: text.clone() }
            }
        })
        .collect();

    Some(edits)
}

/// Apply a `DiffResult` to in-memory text and return the modified text.
/// This is a pure function — no side effects.
pub fn preview_diff(diff: &DiffResult, current_text: &str) -> Option<String> {
    diff.apply_to(current_text)
}

/// Validate that a `DiffResult` is safe to apply (no out-of-bounds indices).
pub fn validate_diff(diff: &DiffResult, text_len: usize) -> Result<(), String> {
    for change in &diff.changes {
        match change {
            DiffChange::Insert { index, .. } => {
                if *index > text_len {
                    return Err(format!(
                        "Insert index {index} out of bounds (text length: {text_len})"
                    ));
                }
            }
            DiffChange::Delete { start, end } => {
                if *start > text_len || *end > text_len {
                    return Err(format!(
                        "Delete range {start}..{end} out of bounds (text length: {text_len})"
                    ));
                }
                if *start > *end {
                    return Err(format!("Delete range start {start} > end {end}"));
                }
            }
            DiffChange::Replace { start, end, .. } => {
                if *start > text_len || *end > text_len {
                    return Err(format!(
                        "Replace range {start}..{end} out of bounds (text length: {text_len})"
                    ));
                }
                if *start > *end {
                    return Err(format!("Replace range start {start} > end {end}"));
                }
            }
        }
    }
    Ok(())
}

/// Compute the total character delta from a diff (for status display).
pub fn diff_summary(diff: &DiffResult) -> String {
    let inserts = diff.changes.iter().filter(|c| matches!(c, DiffChange::Insert { .. })).count();
    let deletes = diff.changes.iter().filter(|c| matches!(c, DiffChange::Delete { .. })).count();
    let replaces = diff.changes.iter().filter(|c| matches!(c, DiffChange::Replace { .. })).count();

    match (inserts, deletes, replaces) {
        (0, 0, 0) => "No changes".into(),
        (0, 0, r) => format!("{r} replacement(s)"),
        (i, 0, 0) => format!("{i} insertion(s)"),
        (0, d, 0) => format!("{d} deletion(s)"),
        (i, d, r) => format!("{i} insert, {d} delete, {r} replace"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_to_edits_insert() {
        let diff = DiffResult {
            buffer_id: "buf:test".into(),
            changes: vec![DiffChange::Insert { index: 4, text: " world".into() }],
            full_replacement: None,
            summary: "add".into(),
        };
        let edits = diff_to_text_edits(&diff).unwrap();
        assert_eq!(edits.len(), 1);
        assert!(matches!(edits[0], TextEdit::Insert { index: 4, .. }));
    }

    #[test]
    fn diff_to_edits_empty_returns_none() {
        let diff = DiffResult::empty("buf");
        assert!(diff_to_text_edits(&diff).is_none());
    }

    #[test]
    fn preview_diff_applies_correctly() {
        let diff = DiffResult {
            buffer_id: "buf:test".into(),
            changes: vec![DiffChange::Replace { start: 0, end: 5, text: "HI".into() }],
            full_replacement: None,
            summary: "replace".into(),
        };
        let result = preview_diff(&diff, "hello");
        assert_eq!(result, Some("HI".into()));
    }

    #[test]
    fn validate_diff_rejects_out_of_bounds_insert() {
        let diff = DiffResult {
            buffer_id: "buf:test".into(),
            changes: vec![DiffChange::Insert { index: 100, text: "x".into() }],
            full_replacement: None,
            summary: "bad".into(),
        };
        assert!(validate_diff(&diff, 5).is_err());
    }

    #[test]
    fn validate_diff_rejects_invalid_range() {
        let diff = DiffResult {
            buffer_id: "buf:test".into(),
            changes: vec![DiffChange::Delete { start: 10, end: 5 }],
            full_replacement: None,
            summary: "bad".into(),
        };
        assert!(validate_diff(&diff, 20).is_err());
    }

    #[test]
    fn validate_diff_accepts_valid_changes() {
        let diff = DiffResult {
            buffer_id: "buf:test".into(),
            changes: vec![
                DiffChange::Insert { index: 0, text: "a".into() },
                DiffChange::Delete { start: 1, end: 3 },
                DiffChange::Replace { start: 0, end: 5, text: "new".into() },
            ],
            full_replacement: None,
            summary: "valid".into(),
        };
        assert!(validate_diff(&diff, 10).is_ok());
    }

    #[test]
    fn diff_summary_describes_changes() {
        let diff = DiffResult {
            buffer_id: "buf:test".into(),
            changes: vec![
                DiffChange::Insert { index: 0, text: "a".into() },
                DiffChange::Delete { start: 1, end: 3 },
                DiffChange::Replace { start: 0, end: 5, text: "new".into() },
            ],
            full_replacement: None,
            summary: "".into(),
        };
        let s = diff_summary(&diff);
        assert!(s.contains("1 insert"));
        assert!(s.contains("1 delete"));
        assert!(s.contains("1 replace"));
    }
}
