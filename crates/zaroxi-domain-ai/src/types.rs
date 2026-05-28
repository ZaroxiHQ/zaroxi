#![allow(dead_code)]
// Domain types for AI editing (Phase 11).
//
// Keep these types lightweight, UI-agnostic and independent of interface crates.

/// Unique identifier for a proposal.
pub type ProposalId = String;

/// AI edit request captured by the application layer and forwarded to an AI provider.
#[derive(Clone, Debug)]
pub struct AiEditRequest {
    /// Session identifier (shell/session id).
    pub session_id: String,
    /// Target buffer identifier (opaque string form).
    pub buffer_id: String,
    /// Snapshot of the buffer content at request time.
    pub content: String,
}

/// Possible states for an AI edit proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiProposalState {
    /// A proposal has been produced and is awaiting accept/reject.
    Proposed,
    /// Proposal was accepted and applied to the buffer.
    Applied,
    /// Proposal was explicitly rejected by the user.
    Rejected,
    /// Provider or apply failed.
    Failed,
}

/// AI edit proposal returned by the provider and stored as a pending proposal
/// in application state until the user accepts or rejects it.
#[derive(Clone, Debug)]
pub struct AiEditProposal {
    /// Stable identifier for the proposal (opaque).
    pub id: ProposalId,
    /// Target buffer id the proposal is intended to modify.
    pub buffer_id: String,
    /// Short human readable summary for UI listings (one-line).
    pub summary: String,
    /// Full proposal text (could be a patch, a replacement, or structured instruction).
    pub proposal_text: String,
    /// Current state of the proposal.
    pub state: AiProposalState,
}

/// Result of attempting to apply a proposal to a buffer.
#[derive(Clone, Debug)]
pub struct AiEditApplyResult {
    pub ok: bool,
    pub message: Option<String>,
}
