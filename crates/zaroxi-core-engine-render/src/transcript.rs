use crate::plan::ShellDrawPlan;

/// A tiny, deterministic, backend-free textual render transcript.
///
/// The transcript is intentionally minimal: an ordered list of lines that
/// describe the semantic structure of a ShellDrawPlan. Implementation uses
/// a stable Debug formatting of the plan to produce a repeatable textual
/// representation suitable for tests and debug output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRenderTranscript {
    /// Ordered deterministic textual lines describing the draw plan.
    pub lines: Vec<String>,
}

impl std::fmt::Display for ShellRenderTranscript {
    /// Join lines into a single string transcript.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lines.join("\n"))
    }
}

impl From<&ShellDrawPlan> for ShellRenderTranscript {
    fn from(plan: &ShellDrawPlan) -> Self {
        // Use a stable Debug representation as a tiny, backend-free transcript.
        // This produces a deterministic, ordered textual description of the plan.
        let s = format!("{:#?}", plan);
        let lines = s.lines().map(|l| l.to_string()).collect();
        ShellRenderTranscript { lines }
    }
}

impl From<ShellDrawPlan> for ShellRenderTranscript {
    fn from(plan: ShellDrawPlan) -> Self {
        ShellRenderTranscript::from(&plan)
    }
}
