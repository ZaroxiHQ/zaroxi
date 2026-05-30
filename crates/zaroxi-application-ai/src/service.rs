#![doc = "Application-level AI service used in Phase 11 to orchestrate AI edit proposals.\n\nThis service holds a small in-memory pending proposal list (presentation only)\nand exposes a simple request/accept/reject API that application orchestrators\nand interface adapters can call during Phase 11. The concrete AI provider is\nplumbed as an `AiClient` trait object provided by application composition.\n"]

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

use zaroxi_domain_ai::types::{AiEditApplyResult, AiEditProposal, AiEditRequest, AiProposalState};
use zaroxi_kernel_types::Id;

/// Re-export port traits used by infra adapters (AiClient trait lives in crate::ports).
use crate::ports::{AiClient, AiRequest};

/// AI service for handling AI-related operations and holding pending proposals.
pub struct AiService {
    /// Internal state.
    state: Arc<Mutex<AiServiceState>>,
}

struct AiServiceState {
    /// Whether the service is running.
    running: bool,
    /// In-memory pending proposals (presentation-oriented).
    pending_proposals: Vec<AiEditProposal>,
}

impl AiService {
    /// Create a new AI service.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AiServiceState {
                running: false,
                pending_proposals: Vec::new(),
            })),
        }
    }

    /// Start the AI service.
    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if state.running {
            return Err(anyhow::anyhow!("AI service is already running"));
        }
        state.running = true;
        info!("AI service started");
        Ok(())
    }

    /// Stop the AI service.
    pub async fn stop(&self) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if !state.running {
            return Err(anyhow::anyhow!("AI service is not running"));
        }
        state.running = false;
        info!("AI service stopped");
        Ok(())
    }

    /// Check if the service is running.
    pub async fn is_running(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.running
    }

    /// Request an AI edit proposal for the given request using the provided AiClient.
    ///
    /// - Calls the AiClient adapter to obtain text output.
    /// - Normalizes the provider output into an AiEditProposal (presentation DTO).
    /// - Stores the proposal in the in-memory pending_proposals list and returns it.
    ///
    /// Note: this function intentionally keeps the I/O call outside the mutex to avoid
    /// holding locks across awaits.
    pub async fn request_ai_edit(
        &self,
        req: AiEditRequest,
        client: std::sync::Arc<dyn AiClient>,
    ) -> Result<AiEditProposal, anyhow::Error> {
        // Build the adapter-level AiRequest expected by infra adapters.
        let ai_req = AiRequest {
            session_id: Id::new(),
            workspace_id: Id::new(),
            buffer_id: zaroxi_core_editor_buffer::ports::BufferId(req.buffer_id.clone()),
            content_snapshot: req.content.clone(),
        };

        // Call provider (async).
        let resp = client
            .request(ai_req)
            .await
            .map_err(|e| anyhow::anyhow!(format!("ai client error: {:?}", e)))?;

        // Build deterministic proposal id using epoch millis.
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let id = format!("proposal-{}", now.as_millis());

        let summary: String = if resp.text.len() > 80 {
            format!("{}...", &resp.text[..80])
        } else {
            resp.text.clone()
        };

        let proposal = AiEditProposal {
            id: id.clone(),
            buffer_id: req.buffer_id.clone(),
            summary,
            proposal_text: resp.text.clone(),
            state: AiProposalState::Proposed,
        };

        // Store proposal in-memory (short critical section).
        {
            let mut state = self.state.lock().unwrap();
            state.pending_proposals.push(proposal.clone());
        }

        Ok(proposal)
    }

    /// List currently pending proposals (snapshot).
    pub fn list_pending(&self) -> Vec<AiEditProposal> {
        let state = self.state.lock().unwrap();
        state.pending_proposals.clone()
    }

    /// Get a pending proposal by id.
    pub fn get_pending(&self, id: &str) -> Option<AiEditProposal> {
        let state = self.state.lock().unwrap();
        state.pending_proposals.iter().find(|p| p.id == id).cloned()
    }

    /// Accept (apply) a pending proposal.
    ///
    /// This method updates the in-memory proposal state to Applied and returns an
    /// AiEditApplyResult. The actual application of the edit to a buffer must be
    /// performed by the workspace/orchestrator using the proposal.proposal_text
    /// via the editor transaction pipeline. For Phase 1 we mark the proposal as
    /// Applied here to reflect the intent; the caller should perform the real apply.
    pub fn accept_proposal(&self, id: &str) -> Result<AiEditApplyResult, anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if let Some(pos) = state.pending_proposals.iter().position(|p| p.id == id) {
            if let Some(p) = state.pending_proposals.get_mut(pos) {
                p.state = AiProposalState::Applied;
                return Ok(AiEditApplyResult {
                    ok: true,
                    message: Some("Marked applied (service)".to_string()),
                });
            }
        }
        Err(anyhow::anyhow!("proposal not found"))
    }

    /// Reject (clear) a pending proposal: mark rejected and remove from pending list.
    pub fn reject_proposal(&self, id: &str) -> Result<(), anyhow::Error> {
        let mut state = self.state.lock().unwrap();
        if let Some(pos) = state.pending_proposals.iter().position(|p| p.id == id) {
            state.pending_proposals.remove(pos);
            return Ok(());
        }
        Err(anyhow::anyhow!("proposal not found"))
    }
}
