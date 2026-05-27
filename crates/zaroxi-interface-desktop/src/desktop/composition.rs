use std::sync::Arc;
use zaroxi_application_workspace::ports::{SessionId, WorkspaceView};
use zaroxi_kernel_types::Id;

/// Composition helper functions extracted from the parent `desktop` module.
/// These functions are intentionally implemented in a submodule so that the
/// orchestration/facade in `desktop.rs` remains compact. They are small
/// wrappers that mutate or read the parent `DesktopComposition` via the
/// `super::DesktopComposition` type and preserve behavior exactly.

pub async fn refresh_with_service(
    comp: &mut super::DesktopComposition,
    view: Arc<dyn WorkspaceView>,
    session_id: SessionId,
    workspace_id: Option<Id>,
    service: Option<Arc<dyn crate::ports::WorkspaceService>>,
) -> Result<(), String> {
    // Capture previous presenter snapshot to detect content changes.
    let prev_presenter_snapshot = comp.presenter.latest();

    // 1) Refresh presenter snapshot (reuses adapter seam and existing projection).
    comp.presenter.refresh(view.clone(), session_id.clone()).await?;

    // Capture the new presenter snapshot so we can detect buffer content changes
    // (shell-facing, presentation-only signal).
    let new_presenter_snapshot = comp.presenter.latest();

    // 2) Attempt to read the active editor document via the WorkspaceView seam.
    let active_buf_opt = match view
        .get_active_editor_document(crate::ports::GetActiveEditorDocumentRequest {
            session_id: session_id.clone(),
        })
        .await
    {
        Ok(resp) => Some(resp.document.buffer_id.clone()),
        Err(_) => None,
    };

    // 2b) Attempt to obtain a direct visible-lines projection from the WorkspaceView.
    // When available, we capture a small, stable VisibleWindowBasic projection that
    // strengthens the editor viewport semantics for shells (preferred over transcripts).
    // Note: GetVisibleLinesRequest requires a buffer_id; only call the port when we
    // have an active buffer id available from the earlier active_buf_opt read.
    let visible_window_opt: Option<super::VisibleWindowBasic> =
        if let Some(bid) = active_buf_opt.clone() {
            match view
                .get_visible_lines(crate::ports::GetVisibleLinesRequest {
                    session_id: session_id.clone(),
                    buffer_id: bid.clone(),
                })
                .await
            {
                Ok(resp) => {
                    // Build a tiny basic projection decoupled from presenter view types.
                    let mut lines_vec: Vec<String> = Vec::with_capacity(resp.window.lines.len());
                    let mut cursor_line: Option<usize> = None;
                    let mut cursor_column: Option<usize> = None;
                    let mut selection_present: bool = false;
                    for vl in resp.window.lines.iter() {
                        lines_vec.push(vl.text.clone());
                        if vl.is_cursor_line {
                            cursor_line = Some(vl.line_number as usize);
                            if let Some(col) = vl.cursor_column {
                                cursor_column = Some(col as usize);
                            }
                        }
                        if vl.selection_intersects {
                            selection_present = true;
                        }
                    }
                    Some(super::VisibleWindowBasic {
                        top_line: resp.window.top_line as usize,
                        total_lines: resp.window.total_lines as usize,
                        lines: lines_vec,
                        cursor_line,
                        cursor_column,
                        selection_present,
                    })
                }
                Err(_) => None,
            }
        } else {
            None
        };

    // Prepare default conservative projection values.
    let mut opened_count = if active_buf_opt.is_some() { 1 } else { 0 };
    let mut opened_list: Vec<super::OpenedBufferItem> = Vec::new();

    // 3) If a WorkspaceService is provided, attempt to obtain the authoritative opened buffer list.
    if let Some(svc) = &service {
        // Request list of opened buffers for the session (application-owned use-case).
        match svc
            .list_open_buffers(crate::ports::ListBuffersRequest { session_id: session_id.clone() })
            .await
        {
            Ok(list_res) => {
                opened_count = list_res.buffer_ids.len();
                // Build small projection items. Use path/display when available.
                for bid in list_res.buffer_ids.iter() {
                    let display = bid.path().map(|p| p.to_string_lossy().to_string());
                    let is_active =
                        list_res.active_buffer.as_ref().map(|ab| ab == bid).unwrap_or(false);
                    opened_list.push(super::OpenedBufferItem {
                        buffer_id: bid.clone(),
                        display,
                        active: is_active,
                    });
                }

                // If the service reports an active_buffer that is not present in the
                // returned buffer_ids, include it in the projection and mark it active.
                // This covers lightweight service implementations that may set active
                // without also adding the buffer to their opened list (test doubles).
                if let Some(active_bid) = list_res.active_buffer.clone() {
                    if !list_res.buffer_ids.iter().any(|b| b == &active_bid) {
                        let display = active_bid.path().map(|p| p.to_string_lossy().to_string());
                        opened_list.push(super::OpenedBufferItem {
                            buffer_id: active_bid.clone(),
                            display,
                            active: true,
                        });
                        opened_count = opened_count.saturating_add(1);
                    }
                }
            }
            Err(_) => {
                // On error, fall back to conservative single-item projection when active exists.
                if let Some(bid) = active_buf_opt.clone() {
                    let display = bid.path().map(|p| p.to_string_lossy().to_string());
                    opened_list.push(super::OpenedBufferItem {
                        buffer_id: bid.clone(),
                        display,
                        active: true,
                    });
                }
            }
        }
    } else {
        // No service provided: keep conservative projection (only active buffer when present).
        if let Some(bid) = active_buf_opt.clone() {
            let display = bid.path().map(|p| p.to_string_lossy().to_string());
            opened_list.push(super::OpenedBufferItem {
                buffer_id: bid.clone(),
                display,
                active: true,
            });
        }
    }

    // 4) Update composition metadata and simple recorded ids.
    // Compute authoritative active buffer: prefer service-provided opened-buffer active marker when present.
    // `opened_list` is already built above and is authoritative when `service` was provided.
    let current_opened_active = opened_list.iter().find(|i| i.active).map(|i| i.buffer_id.clone());

    // Determine authoritative active buffer for metadata and details: service (opened list) wins, else presenter-derived active.
    let authoritative_active = current_opened_active.clone().or(active_buf_opt.clone());

    // Compute a tiny active-buffer details projection using the authoritative active buffer.
    let active_buffer_details: Option<super::ActiveBufferDetails> =
        if let Some(bid) = authoritative_active.clone() {
            // Prefer the display label from the opened_buffers projection if available.
            let display_label = opened_list
                .iter()
                .find(|i| i.buffer_id == bid)
                .and_then(|i| i.display.clone())
                .or_else(|| bid.path().map(|p| p.to_string_lossy().to_string()));

            // Use visible-window projection when present to obtain a reliable line_count metric,
            // otherwise fall back to the presenter's latest snapshot.
            let line_count = if let Some(vw) = &visible_window_opt {
                vw.total_lines
            } else {
                comp.presenter.latest().map(|w| w.total_lines).unwrap_or(0usize)
            };

            Some(super::ActiveBufferDetails {
                buffer_id: bid.clone(),
                display: display_label,
                line_count,
            })
        } else {
            None
        };

    // Attempt to read recent events to build a tiny AI projection when a WorkspaceService is available.
    // We intentionally use the existing `get_recent_events` port (read-only) and only surface
    // the most recent ExplainExecuted event if present. This keeps composition purely read-only
    // and avoids duplicating AI orchestration logic.
    let mut ai_proj: Option<super::AiProjection> = None;
    // Tiny shell-facing last-command-line string (computed below when service present).
    let mut last_command_line: Option<String> = None;

    if let Some(svc) = &service {
        if let Ok(ev_res) = svc
            .get_recent_events(crate::ports::GetRecentEventsRequest {
                session_id: session_id.clone(),
                limit: 20,
            })
            .await
        {
            // Iterate from newest to oldest and pick the first ExplainExecuted we find.
            for ev in ev_res.events.iter().rev() {
                if let crate::ports::WorkspaceEventKind::ExplainExecuted { buffer_id, result } =
                    &ev.kind
                {
                    ai_proj = Some(super::AiProjection {
                        kind: Some("ExplainExecuted".to_string()),
                        result: Some(result.clone()),
                        target_buffer: Some(buffer_id.clone()),
                        proposal_text: None,
                        state: Some(super::AiState::Idle),
                    });
                    break;
                }
            }
        }

        // Attempt to obtain the most recent command (limit=1) and render a tiny one-line string.
        if let Ok(cmd_res) = svc
            .get_recent_commands(crate::ports::GetRecentCommandsRequest {
                session_id: session_id.clone(),
                limit: 1,
            })
            .await
        {
            if let Some(rec) = cmd_res.commands.last() {
                let kind_name = super::command_kind_short_name(&rec.kind);
                let suffix = if rec.success { " ✓" } else { " ✗" };
                last_command_line = Some(format!("{}{}", kind_name, suffix));
            }
        }
    }

    // --- Refresh reason detection ---
    //
    // Compute a small set of lightweight change-detections that the shell cares about.
    // Preference order:
    // 1) Explicit pending reason set by caller (actions).
    // 2) AI projection changed (new explain executed result became available).
    // 3) First-ever refresh should be reported as InitialLoad (stable shell expectation).
    // 4) Active buffer changed (shell cares which buffer is active).
    //    * When a WorkspaceService was provided prefer comparing the opened-buffer
    //      projection's active marker (service authoritative for opened buffers).
    //    * Otherwise fall back to comparing the presenter's active buffer (view).
    // 5) Buffer content changed as observed by the presenter snapshot (BufferUpdated).
    // 6) Generic RefreshAction otherwise.
    //
    // Note: comparisons are tiny and presentation-only (strings / buffer ids); we avoid
    // introducing an event stream or mirroring application internals.
    let prev_active = comp.metadata.as_ref().and_then(|m| m.active_buffer.clone());
    let prev_opened_active = comp
        .metadata
        .as_ref()
        .and_then(|m| m.opened_buffers.iter().find(|i| i.active).map(|i| i.buffer_id.clone()));
    let prev_ai_result = comp
        .metadata
        .as_ref()
        .and_then(|m| m.ai_projection.as_ref().and_then(|a| a.result.clone()));

    // signature helper for presenter snapshots (concatenate span texts)
    let make_presenter_sig = |opt: Option<super::InterfaceRenderableWindow>| -> String {
        if let Some(w) = opt {
            let mut out = String::new();
            for line in w.lines.iter() {
                for sp in line.spans.iter() {
                    out.push_str(&sp.text);
                    out.push('|');
                }
                out.push('\n');
            }
            out
        } else {
            String::new()
        }
    };

    let prev_sig = make_presenter_sig(prev_presenter_snapshot.clone());
    let new_sig = make_presenter_sig(new_presenter_snapshot.clone());
    let new_ai_result = ai_proj.as_ref().and_then(|a| a.result.clone());

    // If the composition consulted a WorkspaceService, prefer the service-provided
    // opened-buffer active marker as the source of truth for "ActiveBufferChanged".
    let current_opened_active = opened_list.iter().find(|i| i.active).map(|i| i.buffer_id.clone());

    let reason = if let Some(pending) = comp.pending_refresh_reason.take() {
        // 1) Explicit caller-supplied reason wins.
        pending
    } else if prev_ai_result != new_ai_result {
        // 2) AI projection updates take precedence.
        super::RefreshReason::AiProjectionUpdated
    } else if comp.session_id.is_none() {
        // 3) If this composition has never been refreshed before, treat this as InitialLoad.
        //    This aligns the status bar semantics with shell/harness expectations for the
        //    first refresh lifecycle event.
        super::RefreshReason::InitialLoad
    } else if current_opened_active.is_some() || prev_opened_active.is_some() {
        // 4) When we have an opened-buffer projection (service used previously or now),
        //    compare the previous opened-active against the current opened-active.
        if prev_opened_active != current_opened_active {
            super::RefreshReason::ActiveBufferChanged
        } else if prev_active != active_buf_opt {
            // Fallback: also consider presenter-level active buffer changes if they differ.
            super::RefreshReason::ActiveBufferChanged
        } else if prev_sig != new_sig {
            super::RefreshReason::BufferUpdated
        } else {
            super::RefreshReason::RefreshAction
        }
    } else if prev_active != active_buf_opt {
        super::RefreshReason::ActiveBufferChanged
    } else if prev_sig != new_sig {
        super::RefreshReason::BufferUpdated
    } else {
        super::RefreshReason::RefreshAction
    };

    comp.session_id = Some(session_id.clone());
    comp.workspace_id = workspace_id;

    // Compute metadata and status snapshots derived from the refresh work above.
    let metadata = super::DesktopMetadata {
        session_id: Some(session_id),
        workspace_id: comp.workspace_id.clone(),
        // Prefer service-provided opened-buffer active marker when present; fall back to presenter's active buffer.
        active_buffer: authoritative_active.clone(),
        opened_buffer_count: opened_count,
        opened_buffers: opened_list.clone(),
        active_buffer_details: active_buffer_details.clone(),
        ai_projection: ai_proj.clone(),
        // Surface visible-window projection when we could obtain one from the WorkspaceView.
        visible_window: visible_window_opt.clone(),
        last_command_line: last_command_line.clone(),
        refresh_reason: Some(reason),
    };

    // Status summarizes availability of key projections: presenter window, metadata, active details, opened list, AI projection.
    let status = super::DesktopStatus {
        has_render_window: comp.presenter.latest().is_some(),
        has_metadata: true,
        has_active_buffer_details: active_buffer_details.is_some(),
        has_opened_buffers: !metadata.opened_buffers.is_empty(),
        has_ai_projection: ai_proj.is_some(),
    };

    comp.metadata = Some(metadata);
    comp.status = Some(status);

    // Increment the small, shell-facing revision counter on each successful refresh.
    comp.revision = comp.revision.saturating_add(1);

    Ok(())
}

