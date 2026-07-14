//! Context collector — assembles multi-file IDE context from workspace state.
//!
//! Phase 2: collects active file, selection, open tabs, diagnostics, git status,
//! and workspace root information into a `ContextPack` for AI requests.

use zaroxi_domain_ai::actions::{ActionKind, ActionSpec, DiagnosticInfo};
use zaroxi_domain_ai::context_ide::{ContextPack, ContextPackBuilder};

/// Collect multi-file context for an AI action.
///
/// Assembles context from available IDE state:
/// - Active file content + path
/// - Editor selection (if present)
/// - Open tabs / files
/// - Diagnostics summary
/// - Workspace root
/// - Git status summary (if available)
/// - User-attached content
pub fn collect_context(
    spec: &ActionSpec,
    active_file_path: Option<&str>,
    open_tabs: &[String],
    workspace_root: Option<&str>,
    workspace_name: Option<&str>,
    git_status: Option<&str>,
    max_tokens: usize,
) -> ContextPack {
    let mut builder = ContextPackBuilder::new(max_tokens);

    // Active file — always auto-attached (within budget)
    if let Some(path) = active_file_path {
        let file_content = &spec.target_content;
        let token_est = token_estimate(file_content);
        builder = builder.add_active_file(path, file_content, token_est);

        // Surrounding context as a separate item
        if let Some(ref surrounding) = spec.surrounding_context {
            if !surrounding.is_empty() {
                let est = token_estimate(surrounding);
                builder = builder.add_selection("surrounding code", surrounding, est);
            }
        }
    }

    // Selection auto-attached
    if !spec.target_content.is_empty() {
        let label = if spec.instruction.is_some() { "target + instruction" } else { "target code" };
        let est = token_estimate(&spec.target_content);
        builder = builder.add_selection(label, &spec.target_content, est);
    }

    // Workspace root
    if let Some(root) = workspace_root {
        let name = workspace_name.unwrap_or(root);
        builder = builder.add_workspace_root(root, name, 4);
    }

    // Diagnostics — opt-out
    if !spec.diagnostics.is_empty() {
        let summary = diagnostics_summary(&spec.diagnostics);
        let est = token_estimate(&summary);
        builder = builder.add_diagnostics(&summary, est);
    }

    // Open tabs — opt-in
    if !open_tabs.is_empty() {
        let est = open_tabs.len() * 10; // rough estimate
        builder = builder.add_open_tabs(open_tabs, est);
    }

    // Git status — opt-in
    if let Some(status) = git_status {
        let est = token_estimate(status);
        builder = builder.add_git_status(status, est);
    }

    builder.build()
}

/// Collect context specifically for a diagnostics-fix action.
pub fn collect_diagnostics_context(
    spec: &ActionSpec,
    active_file_path: Option<&str>,
    max_tokens: usize,
) -> ContextPack {
    let mut builder = ContextPackBuilder::new(max_tokens);

    if let Some(path) = active_file_path {
        let est = token_estimate(&spec.target_content);
        builder = builder.add_active_file(path, &spec.target_content, est);
    }

    // Diagnostics are the primary payload
    if !spec.diagnostics.is_empty() {
        let summary = diagnostics_summary(&spec.diagnostics);
        let est = token_estimate(&summary);
        builder = builder.add_diagnostics(&summary, est);
    }

    builder.build()
}

/// Build a formatted diagnostics summary string.
fn diagnostics_summary(diagnostics: &[DiagnosticInfo]) -> String {
    let mut lines = Vec::new();
    for d in diagnostics {
        let mut line = format!("[{}] {}", d.severity, d.message);
        if let (Some(l), Some(c)) = (d.line, d.column) {
            line.push_str(&format!(" (L{l}:C{c})"));
        }
        lines.push(line);
    }
    if lines.len() > 20 {
        let remaining = lines.len() - 20;
        lines.truncate(20);
        lines.push(format!("... and {remaining} more issues"));
    }
    lines.join("\n")
}

