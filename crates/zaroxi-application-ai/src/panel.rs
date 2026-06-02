/// Application-layer mapping: converts domain AI panel content and
/// AI proposal state into engine-owned `ContentView` for Core composition.
///
/// This module bridges the Domain (AiPanelContent, AiEditProposal) and
/// Core (ContentView) layers. The interface/desktop layer calls these
/// free functions to produce composable content for the AI panel.
use zaroxi_core_engine_ui::ContentView;
use zaroxi_domain_ai::panel::AiPanelContent;
use zaroxi_domain_ai::types::AiEditProposal;

/// Convert a domain `AiPanelContent` into a Core `ContentView`.
///
/// Flattens title, subtitle, summary, body, and action labels into
/// the generic `ContentView.lines` sequence so the existing
/// `compose_content_view` pipeline can render it.
pub fn into_content_view(panel: &AiPanelContent) -> ContentView {
    let mut lines: Vec<String> = Vec::new();

    if !panel.summary.is_empty() {
        lines.push(panel.summary.clone());
    }
    lines.extend(panel.body_lines.clone());
    if !panel.action_labels.is_empty() {
        lines.push(format!("[{}]", panel.action_labels.join(" | ")));
    }

    ContentView::new(&panel.title, &panel.subtitle, lines)
}

/// Build an `AiPanelContent` from a pending `AiEditProposal`.
///
/// `buffer_display` is an optional human-readable buffer name shown in
/// the subtitle and target field when available.
pub fn from_proposal(proposal: &AiEditProposal, buffer_display: Option<&str>) -> AiPanelContent {
    let target = buffer_display.map(String::from).unwrap_or_else(|| proposal.buffer_id.clone());
    AiPanelContent {
        title: "Assistant".into(),
        subtitle: format!("Proposal for {}", target),
        kind: Some("edit".into()),
        target_buffer: Some(target),
        state: zaroxi_domain_ai::panel::AiPanelState::Proposed,
        summary: proposal.summary.clone(),
        body_lines: proposal.proposal_text.lines().map(|l| l.to_string()).collect(),
        action_labels: vec!["Accept".into(), "Reject".into(), "Edit".into()],
    }
}

/// Convenience: produce a `ContentView` directly from an idle (no-session)
/// AI panel content, matching the previous `ContentView::ai_panel()` default.
pub fn idle_content_view() -> ContentView {
    into_content_view(&AiPanelContent::idle())
}
