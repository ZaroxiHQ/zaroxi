use std::path::PathBuf;

use zaroxi_application_workspace::in_memory_adapters;
use zaroxi_application_workspace::ports::{
    ApplyAiEditRequest, OpenBufferRequest, RequestAiEditRequest, WorkspaceBootRequest,
    WorkspaceService,
};
use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;

#[tokio::test]
async fn request_and_apply_ai_edit_flow() {
    // Build simple test adapters similar to harness
    let repo = in_memory_adapters::InMemoryWorkspaceRepo::new();
    let repo_dyn = in_memory_adapters::into_workspace_repo(repo);

    let buffer_store = in_memory_adapters::InMemoryBufferStore::new();
    let buffer_dyn = in_memory_adapters::into_buffer_store(buffer_store);

    // Use the infra mock already present in the workspace test harness.
    let ai = zaroxi_infrastructure_ai_mock::MockAiClient::new();
    let ai_dyn = zaroxi_infrastructure_ai_mock::into_dyn(ai);

    // No-op history and durability for tests.
    let history = zaroxi_infrastructure_memory::InMemoryHistoryStore::new();
    let history_dyn = zaroxi_infrastructure_memory::into_history_store(history);

    let checkpoint_store = zaroxi_infrastructure_memory::InMemoryCheckpointStore::new();
    let checkpoint_dyn = zaroxi_infrastructure_memory::into_checkpoint_store(checkpoint_store);

    let orch = WorkspaceOrchestrator::new_with_history_and_durability(
        repo_dyn,
        buffer_dyn,
        ai_dyn,
        history_dyn.clone(),
        checkpoint_dyn.clone(),
    );

    // Boot workspace
    let boot_req = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orch.boot_workspace(boot_req).await.expect("boot ok");

    // Open buffer
    let open_req = OpenBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        path: PathBuf::from("main.rs"),
    };
    let open_res = orch.open_buffer(open_req).await.expect("open ok");

    // Request AI edit for the active buffer via the workspace service.
    let req = RequestAiEditRequest {
        session_id: boot_res.session.session_id.clone(),
        buffer_id: open_res.buffer_id.clone(),
        content: Some(String::new()),
    };

    let resp = orch.request_ai_edit(req).await.expect("request ai edit ok");

    // We expect a proposal to be returned.
    assert!(resp.proposal.summary.as_deref().map(|s| s.len()).unwrap_or(0) > 0);
    assert!(!resp.proposal.proposal_text.is_empty());

    // Apply the proposal (use the returned proposal_text as the authoritative payload).
    let apply_req = ApplyAiEditRequest {
        session_id: boot_res.session.session_id.clone(),
        buffer_id: open_res.buffer_id.clone(),
        proposal_text: resp.proposal.proposal_text.clone(),
    };

    let apply_resp = orch.apply_ai_edit(apply_req).await.expect("apply ok");
    assert!(apply_resp.ok);
}
