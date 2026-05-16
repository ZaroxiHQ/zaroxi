use std::path::PathBuf;

use tokio;

use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, DispatchCommandRequest, AppCommand,
    ListBuffersRequest, SetActiveBufferRequest, GetActiveBufferRequest,
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
    let boot_res = orchestrator.boot_workspace(boot_req).await.map_err(|e| e.to_string())?;
    println!("Harness: opened workspace session: {}", boot_res.session.session_id);

    // Open two buffers
    let open1 = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open1_res = orchestrator.open_buffer(open1).await.map_err(|e| e.to_string())?;
    println!("Harness: opened buffer id: {}", open1_res.buffer_id);

    let open2 = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("lib.rs") };
    let open2_res = orchestrator.open_buffer(open2).await.map_err(|e| e.to_string())?;
    println!("Harness: opened buffer id: {}", open2_res.buffer_id);

    // List buffers and show active buffer
    let list_req = ListBuffersRequest { session_id: boot_res.session.session_id.clone() };
    let list_res = orchestrator.list_open_buffers(list_req).await.map_err(|e| e.to_string())?;
    println!("Harness: opened buffers: {:?}", list_res.buffer_ids);
    println!("Harness: active buffer: {:?}", list_res.active_buffer);

    // Switch active buffer explicitly to the second
    let set_active = SetActiveBufferRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open2_res.buffer_id.clone() };
    let set_res = orchestrator.set_active_buffer(set_active).await.map_err(|e| e.to_string())?;
    println!("Harness: set active ok: {}", set_res.ok);

    // Confirm active buffer
    let get_active = GetActiveBufferRequest { session_id: boot_res.session.session_id.clone() };
    let active_res = orchestrator.get_active_buffer(get_active).await.map_err(|e| e.to_string())?;
    println!("Harness: current active buffer: {}", active_res.buffer_id);

    // Explain the active buffer (shorthand use-case)
    let explain_req = GetActiveBufferRequest { session_id: boot_res.session.session_id.clone() };
    let explain_res = orchestrator.explain_active_buffer(explain_req).await.map_err(|e| e.to_string())?;
    println!("Harness: explain result: {}", explain_res.result.message);

    Ok(())
}
