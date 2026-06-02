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

/// Build a `ContentView` for a proposed AI edit, given the proposal text and
/// target buffer name. Includes Accept / Reject / Edit action labels.
pub fn proposal_content_view(
    proposal_text: &str,
    target_buffer: &str,
    summary: &str,
) -> ContentView {
    let content = AiPanelContent {
        title: "Assistant".into(),
        subtitle: format!("Proposal for {}", target_buffer),
        kind: Some("edit".into()),
        target_buffer: Some(target_buffer.to_string()),
        state: zaroxi_domain_ai::panel::AiPanelState::Proposed,
        summary: summary.to_string(),
        body_lines: proposal_text.lines().map(|l| l.to_string()).collect(),
        action_labels: vec!["Accept".into(), "Reject".into(), "Edit".into()],
    };
    into_content_view(&content)
}

/// Build a `ContentView` for an AI explain/analysis result.
pub fn explain_content_view(result: &str, target_buffer: &str) -> ContentView {
    let content = AiPanelContent {
        title: "Assistant".into(),
        subtitle: format!("Analysis: {}", target_buffer),
        kind: Some("explain".into()),
        target_buffer: Some(target_buffer.to_string()),
        state: zaroxi_domain_ai::panel::AiPanelState::Ready,
        summary: String::new(),
        body_lines: result.lines().map(|l| l.to_string()).collect(),
        action_labels: vec![],
    };
    into_content_view(&content)
}

/// Build a `ContentView` for an applied AI edit showing the confirmation result.
pub fn applied_content_view(result: &str, target_buffer: &str) -> ContentView {
    let content = AiPanelContent {
        title: "Assistant".into(),
        subtitle: format!("Applied: {}", target_buffer),
        kind: Some("edit".into()),
        target_buffer: Some(target_buffer.to_string()),
        state: zaroxi_domain_ai::panel::AiPanelState::Applied,
        summary: result.to_string(),
        body_lines: vec![],
        action_labels: vec![],
    };
    into_content_view(&content)
}

/// Build a `ContentView` from diagnostics counts for the active buffer.
/// Produces severity-count lines and optional individual message lines.
pub fn diagnostics_content_view(
    errors: u32,
    warnings: u32,
    infos: u32,
    hints: u32,
    buffer_display: &str,
    detail_lines: &[String],
) -> ContentView {
    let mut lines = Vec::new();
    if errors > 0 {
        lines.push(format!("• {} error(s)", errors));
    }
    if warnings > 0 {
        lines.push(format!("• {} warning(s)", warnings));
    }
    if infos > 0 {
        lines.push(format!("• {} info(s)", infos));
    }
    if hints > 0 {
        lines.push(format!("• {} hint(s)", hints));
    }
    // Individual diagnostic messages when available.
    for dl in detail_lines.iter().take(5) {
        lines.push(format!("  {}", dl));
    }
    let subtitle = format!("Diagnostics for {}", buffer_display);
    ContentView::new("Problems", &subtitle, lines)
}