pub fn latest_active_document_summary(
    comp: &super::DesktopComposition,
) -> Option<super::ActiveDocumentSummary> {
    let meta = comp.metadata.as_ref()?;
    let abd = meta.active_buffer_details.clone()?;

    // Prefer a direct visible-window projection from WorkspaceView when available;
    // otherwise fall back to the presenter's latest renderable window.
    let vw_opt = comp.metadata.as_ref().and_then(|m| m.visible_window.clone());
    let mut cursor_line: Option<usize> = None;
    let mut cursor_column: Option<usize> = None;
    let mut selection_present = false;
    let mut current_line_snippet: Option<String> = None;

    if let Some(vw) = vw_opt {
        // Use the basic visible-window projection to fill cursor/selection/snippet.
        cursor_line = vw.cursor_line;
        cursor_column = vw.cursor_column;
        selection_present = vw.selection_present;

        // Determine a reasonable current-line snippet: prefer cursor line, else top_line.
        let snippet_line_no = cursor_line.unwrap_or(vw.top_line);
        // Convert snippet_line_no into an index in vw.lines (lines stored from top_line).
        if snippet_line_no >= vw.top_line {
            let idx = snippet_line_no.saturating_sub(vw.top_line);
            if let Some(line_text) = vw.lines.get(idx) {
                let snippet: String = line_text.chars().take(120).collect();
                current_line_snippet = Some(snippet);
            }
        }
    } else {
        // Fallback: inspect the presenter's InterfaceRenderableWindow spans as before.
        let win_opt = comp.presenter.latest();
        if let Some(win) = win_opt {
            // Scan spans to find a cursor or selection.
            for line in win.lines.iter() {
                for sp in line.spans.iter() {
                    match sp.kind {
                        crate::view_adapter::InterfaceSpanKind::SelectionCursor
                        | crate::view_adapter::InterfaceSpanKind::Cursor => {
                            cursor_line = Some(line.line_number);
                            cursor_column = Some(sp.start_col);
                        }
                        crate::view_adapter::InterfaceSpanKind::Selection => {
                            selection_present = true;
                        }
                        _ => {}
                    }
                    // stop early if we found both
                    if cursor_line.is_some() && selection_present {
                        break;
                    }
                }
                if cursor_line.is_some() && selection_present {
                    break;
                }
            }

            // If we didn't detect selection while scanning for cursor, do a secondary lightweight check.
            if !selection_present {
                'outer2: for line in win.lines.iter() {
                    for sp in line.spans.iter() {
                        if let crate::view_adapter::InterfaceSpanKind::Selection = sp.kind {
                            selection_present = true;
                            break 'outer2;
                        }
                    }
                }
            }

            // Determine a reasonable current-line snippet: prefer cursor line, else top_line.
            // NOTE: Only include user-facing text in the snippet. Exclude presenter marker spans
            // (cursor/selection/debug) so the produced snippet is clean and free of inline
            // debug markers like "|^|" or "|/|/" that some renderers may inject.
            let snippet_line_no = cursor_line.unwrap_or(win.top_line);
            if let Some(l) = win.lines.iter().find(|l| l.line_number == snippet_line_no) {
                let mut s = String::new();
                // Only include "text" spans in the plain snippet. Exclude cursor/selection/debug spans
                // so that the resulting snippet remains clean and user-facing.
                for sp in l.spans.iter() {
                    match sp.kind {
                        crate::view_adapter::InterfaceSpanKind::SelectionCursor
                        | crate::view_adapter::InterfaceSpanKind::Cursor
                        | crate::view_adapter::InterfaceSpanKind::Selection => {
                            // skip marker spans from presenter (cursor/selection); these are surfaced
                            // separately via cursor_line/cursor_column/selection_present.
                        }
                        _ => {
                            s.push_str(&sp.text);
                        }
                    }
                }
                // Truncate to 120 Unicode scalars for compactness.
                let snippet: String = s.chars().take(120).collect();
                current_line_snippet = Some(snippet);
            }
        }
    }

    Some(super::ActiveDocumentSummary {
        buffer_id: meta.active_buffer.clone(),
        display: abd.display,
        line_count: abd.line_count,
        cursor_line,
        cursor_column,
        selection_present,
        current_line_snippet,
    })
}

