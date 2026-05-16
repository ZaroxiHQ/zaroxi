use std::path::PathBuf;

use tokio;

use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, DispatchCommandRequest, AppCommand,
};
use zaroxi_application_workspace::ports::WorkspaceService;

// Infra adapters
use zaroxi_infrastructure_ai_mock;
use zaroxi_infrastructure_memory;

// Application orchestrator (concrete implementation lives in application crate)
use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Build concrete infra adapters
    let repo = zaroxi_infrastructure_memory::InMemoryWorkspaceRepo::new();
    let repo_dyn = zaroxi_infrastructure_memory::into_workspace_repo(repo);

    let buffer_store = zaroxi_infrastructure_memory::InMemoryBufferStore::new();
    let buffer_dyn = zaroxi_infrastructure_memory::into_buffer_store(buffer_store);

    // AI mock
    let ai = zaroxi_infrastructure_ai_mock::MockAiClient::new();
    let ai_dyn = zaroxi_infrastructure_ai_mock::into_dyn(ai);

    // Compose the application orchestrator (implementation owned by application layer).
    let orchestrator = WorkspaceOrchestrator::new(repo_dyn, buffer_dyn, ai_dyn);

    // Boot workspace (use-case)
    let boot_req = WorkspaceBootRequest { path: PathBuf::from("./sample-workspace") };
    let boot_res = orchestrator.boot_workspace(boot_req).await?;
    println!("Harness: opened workspace session: {}", boot_res.session.session_id);

    // Open buffer (use-case)
    let open_req = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open_req).await?;
    println!("Harness: opened buffer id: {}", open_res.buffer_id);

    // Mutate buffer content via application use-case (Phase 4)
    let update_req = zaroxi_application_workspace::ports::UpdateBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        buffer_id: open_res.buffer_id.clone(),
        new_content: "fn main() { println!(\"Mutated by harness\"); }".to_string(),
    };
    let update_res = orchestrator.update_buffer(update_req).await?;
    println!("Harness: update ok: {}", update_res.ok);

    // Dispatch AI explain command (use-case)
    let dispatch_req = DispatchCommandRequest {
        session_id: boot_res.session.session_id.clone(),
        command: AppCommand::AiExplain { buffer_id: open_res.buffer_id.clone() },
    };
    let dispatch_res = orchestrator.dispatch_command(dispatch_req).await?;
    println!("Harness: command result: {}", dispatch_res.result.message);

    Ok(())
}
