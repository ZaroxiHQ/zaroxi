use std::path::PathBuf;

use tokio;

use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, ListBuffersRequest, SetActiveBufferRequest, GetActiveBufferRequest, GetSessionSnapshotRequest,
    SaveCheckpointRequest, LoadCheckpointRequest,
};
use zaroxi_application_workspace::ports::{WorkspaceService, WorkspaceView};

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

    // Compose the checkpoint durability adapter and the application orchestrator.
    let checkpoint_store = zaroxi_infrastructure_memory::InMemoryCheckpointStore::new();
    let checkpoint_dyn = zaroxi_infrastructure_memory::into_checkpoint_store(checkpoint_store);
 
    // Compose the application orchestrator (implementation owned by application layer).
    let orchestrator = WorkspaceOrchestrator::new_with_history_and_durability(repo_dyn, buffer_dyn, ai_dyn, history_dyn.clone(), checkpoint_dyn.clone());
    let orchestrator = std::sync::Arc::new(orchestrator);

    // Boot workspace (use-case)
    let boot_req = WorkspaceBootRequest { path: PathBuf::from("./sample-workspace") };
    let boot_res = orchestrator.boot_workspace(boot_req).await.map_err(|e| e.to_string())?;
    println!("Harness: opened workspace session: {}", boot_res.session.session_id);

    // Open two buffers
    let open1 = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open1_res = orchestrator.open_buffer(open1).await.map_err(|e| e.to_string())?;
    println!("Harness: opened buffer id: {}", open1_res.buffer_id);

    // Phase 3: set a cursor for this buffer (editor-state seam) and read it back.
    use zaroxi_application_workspace::ports::{SetEditorCursorRequest, EditorCursor, GetEditorStateRequest, GetActiveEditorDocumentRequest};
    let _ = orchestrator.set_editor_cursor(SetEditorCursorRequest {
        session_id: boot_res.session.session_id.clone(),
        buffer_id: open1_res.buffer_id.clone(),
        cursor: EditorCursor { line: 0, column: 0 },
    }).await.map_err(|e| e.to_string())?;

    match orchestrator.get_buffer_content(open1_res.buffer_id.clone()).await {
        Ok(Some(text)) => {
            let snippet = if text.len() > 200 { format!("{}...", &text[..200]) } else { text.clone() };
            println!("Harness: buffer content (snippet): {}", snippet);
        }
        Ok(None) => println!("Harness: buffer content: <empty>"),
        Err(e) => println!("Harness: failed to read buffer content: {}", e),
    }

    // Read and print the editor-state for the buffer.
    match orchestrator.get_editor_state(GetEditorStateRequest { session_id: boot_res.session.session_id.clone(), buffer_id: open1_res.buffer_id.clone() }).await {
        Ok(s) => {
            if let Some(state) = s.state {
                println!("Harness: editor state cursor = {}:{}", state.cursor.line, state.cursor.column);
            } else {
                println!("Harness: editor state: <none>");
            }
        }
        Err(e) => println!("Harness: failed to get editor state: {}", e),
    }

    // Phase 6: fetch the structured editor document/view model for the active buffer.
    match orchestrator.get_active_editor_document(GetActiveEditorDocumentRequest { session_id: boot_res.session.session_id.clone() }).await {
        Ok(resp) => {
            let doc = resp.document;
            println!("Harness: editor document summary for session {}:", boot_res.session.session_id);
            println!(" - buffer: {}", doc.buffer_id);
            println!(" - cursor: {}:{}", doc.cursor.line, doc.cursor.column);
            println!(" - selection: {:?}", doc.selection);
            println!(" - line_count: {}", doc.line_count);
            if let Some(ref line) = doc.current_line {
                let snippet = if line.len() > 200 { format!("{}...", &line[..200]) } else { line.to_owned() };
                println!(" - current line snippet: {}", snippet);
            }

            // Phase 7 (new): set a small default viewport so the application view seam
            // computes a predictable visible window before the interface adapter fetches it.
            let vp = zaroxi_application_workspace::ports::ViewportState { top_line: 1, window_height: 10, center_cursor: true };
            let _ = orchestrator.set_viewport_state(zaroxi_application_workspace::ports::SetViewportRequest {
                session_id: boot_res.session.session_id.clone(),
                buffer_id: doc.buffer_id.clone(),
                viewport: vp.clone(),
            }).await.map_err(|e| e.to_string())?;

            // Use a tiny desktop composition to fetch and hold the renderable window.
            // The composition reuses the existing Presenter and adapter seam and
            // records minimal shell metadata (session/workspace) for harness printing.
            let view_dyn: std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceView> = orchestrator.clone();
            // Also expose the orchestrator as a WorkspaceService for tiny shell actions.
            let service_dyn: std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceService> = orchestrator.clone();
            let mut composition = zaroxi_interface_desktop::DesktopComposition::new();
            // Delegate refreshing to the tiny interface action introduced in Phase 14.
            match zaroxi_interface_desktop::refresh_desktop(
                &mut composition,
                view_dyn.clone(),
                boot_res.session.session_id.clone(),
                Some(boot_res.session.workspace_id),
                Some(service_dyn.clone()),
            ).await {
                Ok(action_result) => {
                    println!("Harness: refresh action result: success={} refreshed={} message={:?}", action_result.success, action_result.refreshed, action_result.message);
                    if let Some(win) = composition.latest_window() {
                        println!("Harness: visible render window (composition): top_line={} total_lines={}", win.top_line, win.total_lines);
                        for rl in win.lines.iter() {
                            let mut out = String::new();
                            for sp in rl.spans.iter() {
                                match sp.kind {
                                    zaroxi_interface_desktop::view_adapter::InterfaceSpanKind::Normal => out.push_str(&sp.text),
                                    zaroxi_interface_desktop::view_adapter::InterfaceSpanKind::Selection => {
                                        out.push_str(&format!("[{}]", sp.text));
                                    }
                                    zaroxi_interface_desktop::view_adapter::InterfaceSpanKind::Cursor => {
                                        if sp.text.is_empty() {
                                            out.push_str("|^|");
                                        } else {
                                            out.push_str(&format!("|{}|", sp.text));
                                        }
                                    }
                                    zaroxi_interface_desktop::view_adapter::InterfaceSpanKind::SelectionCursor => {
                                        out.push_str(&format!("[|{}|]", sp.text));
                                    }
                                }
                            }
                            println!("{:4} | {}", rl.line_number, out);
                        }
                    } else {
                        println!("Harness: composition contained no window after refresh");
                    }

                    // Print the small, read-only metadata projection exposed by the composition.
                    if let Some(meta) = composition.latest_metadata() {
                        let active_buf = meta.active_buffer.as_ref().map(|b| b.as_str()).unwrap_or("<none>");
                        println!("Harness: composition metadata: session={:?} workspace={:?} active_buffer={} opened_count={}", meta.session_id, meta.workspace_id, active_buf, meta.opened_buffer_count);

                        // Print the tiny opened-buffers projection (explicit and shell-oriented).
                        if !meta.opened_buffers.is_empty() {
                            println!("Harness: opened buffers projection (count={}):", meta.opened_buffers.len());
                            for item in meta.opened_buffers.iter() {
                                let display = item.display.as_deref().unwrap_or("<no-display>");
                                let active_mark = if item.active { "*" } else { " " };
                                println!("  {} {} ({})", active_mark, item.buffer_id, display);
                            }
                        } else {
                            println!("Harness: opened buffers projection: <empty>");
                        }
                    } else {
                        println!("Harness: no composition metadata available");
                    }
                }
                Err(e) => {
                    println!("Harness: failed to refresh desktop composition: {}", e);
                }
            }

            // Demonstrate the new tiny shell action: move the cursor to document start and refresh composition.
            match zaroxi_interface_desktop::actions::move_cursor_to_start_and_refresh(
                &mut composition,
                service_dyn.clone(),
                view_dyn.clone(),
                boot_res.session.session_id.clone(),
                Some(boot_res.session.workspace_id),
            ).await {
                Ok(action_result) => {
                    println!("Harness: move-cursor action result: success={} refreshed={} message={:?}", action_result.success, action_result.refreshed, action_result.message);
                }
                Err(e) => {
                    println!("Harness: move cursor action failed: {}", e);
                }
            }

            // New tiny shell action: insert a blank line at start of active buffer and refresh composition.
            match zaroxi_interface_desktop::actions::insert_line_at_start_and_refresh(
                &mut composition,
                service_dyn.clone(),
                view_dyn.clone(),
                boot_res.session.session_id.clone(),
                Some(boot_res.session.workspace_id),
            ).await {
                Ok(action_result) => {
                    println!("Harness: insert-line-at-start action result: success={} refreshed={} message={:?}", action_result.success, action_result.refreshed, action_result.message);
                }
                Err(e) => {
                    println!("Harness: insert-line action failed: {}", e);
                }
            }
        }
        Err(e) => println!("Harness: failed to get editor document: {}", e),
    }

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
    println!("Harness: creating and saving checkpoint for session {}", boot_res.session.session_id);
    let save_res = orchestrator.save_checkpoint(SaveCheckpointRequest { session_id: boot_res.session.session_id.clone() }).await.map_err(|e| e.to_string())?;
    let location = save_res.location;
    println!("Harness: checkpoint persisted at location: {}", location);
 
    // Build fresh infra instances for restore target
    let repo2 = zaroxi_infrastructure_memory::InMemoryWorkspaceRepo::new();
    let repo2_dyn = zaroxi_infrastructure_memory::into_workspace_repo(repo2);
 
    let buffer_store2 = zaroxi_infrastructure_memory::InMemoryBufferStore::new();
    let buffer2_dyn = zaroxi_infrastructure_memory::into_buffer_store(buffer_store2);
 
    let history2 = zaroxi_infrastructure_memory::InMemoryHistoryStore::new();
    let history2_dyn = zaroxi_infrastructure_memory::into_history_store(history2);
 
    let ai2 = zaroxi_infrastructure_ai_mock::MockAiClient::new();
    let ai2_dyn = zaroxi_infrastructure_ai_mock::into_dyn(ai2);
 
    // Compose restore target with the same checkpoint durability adapter.
    let orchestrator2 = WorkspaceOrchestrator::new_with_history_and_durability(repo2_dyn, buffer2_dyn, ai2_dyn, history2_dyn, checkpoint_dyn.clone());
 
    println!("Harness: loading checkpoint into fresh orchestrator...");
    let load_res = orchestrator2.load_checkpoint(LoadCheckpointRequest { location: location.clone() }).await.map_err(|e| e.to_string())?;
    println!("Harness: loaded/restored session: {}", load_res.session.session_id);
 
    // Print restored snapshot for verification
    let snap_req2 = GetSessionSnapshotRequest { session_id: load_res.session.session_id.clone(), recent_limit: 10 };
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
