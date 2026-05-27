/*!
Render consistency seam (Phase 56).

Provides a tiny deterministic verification utility that checks whether the
text produced by the ShellTextRenderer aligns with the canonical
ShellRenderTranscript for the same ShellDrawPlan.

This module is intentionally small and deterministic:
- Type: RenderConsistencyReport
- Function: analyze(&ShellDrawPlan) -> RenderConsistencyReport

Comparison rule:
- Build transcript_str = ShellRenderTranscript::from(&plan).to_string()
- Build renderer_str  = ShellTextRenderer::new().render(plan).to_string()
- If renderer_str starts with "ShellDrawPlan:\n", strip that prefix.
- Compare transcript_str == renderer_plan_part (exact string equality).
- Produce a tiny deterministic report indicating alignment or a short mismatch message.

No new rendering logic is added. This is a verification seam only.
*/

use crate::{ShellTextRenderer, plan::ShellDrawPlan, transcript::ShellRenderTranscript};

/// Tiny deterministic consistency report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderConsistencyReport {
    /// True when transcript and renderer-derived text match under the seam rules.
    pub aligned: bool,
    /// Short deterministic mismatch messages when aligned == false.
    pub mismatches: Vec<String>,
}

/// Analyze a ShellDrawPlan and produce a deterministic RenderConsistencyReport.
///
/// The comparison is intentionally simple and deterministic: it compares the
/// canonical ShellRenderTranscript text against the ShellTextRenderer output
/// for the same plan, after removing the renderer header ("ShellDrawPlan:\n")
/// if present.
pub fn analyze(plan: &ShellDrawPlan) -> RenderConsistencyReport {
    // Canonical transcript for the plan.
    let transcript: ShellRenderTranscript = ShellRenderTranscript::from(plan);
    let transcript_str = transcript.to_string();

    // Renderer output (deterministic text).
    let renderer = ShellTextRenderer::new();
    let renderer_transcript = renderer.render(plan);
    let renderer_str = renderer_transcript.to_string();

    // Normalize by stripping the known renderer header if present.
    const HEADER: &str = "ShellDrawPlan:\n";
    let renderer_plan_part = if renderer_str.starts_with(HEADER) {
        &renderer_str[HEADER.len()..]
    } else {
        &renderer_str[..]
    };

    let mut mismatches = Vec::new();

    if transcript_str != renderer_plan_part {
        // Keep the message small and deterministic. Provide a short preview of the first differing region.
        // Find first index of difference (byte-wise) for deterministic context.
        let t_bytes = transcript_str.as_bytes();
        let r_bytes = renderer_plan_part.as_bytes();
        let mut idx = 0usize;
        let min_len = std::cmp::min(t_bytes.len(), r_bytes.len());
        while idx < min_len && t_bytes[idx] == r_bytes[idx] {
            idx += 1;
        }

        // Extract short context (up to 40 bytes) around first difference.
        let context_len = 40usize;
        let t_snip = if idx >= context_len {
            &transcript_str
                [idx - context_len..std::cmp::min(idx + context_len, transcript_str.len())]
        } else {
            &transcript_str[0..std::cmp::min(context_len * 2, transcript_str.len())]
        };
        let r_snip = if idx >= context_len {
            &renderer_plan_part
                [idx - context_len..std::cmp::min(idx + context_len, renderer_plan_part.len())]
        } else {
            &renderer_plan_part[0..std::cmp::min(context_len * 2, renderer_plan_part.len())]
        };

        mismatches.push(format!("textual plan mismatch"));
        mismatches.push(format!("transcript_preview=\"{}\"", t_snip.replace("\n", "\\n")));
        mismatches.push(format!("renderer_preview=\"{}\"", r_snip.replace("\n", "\\n")));
    }

    RenderConsistencyReport { aligned: mismatches.is_empty(), mismatches }
}