/// Approximate token count from character length.
fn token_estimate(text: &str) -> usize {
    text.len() / 4 + 1
}

/// Build an ActionSpec for a selection-based action.
pub fn build_action_spec(
    kind: ActionKind,
    buffer_id: &str,
    content: &str,
    selection: Option<&str>,
    instruction: Option<&str>,
    file_path: Option<&str>,
    language: Option<&str>,
    diagnostics: Vec<DiagnosticInfo>,
    surrounding: Option<&str>,
) -> ActionSpec {
    let target = selection.unwrap_or(content).to_string();
    let surrounding_ctx = surrounding.map(|s| s.to_string());

    ActionSpec {
        kind,
        instruction: instruction.map(|s| s.to_string()),
        target_buffer: buffer_id.to_string(),
        target_content: target,
        surrounding_context: surrounding_ctx,
        diagnostics,
        file_path: file_path.map(|s| s.to_string()),
        language: language.map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_context_includes_active_file_and_workspace() {
        let spec = ActionSpec {
            kind: ActionKind::Explain,
            instruction: None,
            target_buffer: "buf:main".into(),
            target_content: "fn main() {}".into(),
            surrounding_context: None,
            diagnostics: vec![],
            file_path: Some("src/main.rs".into()),
            language: None,
        };
        let pack = collect_context(
            &spec,
            Some("src/main.rs"),
            &[],
            Some("/home/project"),
            Some("my-crate"),
            None,
            64_000,
        );
        assert!(pack.has_any_context());
        assert!(!pack.auto_attached().is_empty());
    }

    #[test]
    fn collect_context_includes_diagnostics() {
        let spec = ActionSpec {
            kind: ActionKind::FixDiagnostics,
            instruction: None,
            target_buffer: "buf:main".into(),
            target_content: "fn main() {}".into(),
            surrounding_context: None,
            diagnostics: vec![DiagnosticInfo {
                severity: "ERROR".into(),
                message: "syntax error".into(),
                line: Some(1),
                column: Some(1),
            }],
            file_path: Some("src/main.rs".into()),
            language: None,
        };
        let pack = collect_context(&spec, Some("src/main.rs"), &[], None, None, None, 64_000);
        assert!(pack.has_any_context());
    }

    #[test]
    fn diagnostics_summary_formats_items() {
        let diags = vec![
            DiagnosticInfo {
                severity: "ERROR".into(),
                message: "bad".into(),
                line: Some(10),
                column: Some(5),
            },
            DiagnosticInfo {
                severity: "WARNING".into(),
                message: "meh".into(),
                line: None,
                column: None,
            },
        ];
        let summary = diagnostics_summary(&diags);
        assert!(summary.contains("ERROR"));
        assert!(summary.contains("bad"));
        assert!(summary.contains("L10:C5"));
        assert!(summary.contains("WARNING"));
        assert!(summary.contains("meh"));
    }

    #[test]
    fn build_action_spec_uses_selection_when_present() {
        let spec = build_action_spec(
            ActionKind::Explain,
            "buf:test",
            "full file content here",
            Some("selected text"),
            Some("explain this"),
            Some("src/test.rs"),
            Some("rust"),
            vec![],
            None,
        );
        assert_eq!(spec.kind, ActionKind::Explain);
        assert_eq!(spec.target_content, "selected text");
        assert_eq!(spec.instruction, Some("explain this".into()));
        assert_eq!(spec.file_path, Some("src/test.rs".into()));
    }

    #[test]
    fn build_action_spec_falls_back_to_full_content() {
        let spec = build_action_spec(
            ActionKind::Edit,
            "buf:test",
            "full content",
            None,
            None,
            None,
            None,
            vec![],
            None,
        );
        assert_eq!(spec.target_content, "full content");
        assert!(spec.instruction.is_none());
    }
}
