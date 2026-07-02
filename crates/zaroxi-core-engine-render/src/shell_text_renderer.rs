/*!
ShellTextRenderer — tiny deterministic text renderer (Phase 55).

Rationale:
- Provide a minimal, deterministic, backend‑free renderer that consumes
  ShellDrawPlan and emits a stable textual representation.
- Keep all logic inside the existing `zaroxi-core-engine-render` crate.
- Avoid any UI/graphics dependencies; use a debug-based deterministic
  textual format suitable for tests and early validation.

Public API added:
- type: ShellTextRenderer
- fn new() -> Self
- fn render(&self, &ShellDrawPlan) -> ShellRenderTranscript
- fn render_lines(&self, &ShellDrawPlan) -> Vec<String>
- fn matches_transcript(&self, &ShellDrawPlan, &ShellRenderTranscript) -> bool

Output type:
- ShellRenderTranscript (crate-local type) — carries Vec<String> lines.

Validation commands (run from repository root):
- cargo test -p zaroxi-core-engine-render
- bash scripts/architecture_check.sh

This module intentionally stays tiny and deterministic: it formats the
ShellDrawPlan using Debug (pretty) output and prefixes a stable header.
*/

use crate::{plan::ShellDrawPlan, transcript::ShellRenderTranscript};

#[derive(Debug, Clone, Copy)]
pub struct ShellTextRenderer;

impl Default for ShellTextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellTextRenderer {
    /// Create a new renderer instance.
    pub fn new() -> Self {
        Self
    }

    /// Render a ShellDrawPlan into a deterministic ShellRenderTranscript.
    ///
    /// The rendering is intentionally simple: we emit a header line and the
    /// pretty-printed Debug representation of the plan. This keeps the output
    /// deterministic and backend-free while being useful for tests and early
    /// integration.
    pub fn render(&self, plan: &ShellDrawPlan) -> ShellRenderTranscript {
        let mut lines = Vec::new();
        lines.push("ShellDrawPlan:".to_string());
        lines.push(format!("{:#?}", plan));
        ShellRenderTranscript { lines }
    }

    /// Convenience: return the rendered lines directly.
    pub fn render_lines(&self, plan: &ShellDrawPlan) -> Vec<String> {
        self.render(plan).lines
    }

    /// Compare the rendered output against an existing transcript.
    /// Returns true when the textual lines match exactly.
    pub fn matches_transcript(
        &self,
        plan: &ShellDrawPlan,
        expected: &ShellRenderTranscript,
    ) -> bool {
        let actual = self.render(plan);
        actual.lines == expected.lines
    }
}
