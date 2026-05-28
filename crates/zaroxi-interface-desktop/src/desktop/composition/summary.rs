/*!
Summary/debug/status helpers for DesktopComposition.

This module contains small, pure helper functions that build concise summaries
from the composition. The AI projection summary was migrated here to keep
summary helpers grouped.
*/

/// Compute a tiny, read-only AI projection summary intended for shell consumption.
///
/// This function mirrors exactly the logic previously embedded in
/// `DesktopComposition::latest_ai_projection_summary` and must remain behaviourally
/// identical. It is pure/derivational and reads the composition only.
pub fn latest_ai_projection_summary(
    comp: &super::DesktopComposition,
) -> Option<super::AiProjectionSummary> {
    let ap = comp.latest_ai_projection()?;
    // Map kind string to small enum
    let kind_opt = ap.kind.as_ref().map(|k| {
        let kl = k.to_lowercase();
        if kl.contains("explain") {
            super::AiKind::Explain
        } else if kl.contains("suggest") || kl.contains("suggestion") {
            super::AiKind::Suggest
        } else if kl.contains("refactor") || kl.contains("refactoring") {
            super::AiKind::Refactor
        } else {
            super::AiKind::Other(k.clone())
        }
    });

    // Determine a minimal state hint
    let state = if ap.result.is_some() {
        super::AiState::Ready
    } else if ap.kind.is_some() {
        super::AiState::Running
    } else {
        super::AiState::Failed
    };

    Some(super::AiProjectionSummary {
        present: true,
        kind: kind_opt,
        target_buffer: ap.target_buffer.clone(),
        state,
    })
}
