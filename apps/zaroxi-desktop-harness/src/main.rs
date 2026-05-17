use std::path::PathBuf;

use tokio;

use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, ListBuffersRequest, SetActiveBufferRequest, GetActiveBufferRequest, GetSessionSnapshotRequest,
    SaveCheckpointRequest, LoadCheckpointRequest,
};
use zaroxi_application_workspace::ports::{WorkspaceService, WorkspaceView};
use zaroxi_interface_desktop::projections::session_identity_line::SessionIdentityLine;

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
    // Prepare shared composition, view and service handles for reuse across scopes.
    // These are created once here so they are available both inside the `get_active_editor_document`
    // match arm and later in the function (e.g. after explain/other use-cases).
    let mut composition = zaroxi_interface_desktop::DesktopComposition::new();
    let view_dyn: std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceView> = orchestrator.clone();
    let service_dyn: std::sync::Arc<dyn zaroxi_application_workspace::ports::WorkspaceService> = orchestrator.clone();

    // Boot workspace (use-case)
    let boot_req = WorkspaceBootRequest { path: PathBuf::from("./sample-workspace") };
    let boot_res = orchestrator.boot_workspace(boot_req).await.map_err(|e| e.to_string())?;
    println!("Harness: opened workspace session: {}", boot_res.session.session_id);

    // SessionIdentityLine: intentionally not emitted until after the first composition refresh.
    // The composition/refresh is the authoritative source for shell-facing projections;
    // do not present session identity here (pre-refresh).

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
            // `view_dyn`, `service_dyn` and `composition` are created above and reused here.
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

                    // After the first successful refresh, the composition is authoritative and may
                    // contain DesktopMetadata/shell snapshot information. Emit a tiny
                    // shell-facing SessionIdentityLine only when the composition contains metadata.
                    if composition.latest_summary().is_some() {
                        let session_identity = SessionIdentityLine::new(
                            Some(boot_res.session.session_id.to_string()),
                            Some(boot_res.session.workspace_id.to_string()),
                            None,
                        );
                        println!("Harness: session identity: {}", session_identity.render());
                    } else {
                        println!("Harness: session identity: <none> (no composition metadata)");
                    }

                    // Print a tiny status bar line when present.
                    if let Some(sline) = composition.latest_status_bar_line() {
                        if let Some(s) = sline.sticky {
                            println!("Harness: status: {} [{}]", sline.text, s);
                        } else {
                            println!("Harness: status: {}", sline.text);
                        }
                    }
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

                    // Print the small viewport summary derived from the presenter's window.
                    if let Some(vs) = composition.latest_viewport_summary() {
                        println!(
                            "Harness: viewport summary: top_visible_line={} visible_line_count={} total_lines={} cursor_visible={} anchoring={:?}",
                            vs.top_visible_line,
                            vs.visible_line_count,
                            vs.total_lines,
                            vs.cursor_visible,
                            vs.anchoring
                        );
                    } else {
                        println!("Harness: viewport summary: <none>");
                    }

                    // Print a compact composition summary via the new summary accessor.
                    if let Some(summary) = composition.latest_summary() {
                        let active_buf = summary.active_buffer.as_ref().map(|b| b.as_str()).unwrap_or("<none>");
                        let rr = summary.refresh_reason.as_ref().map(|r| format!("{:?}", r)).unwrap_or_else(|| "None".to_string());
                        println!("Harness: composition summary: revision={} refresh_reason={} active_buffer={} status_present={}",
                            summary.revision, rr, active_buf, summary.status.is_some());
                        if let Some(status) = summary.status {
                            println!("Harness: composition status: render_window={} metadata={} active_buffer_details={} opened_buffers={} ai_projection={}",
                                status.has_render_window, status.has_metadata, status.has_active_buffer_details, status.has_opened_buffers, status.has_ai_projection);
                        } else {
                            println!("Harness: composition status: <none>");
                        }

                        // Print the tiny current shell context (Phase 29): compact summary useful to shells.
                        if let Some(ctx) = composition.latest_shell_context() {
                            let active = ctx.active_buffer.as_ref().map(|b| b.as_str()).unwrap_or("<none>");
                            println!("Harness: shell context: rev={} active_buffer={} active_display={:?} refresh_reason={:?} ai_present={}",
                                ctx.latest_revision, active, ctx.active_display, ctx.latest_refresh_reason, ctx.has_ai_projection);
                        } else {
                            println!("Harness: shell context: <none>");
                        }

                        if let Some(abd) = composition.latest_active_buffer_details() {
                            let disp = abd.display.as_deref().unwrap_or("<no-display>");
                            println!("Harness: active buffer details: id={} display={} line_count={}", abd.buffer_id, disp, abd.line_count);
                        } else {
                            println!("Harness: active buffer details: <none>");
                        }

                        // Print the tiny opened-buffers summary (shell-facing, derived projection).
                        let obs = composition.latest_opened_buffers_summary();
                        if obs.count > 0 {
                            println!("Harness: opened buffers summary (count={}):", obs.count);
                            for item in obs.items.iter() {
                                let display = item.display.as_deref().unwrap_or("<no-display>");
                                let active_mark = if item.active { "*" } else { " " };
                                println!("  {} {} ({}) lines={}", active_mark, item.buffer_id, display, item.line_count);
                            }
                        } else {
                            println!("Harness: opened buffers summary: <empty>");
                        }
                    } else {
                        println!("Harness: no composition metadata available");
                    }

                    // Small consistency report so outer harnesses can print a compact assertion of composition coherence.
                    let crep = composition.latest_consistency_report();
                    println!("Harness: composition consistency: overall_ok={} status_match={} active_matches_details={} active_in_opened={} presenter_matches_status={}",
                        crep.overall_ok,
                        crep.status_present_matches_summary,
                        crep.active_buffer_matches_details,
                        crep.active_buffer_in_opened_buffers,
                        crep.presenter_window_matches_status);

                    // Small, convenient shell snapshot print for harness diagnostics (Phase 38).
                    if let Some(ss) = composition.latest_shell_snapshot() {
                        println!("Harness: ShellSnapshot: rev={} active_buffer={:?} active_display={:?} cursor_line={:?} cursor_col={:?} selection_present={} viewport_top={} viewport_visible={} ai_present={} opened_count={}",
                            ss.context.latest_revision,
                            ss.context.active_buffer,
                            ss.context.active_display,
                            ss.active_document.as_ref().and_then(|d| d.cursor_line),
                            ss.active_document.as_ref().and_then(|d| d.cursor_column),
                            ss.active_document.as_ref().map(|d| d.selection_present).unwrap_or(false),
                            ss.viewport.as_ref().map(|v| v.top_visible_line).unwrap_or(0),
                            ss.viewport.as_ref().map(|v| v.visible_line_count).unwrap_or(0),
                            ss.ai_summary.as_ref().map(|a| a.present).unwrap_or(false),
                            ss.opened_buffers.count,
                        );
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
                    if let Some(rr) = composition.latest_refresh_reason() {
                        println!("Harness: composition refresh reason after move-cursor: {:?}", rr);
                    }

                    // Small read-only visible-text model print for harness visibility.
                    if let Some(tv) = zaroxi_interface_desktop::TextView::from_composition(&composition) {
                        println!("Harness: visible text (top_line={} total_lines={}):", tv.top_line, tv.total_lines);
                        for line in tv.lines_with_cursor_marker("|^|").iter() {
                            println!("    {}", line);
                        }
                    } else {
                        println!("Harness: no visible window available for TextView");
                    }

                    // Tiny read-only selection view: surface selection information for harnesses.
                    if let Some(sv) = zaroxi_interface_desktop::SelectionView::from_composition(&composition) {
                        println!(
                            "Harness: selection present: start={}:{} end={}:{} visible_in_window={}",
                            sv.start.line, sv.start.column, sv.end.line, sv.end.column, sv.visible_in_window
                        );
                    } else {
                        println!("Harness: no selection present");
                    }

                    // ActiveDocumentSummary: a tiny, read-only shell-facing projection with
                    // active buffer name/display, line count, cursor, selection presence, and a small snippet.
                    if let Some(ads) = composition.latest_active_document_summary() {
                        let buf_display = ads.buffer_id.as_ref().map(|b| b.as_str().to_string()).unwrap_or_else(|| "<none>".to_string());
                        println!("Harness: active document summary: buffer={} display={:?} lines={} cursor={:?}:{:?} selection_present={} snippet={:?}",
                            buf_display,
                            ads.display,
                            ads.line_count,
                            ads.cursor_line,
                            ads.cursor_column,
                            ads.selection_present,
                            ads.current_line_snippet,
                        );
                    } else {
                        println!("Harness: active document summary: <none>");
                    }
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
                    if let Some(rr) = composition.latest_refresh_reason() {
                        println!("Harness: composition refresh reason after insert-line: {:?}", rr);
                    }

                    // Show the visible-text view model after the insert action.
                    if let Some(tv) = zaroxi_interface_desktop::TextView::from_composition(&composition) {
                        println!("Harness: visible text after insert (top_line={} total_lines={}):", tv.top_line, tv.total_lines);
                        for line in tv.lines_with_cursor_marker("|^|").iter() {
                            println!("    {}", line);
                        }
                    } else {
                        println!("Harness: no visible window available for TextView after insert");
                    }
                }
                Err(e) => {
                    println!("Harness: insert-line action failed: {}", e);
                }
            }

            // New tiny convenience: refresh and return the shell context to the harness.
            match zaroxi_interface_desktop::actions::refresh_and_get_shell_context(
                &mut composition,
                view_dyn.clone(),
                boot_res.session.session_id.clone(),
                Some(boot_res.session.workspace_id),
                Some(service_dyn.clone()),
            ).await {
                Ok(res) => {
                    println!("Harness: refresh_and_get_shell_context: action.success={} refreshed={} message={:?}", res.action.success, res.action.refreshed, res.action.message);
                    if let Some(ctx) = res.context {
                        println!("Harness: shell context: rev={} active_buffer={:?} active_display={:?} refresh_reason={:?} has_ai={}",
                            ctx.latest_revision, ctx.active_buffer, ctx.active_display, ctx.latest_refresh_reason, ctx.has_ai_projection);
                    } else {
                        println!("Harness: shell context: <none>");
                    }
                }
                Err(e) => println!("Harness: refresh_and_get_shell_context failed: {}", e),
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

    // New tiny convenience action: set active buffer via service and refresh composition, returning shell context.
    match zaroxi_interface_desktop::actions::set_active_buffer_and_get_shell_context(
        &mut composition,
        service_dyn.clone(),
        view_dyn.clone(),
        boot_res.session.session_id.clone(),
        Some(boot_res.session.workspace_id),
        open2_res.buffer_id.clone(),
    ).await {
        Ok(res) => {
            println!("Harness: set_active_and_get_shell_context: action.success={} refreshed={} message={:?}", res.action.success, res.action.refreshed, res.action.message);
            if let Some(ctx) = res.context {
                println!("Harness: shell context after set_active: rev={} active_buffer={:?} active_display={:?} refresh_reason={:?}", ctx.latest_revision, ctx.active_buffer, ctx.active_display, ctx.latest_refresh_reason);
            }
        }
        Err(e) => println!("Harness: set_active_and_get_shell_context failed: {}", e),
    }

    // Confirm active buffer
    let get_active = GetActiveBufferRequest { session_id: boot_res.session.session_id.clone() };
    let active_res = orchestrator.get_active_buffer(get_active).await.map_err(|e| e.to_string())?;
    println!("Harness: current active buffer: {}", active_res.buffer_id);

    // Explain the active buffer (shorthand use-case)
    let explain_req = GetActiveBufferRequest { session_id: boot_res.session.session_id.clone() };
    let explain_res = orchestrator.explain_active_buffer(explain_req).await.map_err(|e| e.to_string())?;
    println!("Harness: explain result: {}", explain_res.result.message);

    // Refresh composition so the thin interface projection can pick up the latest AI projection
    // (we use the existing read pathway: DesktopComposition::refresh_with_service will consult
    // the WorkspaceService for recent events/commands and populate the AI projection).
    match zaroxi_interface_desktop::refresh_desktop(
        &mut composition,
        view_dyn.clone(),
        boot_res.session.session_id.clone(),
        Some(boot_res.session.workspace_id),
        Some(service_dyn.clone()),
    ).await {
        Ok(_action_result) => {
            // Prefer the tiny AiProjectionSummary for concise harness diagnostics.
            if let Some(ai_sum) = composition.latest_ai_projection_summary() {
                println!(
                    "Harness: AI projection summary: present={} kind={:?} target_buffer={:?} state={:?}",
                    ai_sum.present,
                    ai_sum.kind,
                    ai_sum.target_buffer,
                    ai_sum.state
                );
            } else {
                println!("Harness: AI projection: <none>");
            }
        }
        Err(e) => {
            println!("Harness: failed to refresh composition for AI projection: {}", e);
        }
    }
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

    // Tiny shell-facing LastEventLine: build from the most recent event (if any)
    {
        let last = recent_events.events.last();
        let last_line = zaroxi_interface_desktop::projections::last_event_line::summarize_last_event(last);
        println!("Harness: last event: {}", last_line.text);
    }

    // Tiny shell-facing last-command-line (when available via composition shell context).
    if let Some(ctx) = composition.latest_shell_context() {
        if let Some(lc) = ctx.last_command_line {
            println!("Harness: last command: {}", lc);
        }
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
