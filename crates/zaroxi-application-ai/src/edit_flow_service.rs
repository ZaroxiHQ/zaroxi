//! Edit-flow service — manages the full lifecycle of AI-proposed edits:
//! generation → preview → user review → approve/apply → verify.
//!
//! Phase 4: structured edit workflow with explicit approval gates.

use std::sync::{Arc, Mutex};

use zaroxi_domain_ai::actions::{ActionSpec, DiagnosticInfo, DiffChange, DiffResult};
use zaroxi_domain_ai::edit_flow::{
    EditApplyResult, EditDecision, EditFlow, EditFlowStage, EditProposal,
};

use crate::action_service::ActionService;
use crate::diff_applier;
use crate::ports::AiClient;

/// Service orchestrating edit review and application flows.
pub struct EditFlowService {
    flow: Mutex<EditFlow>,
    action_service: Arc<ActionService>,
}

impl EditFlowService {
    pub fn new(action_service: Arc<ActionService>) -> Self {
        Self { flow: Mutex::new(EditFlow::new()), action_service }
    }

    /// Get current stage.
    pub fn stage(&self) -> EditFlowStage {
        self.flow.lock().unwrap().stage
    }

    /// Get current proposal (if any).
    pub fn current_proposal(&self) -> Option<EditProposal> {
        self.flow.lock().unwrap().proposal.clone()
    }

    /// Start generating an edit for a given action spec.
    /// This is called by the agent when it wants to propose a change.
    pub fn start_generation(&self) {
        self.flow.lock().unwrap().start_generation();
    }

    /// The edit proposal is ready for review.
    pub fn proposal_ready(&self, proposal: EditProposal) {
        self.flow.lock().unwrap().proposal_ready(proposal);
    }

    /// User accepts the proposal.
    pub fn accept(&self) -> Result<EditProposal, String> {
        let mut flow = self.flow.lock().unwrap();
        flow.accept()?;
        Ok(flow.proposal.clone().ok_or("no proposal")?)
    }

    /// User rejects the proposal.
    pub fn reject(&self, reason: &str) -> Result<(), String> {
        self.flow.lock().unwrap().reject(reason)
    }

    /// User wants the AI to regenerate with modifications.
    pub fn modify(&self) -> Result<EditProposal, String> {
        let mut flow = self.flow.lock().unwrap();
        flow.modify()?;
        Ok(flow.proposal.clone().ok_or("no proposal")?)
    }

    /// Preview what applying the diff would produce.
    pub fn preview_diff(&self, current_text: &str) -> Option<String> {
        let flow = self.flow.lock().unwrap();
        let proposal = flow.proposal.as_ref()?;
        proposal.diff.apply_to(current_text)
    }

    /// Apply the accepted diff and report the result.
    pub fn apply_diff(&self, current_text: &str) -> Result<(String, EditApplyResult), String> {
        let mut flow = self.flow.lock().unwrap();

        let proposal = flow.proposal.as_ref().ok_or("no proposal")?;
        if proposal.decision != Some(EditDecision::Accepted) {
            return Err("proposal not accepted".into());
        }

        // Validate before applying
        diff_applier::validate_diff(&proposal.diff, current_text.len())?;

        let modified = diff_applier::preview_diff(&proposal.diff, current_text)
            .ok_or("failed to apply diff")?;

        let summary = diff_applier::diff_summary(&proposal.diff);
        let lines_changed = proposal.diff.changes.len();

        let result = EditApplyResult {
            proposal_id: proposal.proposal_id.clone(),
            success: true,
            message: summary.clone(),
            lines_changed: Some(lines_changed),
        };

        flow.apply_complete(result.clone());

        Ok((modified, result))
    }

    /// Reset the edit flow.
    pub fn reset(&self) {
        self.flow.lock().unwrap().reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{AiClient, AiError, AiRequest, AiResponseDTO, BoxFuture};

    struct TestAiClient;

    impl AiClient for TestAiClient {
        fn request(&self, _req: AiRequest) -> BoxFuture<'static, Result<AiResponseDTO, AiError>> {
            Box::pin(async { Ok(AiResponseDTO { text: "ok".into() }) })
        }
    }

    fn make_service() -> EditFlowService {
        let client: Arc<dyn AiClient> = Arc::new(TestAiClient);
        let action_svc = Arc::new(ActionService::new(client));
        EditFlowService::new(action_svc)
    }

    #[test]
    fn full_accept_flow() {
        let svc = make_service();
        svc.start_generation();
        assert_eq!(svc.stage(), EditFlowStage::Generating);

        let diff = DiffResult {
            buffer_id: "buf:test".into(),
            changes: vec![DiffChange::Replace { start: 0, end: 3, text: "nope".into() }],
            full_replacement: None,
            summary: "change greeting".into(),
        };

        let proposal = EditProposal::new("buf:test", "change greeting", diff, "fix: say hi");
        svc.proposal_ready(proposal);
        assert_eq!(svc.stage(), EditFlowStage::Reviewing);

        svc.accept().unwrap();
        assert_eq!(svc.stage(), EditFlowStage::Applying);

        let (result, _) = svc.apply_diff("foo bar").unwrap();
        assert_eq!(result, "nope bar");
        assert_eq!(svc.stage(), EditFlowStage::Applied);
    }

    #[test]
    fn reject_flow() {
        let svc = make_service();
        svc.start_generation();
        svc.proposal_ready(EditProposal::new("buf", "x", DiffResult::empty("buf"), "ok"));
        svc.reject("don't want this").unwrap();
        assert_eq!(svc.stage(), EditFlowStage::Rejected);
    }

    #[test]
    fn apply_without_accept_fails() {
        let svc = make_service();
        svc.start_generation();
        svc.proposal_ready(EditProposal::new("buf", "x", DiffResult::empty("buf"), "ok"));
        // Not accepted yet
        assert!(svc.apply_diff("hello").is_err());
    }
}
