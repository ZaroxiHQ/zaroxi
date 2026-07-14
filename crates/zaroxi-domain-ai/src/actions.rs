//! AI action tools — domain types for IDE actions (explain, refactor, generate tests,
//! fix diagnostics) and the structured diff model for file mutations.
//!
//! Phase 2: tools/actions + file-aware context + codebase operations.

use serde::{Deserialize, Serialize};

/// The kind of AI action the user invokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionKind {
    /// Explain the selection or active file.
    Explain,
    /// Refactor/improve the selection or active file.
    Refactor,
    /// Generate unit tests for the selection or active file.
    GenerateTests,
    /// Fix or suggest fixes for diagnostics in the active file.
    FixDiagnostics,
    /// General edit / code change.
    Edit,
    /// Review: analyze code for issues, patterns, improvements.
    Review,
}

impl ActionKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            ActionKind::Explain => "Explain",
            ActionKind::Refactor => "Refactor",
            ActionKind::GenerateTests => "Generate Tests",
            ActionKind::FixDiagnostics => "Fix Issues",
            ActionKind::Edit => "Edit",
            ActionKind::Review => "Review",
        }
    }

    /// The prompt instruction template for this action.
    pub fn prompt_instruction(&self) -> &'static str {
        match self {
            ActionKind::Explain => {
                "Explain the following code. What does it do, and how does it work? Be concise."
            }
            ActionKind::Refactor => {
                "Refactor the following code for clarity, performance, and correctness. Provide the improved code and explain your changes."
            }
            ActionKind::GenerateTests => {
                "Generate comprehensive unit tests for the following code. Cover edge cases, error paths, and happy paths."
            }
            ActionKind::FixDiagnostics => {
                "Fix the following issues in this code. Address each problem and produce corrected code."
            }
            ActionKind::Edit => "Edit the following code according to the request above.",
            ActionKind::Review => {
                "Review the following code for bugs, performance issues, security problems, and style violations. Be thorough."
            }
        }
    }

    /// The default system message for this action.
    pub fn system_message(&self) -> &'static str {
        match self {
            ActionKind::Explain => {
                "You are an expert software engineer explaining code to a developer. Be clear, concise, and educational."
            }
            ActionKind::Refactor => {
                "You are an expert software engineer. Refactor code for quality and provide the improved version with clear explanations."
            }
            ActionKind::GenerateTests => {
                "You are an expert software engineer specializing in testing. Write comprehensive, correct unit tests."
            }
            ActionKind::FixDiagnostics => {
                "You are an expert software engineer. Diagnose and fix issues in code. Provide corrected code."
            }
            ActionKind::Edit => {
                "You are an expert software engineer. Make precise, minimal edits to code as requested."
            }
            ActionKind::Review => {
                "You are a senior code reviewer. Identify bugs, performance issues, security vulnerabilities, and style problems."
            }
        }
    }
}

/// Specification for an AI action request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionSpec {
    /// The kind of action.
    pub kind: ActionKind,
    /// User-provided freeform instruction (can be empty for default behaviour).
    pub instruction: Option<String>,
    /// Target buffer/file identifier.
    pub target_buffer: String,
    /// The code/content to operate on (selection or full file).
    pub target_content: String,
    /// Optional: visible context around the target (wider file context).
    pub surrounding_context: Option<String>,
    /// Optional: diagnostics to fix (for FixDiagnostics action).
    pub diagnostics: Vec<DiagnosticInfo>,
    /// Optional: file path for context in prompts.
    pub file_path: Option<String>,
    /// Optional: language identifier for syntax-aware prompts.
    pub language: Option<String>,
}

/// Lightweight diagnostic info for inclusion in action context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub severity: String,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// A single change in a diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffChange {
    /// Insert text at a character index.
    Insert { index: usize, text: String },
    /// Delete text in range [start, end).
    Delete { start: usize, end: usize },
    /// Replace text in range [start, end) with new text.
    Replace { start: usize, end: usize, text: String },
}

impl DiffChange {
    /// Approximate the change in characters (net addition or removal).
    pub fn net_change(&self) -> isize {
        match self {
            DiffChange::Insert { text, .. } => text.chars().count() as isize,
            DiffChange::Delete { start, end } => -((*end as isize) - (*start as isize)),
            DiffChange::Replace { start, end, text } => {
                let removed = (*end as isize) - (*start as isize);
                let added = text.chars().count() as isize;
                added - removed
            }
        }
    }

    /// The affected start index.
    pub fn start(&self) -> usize {
        match self {
            DiffChange::Insert { index, .. } => *index,
            DiffChange::Delete { start, .. } => *start,
            DiffChange::Replace { start, .. } => *start,
        }
    }
}

/// A structured diff result from an AI action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffResult {
    /// The target buffer this diff applies to.
    pub buffer_id: String,
    /// Ordered list of changes (applied from last to first to preserve indices).
    pub changes: Vec<DiffChange>,
    /// Optional: the complete replacement text (when full-file replace is simpler).
    pub full_replacement: Option<String>,
    /// Human-readable summary of the changes.
    pub summary: String,
}