pub fn latest_opened_buffers_summary(
    comp: &super::DesktopComposition,
) -> super::OpenedBuffersSummary {
    if let Some(meta) = &comp.metadata {
        // Build per-item summaries. Prefer line_count from active_buffer_details when it matches.
        let mut items: Vec<super::OpenedBufferItemSummary> =
            Vec::with_capacity(meta.opened_buffers.len());
        for it in meta.opened_buffers.iter() {
            // Try to obtain line_count from active_buffer_details when it matches the buffer id.
            let mut line_count: usize = 0;
            if let Some(abd) = &meta.active_buffer_details {
                if abd.buffer_id == it.buffer_id {
                    line_count = abd.line_count;
                }
            }
            items.push(super::OpenedBufferItemSummary {
                buffer_id: it.buffer_id.clone(),
                display: it.display.clone(),
                line_count,
                active: it.active,
            });
        }
        super::OpenedBuffersSummary {
            count: meta.opened_buffer_count,
            items,
            active: meta.active_buffer.clone(),
        }
    } else {
        super::OpenedBuffersSummary { count: 0, items: Vec::new(), active: None }
    }
}

pub fn latest_shell_context(comp: &super::DesktopComposition) -> Option<super::ShellContext> {
    // Mirror latest_summary presence semantics: require at least one refresh to return a context.
    if comp.revision == 0 && comp.metadata.is_none() && comp.status.is_none() {
        return None;
    }

    // Determine active_display: prefer active_buffer_details.display, fall back to opened_buffers item display.
    let active_display = comp.metadata.as_ref().and_then(|m| {
        m.active_buffer_details
            .as_ref()
            .and_then(|d| d.display.clone())
            .or_else(|| m.opened_buffers.iter().find(|i| i.active).and_then(|i| i.display.clone()))
    });

    let has_ai = comp.metadata.as_ref().and_then(|m| m.ai_projection.as_ref()).is_some();

    Some(super::ShellContext {
        active_buffer: comp.metadata.as_ref().and_then(|m| m.active_buffer.clone()),
        active_display,
        latest_revision: comp.revision,
        latest_refresh_reason: comp.metadata.as_ref().and_then(|m| m.refresh_reason.clone()),
        has_ai_projection: has_ai,
        last_command_line: comp.metadata.as_ref().and_then(|m| m.last_command_line.clone()),
    })
}

