use std::path::PathBuf;

use tokio;

use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, ListBuffersRequest, SetActiveBufferRequest, GetActiveBufferRequest, GetSessionSnapshotRequest,
    CreateCheckpointRequest, RestoreCheckpointRequest,
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

    // History store
    let history = zaroxi_infrastructure_memory::InMemoryHistoryStore::new();
    let history_dyn = zaroxi_infrastructure_memory::into_history_store(history);

    // AI mock
    let ai = zaroxi_infrastructure_ai_mock::MockAiClient::new();
    let ai_dyn = zaroxi_infrastructure_ai_mock::into_dyn(ai);

    // Compose the application orchestrator (implementation owned by application layer).
    let orchestrator = WorkspaceOrchestrator::new_with_history(repo_dyn, buffer_dyn, ai_dyn, history_dyn);

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
    // Query and print a compact session snapshot (Phase 7)
    let snap_req = GetSessionSnapshotRequest { session_id: boot_res.session.session_id.clone(), recent_limit: 10 };
    let snap_res = orchestrator.get_session_snapshot(snap_req).await.map_err(|e| e.to_string())?;
    let snap = snap_res.snapshot;
    println!("Harness: session snapshot for {} (workspace {}):", snap.session_id, snap.workspace_id);
    println!(" - opened buffers: {:?}", snap.opened_buffers);
    println!(" - active buffer: {:?}", snap.active_buffer);
    for b in snap.buffers.iter() {
        println!("   - {} -> {} bytes", b.buffer_id, b.content.as_ref().map(|s| s.len()).unwrap_or(0));
    }
    println!(" - recent commands: {}", snap.recent_commands.len());
    println!(" - recent events: {}", snap.recent_events.len());

    // Print recent commands and events for this session
    use zaroxi_application_workspace::ports::{GetRecentCommandsRequest, GetRecentEventsRequest};
    let recent_cmds = orchestrator.get_recent_commands(GetRecentCommandsRequest { session_id: boot_res.session.session_id.clone(), limit: 20 }).await.map_err(|e| e.to_string())?;
    println!("Harness: recent commands (count={}):", recent_cmds.commands.len());
    for c in recent_cmds.commands.iter() {
        println!("- {:?} success={} result={:?} error={:?}", c.kind, c.success, c.result, c.error);
    }

    let recent_events = orchestrator.get_recent_events(GetRecentEventsRequest { session_id: boot_res.session.session_id.clone(), limit: 20 }).await.map_err(|e| e.to_string())?;
    println!("Harness: recent events (count={}):", recent_events.events.len());
    for e in recent_events.events.iter() {
        println!("- {:?} at {}", e.kind, e.timestamp);
    }

    // Phase 8: create a checkpoint for the current session, then restore it into a fresh orchestrator.
    println!("Harness: creating checkpoint for session {}", boot_res.session.session_id);
    let cp_req = CreateCheckpointRequest { session_id: boot_res.session.session_id.clone() };
    let cp_res = orchestrator.create_checkpoint(cp_req).await.map_err(|e| e.to_string())?;
    let checkpoint = cp_res.checkpoint;
    println!("Harness: checkpoint created at {}", checkpoint.created_at);

    // Build fresh infra instances for restore target
    let repo2 = zaroxi_infrastructure_memory::InMemoryWorkspaceRepo::new();
    let repo2_dyn = zaroxi_infrastructure_memory::into_workspace_repo(repo2);

    let buffer_store2 = zaroxi_infrastructure_memory::InMemoryBufferStore::new();
    let buffer2_dyn = zaroxi_infrastructure_memory::into_buffer_store(buffer_store2);

    let history2 = zaroxi_infrastructure_memory::InMemoryHistoryStore::new();
    let history2_dyn = zaroxi_infrastructure_memory::into_history_store(history2);

    let ai2 = zaroxi_infrastructure_ai_mock::MockAiClient::new();
    let ai2_dyn = zaroxi_infrastructure_ai_mock::into_dyn(ai2);

    let orchestrator2 = WorkspaceOrchestrator::new_with_history(repo2_dyn, buffer2_dyn, ai2_dyn, history2_dyn);

    println!("Harness: restoring checkpoint into fresh orchestrator...");
    let restore_res = orchestrator2.restore_checkpoint(RestoreCheckpointRequest { checkpoint: checkpoint.clone() }).await.map_err(|e| e.to_string())?;
    println!("Harness: restored session: {}", restore_res.session.session_id);

    // Print restored snapshot for verification
    let snap_req2 = GetSessionSnapshotRequest { session_id: restore_res.session.session_id.clone(), recent_limit: 10 };
    let snap_res2 = orchestrator2.get_session_snapshot(snap_req2).await.map_err(|e| e.to_string())?;
    let snap2 = snap_res2.snapshot;
    println!("Harness: restored session snapshot for {} (workspace {}):", snap2.session_id, snap2.workspace_id);
    println!(" - opened buffers: {:?}", snap2.opened_buffers);
    println!(" - active buffer: {:?}", snap2.active_buffer);
    for b in snap2.buffers.iter() {
        println!("   - {} -> {} bytes", b.buffer_id, b.content.as_ref().map(|s| s.len()).unwrap_or(0));
    }

    Ok(())
}