impl DiffResult {
    pub fn empty(buffer_id: impl Into<String>) -> Self {
        Self {
            buffer_id: buffer_id.into(),
            changes: Vec::new(),
            full_replacement: None,
            summary: "No changes".into(),
        }
    }

    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty() || self.full_replacement.is_some()
    }

    /// Apply changes in reverse order to maintain correct indices.
    /// Returns the modified text or None if there are no changes.
    pub fn apply_to(&self, text: &str) -> Option<String> {
        if let Some(ref replacement) = self.full_replacement {
            return Some(replacement.clone());
        }
        if self.changes.is_empty() {
            return None;
        }
        let mut result = text.to_string();
        // Apply changes from last to first to preserve insert/delete indices
        let mut sorted = self.changes.clone();
        sorted.sort_by_key(|c| std::cmp::Reverse(c.start()));
        for change in sorted {
            match change {
                DiffChange::Insert { index, text } => {
                    let idx = index.min(result.len());
                    result.insert_str(idx, &text);
                }
                DiffChange::Delete { start, end } => {
                    let s = start.min(result.len());
                    let e = end.min(result.len());
                    if s < e {
                        result.replace_range(s..e, "");
                    }
                }
                DiffChange::Replace { start, end, text } => {
                    let s = start.min(result.len());
                    let e = end.min(result.len());
                    if s <= e {
                        result.replace_range(s..e, &text);
                    }
                }
            }
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_kind_display_names() {
        assert_eq!(ActionKind::Explain.display_name(), "Explain");
        assert_eq!(ActionKind::Refactor.display_name(), "Refactor");
        assert_eq!(ActionKind::GenerateTests.display_name(), "Generate Tests");
        assert_eq!(ActionKind::FixDiagnostics.display_name(), "Fix Issues");
        assert_eq!(ActionKind::Edit.display_name(), "Edit");
        assert_eq!(ActionKind::Review.display_name(), "Review");
    }

    #[test]
    fn diff_change_net_change() {
        assert_eq!(DiffChange::Insert { index: 0, text: "abc".into() }.net_change(), 3);
        assert_eq!(DiffChange::Delete { start: 0, end: 10 }.net_change(), -10);
        assert_eq!(DiffChange::Replace { start: 0, end: 5, text: "hello".into() }.net_change(), 0);
        assert_eq!(DiffChange::Replace { start: 0, end: 3, text: "hello".into() }.net_change(), 2);
    }

    #[test]
    fn diff_result_apply_insert() {
        let diff = DiffResult {
            buffer_id: "test".into(),
            changes: vec![DiffChange::Insert { index: 5, text: " world".into() }],
            full_replacement: None,
            summary: "add greeting".into(),
        };
        assert_eq!(diff.apply_to("hello"), Some("hello world".into()));
    }

    #[test]
    fn diff_result_apply_delete() {
        let diff = DiffResult {
            buffer_id: "test".into(),
            changes: vec![DiffChange::Delete { start: 4, end: 11 }],
            full_replacement: None,
            summary: "remove".into(),
        };
        assert_eq!(diff.apply_to("hello world"), Some("hell".into()));
    }

    #[test]
    fn diff_result_apply_replace() {
        let diff = DiffResult {
            buffer_id: "test".into(),
            changes: vec![DiffChange::Replace { start: 0, end: 5, text: "HI".into() }],
            full_replacement: None,
            summary: "uppercase".into(),
        };
        assert_eq!(diff.apply_to("hello"), Some("HI".into()));
    }

    #[test]
    fn diff_result_applies_reverse_order() {
        let diff = DiffResult {
            buffer_id: "test".into(),
            changes: vec![
                DiffChange::Insert { index: 0, text: "a".into() },
                DiffChange::Insert { index: 0, text: "b".into() },
            ],
            full_replacement: None,
            summary: "prepend".into(),
        };
        // Sorted reverse by start: both at 0, stable order maintains original
        assert_eq!(diff.apply_to(""), Some("ba".into()));
    }

    #[test]
    fn diff_result_full_replacement() {
        let diff = DiffResult {
            buffer_id: "test".into(),
            changes: vec![DiffChange::Delete { start: 0, end: 5 }],
            full_replacement: Some("completely new".into()),
            summary: "full replace".into(),
        };
        assert_eq!(diff.apply_to("hello"), Some("completely new".into()));
    }

    #[test]
    fn diff_result_no_changes_returns_none() {
        let diff = DiffResult::empty("buf");
        assert!(!diff.has_changes());
        assert_eq!(diff.apply_to("hello"), None);
    }

    #[test]
    fn action_spec_default_instruction() {
        let spec = ActionSpec {
            kind: ActionKind::Explain,
            instruction: None,
            target_buffer: "buf:main".into(),
            target_content: "fn main() {}".into(),
            surrounding_context: None,
            diagnostics: vec![],
            file_path: Some("src/main.rs".into()),
            language: Some("rust".into()),
        };
        assert_eq!(spec.instruction, None);
        assert_eq!(spec.kind.display_name(), "Explain");
    }
}