// ----------------------------
// AI edit/apply helpers (Phase 10)
// ----------------------------

/// Request an AI edit proposal for the currently active buffer in `session_id`.
///
/// This consults the provided `view` to obtain the active editor document (content),
/// calls the deterministic mock AI provider, and stores a proposal into the composition
/// metadata.ai_projection slot with state=Proposed. It also sets a small status message.
pub async fn request_ai_edit_active(
    comp: &mut super::DesktopComposition,
    view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
    session_id: crate::ports::SessionId,
    service: Option<std::sync::Arc<dyn crate::ports::WorkspaceService>>,
) -> Result<(), String> {
    // Attempt to read the active editor document via the view seam.
    let doc_res = view
        .get_active_editor_document(crate::ports::GetActiveEditorDocumentRequest {
            session_id: session_id.clone(),
        })
        .await;

    let document = match doc_res {
        Ok(r) => r.document,
        Err(_) => {
            return Err("failed to read active document".to_string());
        }
    };

    let target_buffer = document.buffer_id.clone();

    // Call deterministic mock AI provider to produce a proposal.
    let provider = crate::ai::MockAiProvider::new();
    let proposal_text = provider.propose_edit(target_buffer.clone(), document.content.clone()).await;

    // Ensure metadata exists and store the ai projection with proposed state.
    if comp.metadata.is_none() {
        comp.metadata = Some(super::DesktopMetadata {
            session_id: Some(session_id.clone()),
            workspace_id: comp.workspace_id.clone(),
            active_buffer: Some(target_buffer.clone()),
            opened_buffer_count: 0,
            opened_buffers: Vec::new(),
            active_buffer_details: None,
            ai_projection: None,
            visible_window: None,
            last_command_line: None,
            refresh_reason: None,
        });
    }

    if let Some(md) = comp.metadata.as_mut() {
        md.ai_projection = Some(super::AiProjection {
            kind: Some("Edit".to_string()),
            result: Some("AI edit proposed".to_string()),
            target_buffer: Some(target_buffer.clone()),
            proposal_text: Some(proposal_text.clone()),
            state: Some(super::AiState::Proposed),
        });
    }

    comp.set_status_message("AI edit proposed".to_string());

    // Optionally, if a service is provided, we may record the proposal in history via
    // get_recent_commands or a custom command; for Phase 10 we keep it local.
    let _ = service;

    Ok(())
}

