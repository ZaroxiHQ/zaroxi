/*!
Small helper module extracted from desktop.rs to produce a one-line StatusBarLine.

This extraction is intentionally narrow: it contains the exact logic that maps the
composition metadata into a compact single-line status suitable for shells/harnesses.
The implementation behavior is unchanged; the helper merely centralizes responsibility
so `desktop.rs` remains smaller and easier to navigate.
*/

/// Compute a small one-line status bar for the given DesktopComposition.
///
/// - Prefers AI projection textual result when present: "AI: <result (truncated)>".
/// - Otherwise falls back to a short mapping of RefreshReason.
/// - Optionally populates `sticky` from active_buffer_details.display or opened_buffers.
///
/// Returns None when no metadata is present or no status is derivable.
pub fn latest_status_bar_line(comp: &super::DesktopComposition) -> Option<super::StatusBarLine> {
    // Require metadata to produce a status line.
    let meta = match &comp.metadata {
        Some(m) => m,
        None => return None,
    };

    // Helper to build sticky display (prefer active_buffer_details.display).
    let sticky = meta
        .active_buffer_details
        .as_ref()
        .and_then(|d| d.display.clone())
        .or_else(|| meta.opened_buffers.iter().find(|it| it.active).and_then(|it| it.display.clone()));

    // Prefer AI projection result when present.
    if let Some(ai) = meta.ai_projection.as_ref() {
        if let Some(result) = ai.result.as_ref() {
            // Truncate to keep status short and stable.
            let snippet: String = if result.chars().count() > 120 {
                result.chars().take(120).collect::<String>() + "..."
            } else {
                result.clone()
            };
            return Some(super::StatusBarLine { text: format!("AI: {}", snippet), sticky });
        }
    }

    // Fallback to mapping refresh reason to a concise single-line message.
    if let Some(rr) = meta.refresh_reason.as_ref() {
        let text = match rr {
            super::RefreshReason::InitialLoad => "initial load".to_string(),
            super::RefreshReason::RefreshAction => "refreshed".to_string(),
            super::RefreshReason::CursorMoved => "cursor moved".to_string(),
            super::RefreshReason::BufferUpdated => "buffer updated".to_string(),
            super::RefreshReason::ActiveBufferChanged => "active buffer changed".to_string(),
            super::RefreshReason::AiProjectionUpdated => "AI projection updated".to_string(),
        };
        return Some(super::StatusBarLine { text, sticky });
    }

    None
}
