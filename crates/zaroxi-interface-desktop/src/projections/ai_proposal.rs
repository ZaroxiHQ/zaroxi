#![allow(dead_code)]
// Lightweight presenter-facing projection for AI edit proposals.
//
// This module defines a small projection type used by the desktop composition
// to render a compact summary and preview of a pending AI edit proposal.
// It intentionally stays presentation-only and converts from the domain DTO
// when required by the composition or actions.

use std::fmt;

/// Presentation projection for an AI edit proposal.
#[derive(Clone, Debug)]
pub struct AiProposalLine {
    /// Proposal id (opaque).
    pub id: String,
    /// Target buffer id (display-friendly).
    pub buffer_id: Option<String>,
    /// Short one-line summary suitable for shell lines.
    pub summary: String,
    /// Full proposal text for preview.
    pub proposal_text: String,
    /// Current proposal state as a small string (e.g. "Proposed", "Applied").
    pub state: String,
}

impl AiProposalLine {
    /// Convert from a domain-level AiEditProposal (if present).
    /// This keeps the projection decoupled from domain internals by copying only
    /// the necessary display fields.
    pub fn from_domain<T: AsRef<str>, U: AsRef<str>>(
        id: T,
        buffer_id: Option<T>,
        summary: U,
        proposal_text: U,
        state: U,
    ) -> Self {
        AiProposalLine {
            id: id.as_ref().to_string(),
            buffer_id: buffer_id.map(|s| s.as_ref().to_string()),
            summary: summary.as_ref().to_string(),
            proposal_text: proposal_text.as_ref().to_string(),
            state: state.as_ref().to_string(),
        }
    }

    /// Render a compact single-line representation suitable for a shell status/line.
    pub fn render_compact(&self) -> String {
        let bid = self.buffer_id.as_deref().unwrap_or("<unknown>");
        format!("AI proposal [{}] target={} state={} summary={}", self.id, bid, self.state, self.summary)
    }

    /// Render a multi-line preview suitable for a small popup or details panel.
    pub fn render_preview(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("AI Proposal {}\n", self.id));
        out.push_str(&format!("Target: {}\n", self.buffer_id.as_deref().unwrap_or("<unknown>")));
        out.push_str(&format!("State: {}\n", self.state));
        out.push_str("Summary:\n");
        out.push_str(&format!("  {}\n\n", self.summary));
        out.push_str("Proposal preview:\n");
        out.push_str(&self.proposal_text);
        out
    }
}

impl fmt::Display for AiProposalLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render_compact())
    }
}