/// Apply the currently proposed AI edit for the active buffer.
///
/// Preconditions:
/// - comp.metadata.ai_projection must be present with state=Proposed and contain a proposal_text.
/// - `service` must be provided and implement the update_buffer port.
///
/// This function applies the proposal using the normal WorkspaceService.update_buffer path,
/// sets the ai_projection.state to Applied on success, sets a user-visible status message,
/// and refreshes the composition so the new content is visible.
pub async fn apply_ai_edit_active(
    comp: &mut super::DesktopComposition,
    view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    service: std::sync::Arc<dyn crate::ports::WorkspaceService>,
) -> Result<(), String> {
    let md = comp
        .metadata
        .as_mut()
        .ok_or_else(|| "no composition metadata present".to_string())?;

    let ai = md.ai_projection.as_ref().ok_or_else(|| "no ai proposal present".to_string())?;

    if ai.state != Some(super::AiState::Proposed) {
        return Err("ai proposal not in proposed state".to_string());
    }

    let proposal = ai
        .proposal_text
        .clone()
        .ok_or_else(|| "ai proposal text missing".to_string())?;

    let buffer_id = ai
        .target_buffer
        .clone()
        .ok_or_else(|| "ai target buffer missing".to_string())?;

    // Build update request and call the WorkspaceService update_buffer port.
    let update_req = crate::ports::UpdateBufferRequest {
        session_id: session_id.clone(),
        buffer_id: buffer_id.clone(),
        new_content: proposal.clone(),
    };

    match service.update_buffer(update_req).await {
        Ok(resp) => {
            if resp.ok {
                // Mark applied in the projection.
                if let Some(md_mut) = comp.metadata.as_mut() {
                    if let Some(ai_mut) = md_mut.ai_projection.as_mut() {
                        ai_mut.state = Some(super::AiState::Applied);
                        ai_mut.result = Some("AI edit applied".to_string());
                    }
                }

                comp.set_status_message("AI edit applied".to_string());

                // Refresh composition so the new content is visible. We pass the service
                // back into the refresh so opened-buffer lists and authoritative active buffer
                // info may be read by the composition helpers.
                comp.refresh_with_service(view, session_id, workspace_id, Some(service.clone()))
                    .await?;
                Ok(())
            } else {
                Err("workspace update reported failure".to_string())
            }
        }
        Err(e) => Err(format!("update_buffer failed: {}", e)),
    }
}

/// Cancel and clear any pending AI proposal in the composition without mutating buffers.
pub fn cancel_ai_edit_active(comp: &mut super::DesktopComposition) {
    if let Some(md) = comp.metadata.as_mut() {
        md.ai_projection = None;
    }
    comp.set_status_message("AI proposal cancelled".to_string());
}
