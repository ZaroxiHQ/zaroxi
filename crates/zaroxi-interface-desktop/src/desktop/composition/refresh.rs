/*!
Refresh/build/update logic for DesktopComposition.

This module contains the implementation of refresh_with_service and the small
AI request/apply/cancel helpers that mutate the composition metadata. The
implementation is a direct migration from the original composition module and
preserves behaviour exactly (modulo file splitting).
*/

use std::sync::Arc;

use zaroxi_application_workspace::ports::{SessionId, WorkspaceView};
use zaroxi_kernel_types::Id;

/// Refresh the DesktopComposition using a WorkspaceView and optional WorkspaceService.
///
/// This is the migrated implementation from the original monolithic composition
/// module. It intentionally preserves behavior and heuristics.
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

    // signature helper for presenter snapshots (concatenate span texts) - compute early for lightweight change detection
    let make_presenter_sig =
        |opt: Option<crate::view_adapter::InterfaceRenderableWindow>| -> String {
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

    // Compute presenter signatures early so we can decide whether to call potentially-expensive
    // service ports (recent events / recent commands). This narrows recomputation and avoids
    // extra service calls for trivial refreshes that did not change the presenter output.
    let prev_sig = make_presenter_sig(prev_presenter_snapshot.clone());
    let new_sig = make_presenter_sig(new_presenter_snapshot.clone());
    // Lightweight previous active buffer read (presenter/service authoritative resolution happens later).
    let prev_active = comp.metadata.as_ref().and_then(|m| m.active_buffer.clone());
    // Detect whether we previously had an AI projection. If none existed before, we must consult
    // the service events during refresh because an ExplainExecuted may have been produced by the
    // application/orchestrator and would not necessarily change presenter output. This is a
    // conservative, narrow addition that only forces event/command queries when we previously
    // had no AI projection recorded in composition metadata.
    let prev_has_ai = comp.metadata.as_ref().and_then(|m| m.ai_projection.as_ref()).is_some();

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
    let visible_window_opt: Option<crate::desktop::projections::VisibleWindowBasic> =
        if let Some(bid) = active_buf_opt.clone() {
            // Lightweight fast-path: if presenter output signature did not change and we already
            // have a visible_window stored in metadata for the same active buffer, reuse it and
            // avoid calling the view port. This preserves exact semantics while avoiding a
            // potentially-expensive call on cursor-only or trivial updates.
            let reuse_prev = prev_sig == new_sig
                && comp.metadata.as_ref().and_then(|m| m.visible_window.clone()).is_some()
                && comp.metadata.as_ref().and_then(|m| m.active_buffer.clone())
                    == Some(bid.clone());

            if reuse_prev {
                comp.metadata.as_ref().and_then(|m| m.visible_window.clone())
            } else {
                match view
                    .get_visible_lines(crate::ports::GetVisibleLinesRequest {
                        session_id: session_id.clone(),
                        buffer_id: bid.clone(),
                    })
                    .await
                {
                    Ok(resp) => {
                        // Build a tiny basic projection decoupled from presenter view types.
                        let mut lines_vec: Vec<String> =
                            Vec::with_capacity(resp.window.lines.len());
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
                        Some(crate::desktop::projections::VisibleWindowBasic {
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
            }
        } else {
            None
        };

    // Sync editor viewport line count to the workspace's ViewportState
    // so centering and scroll calculations use the correct visible height.
    // Only updates window_height when it changes (does not reset top_line).
    if let (Some(svc), Some(bid)) = (&service, active_buf_opt.as_ref()) {
        if let Some(window_height) =
            comp.metadata.as_ref().and_then(|m| m.editor_viewport_line_count)
        {
            if window_height > 0 {
                let prev = comp.metadata.as_ref().and_then(|m| m.last_synced_window_height);
                if prev != Some(window_height) {
                    let _ = svc
                        .set_viewport_state(crate::ports::SetViewportRequest {
                            session_id: session_id.clone(),
                            buffer_id: bid.clone(),
                            viewport: crate::ports::ViewportState {
                                top_line: 1,
                                window_height,
                                center_cursor: prev.is_none(),
                            },
                        })
                        .await;
                    if let Some(ref mut meta) = comp.metadata {
                        meta.last_synced_window_height = Some(window_height);
                    }
                }
            }
        }
    }

    // Process pending vertical scroll delta: call scroll_viewport on the workspace
    // so the ViewportState.top_line stays in sync with the GUI interaction model.
    {
        let delta = comp.pending_scroll_lines;
        comp.pending_scroll_lines = 0;
        if delta != 0 {
            if let (Some(svc), Some(bid)) = (&service, active_buf_opt.as_ref()) {
                let _ = svc
                    .scroll_viewport(crate::ports::ScrollViewportRequest {
                        session_id: session_id.clone(),
                        buffer_id: bid.clone(),
                        delta_lines: delta,
                    })
                    .await;
            }
        }
    }

    // Process pending horizontal scroll delta: update the editor viewport's
    // horizontal_offset_px. This is a renderer-local offset (no workspace state).
    {
        let hdelta = comp.pending_hscroll_px;
        comp.pending_hscroll_px = 0.0;
        if hdelta != 0.0 {
            // Store horizontal offset in metadata so app.rs can read it
            // and apply to UiBlock.content_offset_x
            if let Some(ref mut meta) = comp.metadata {
                let current = meta.editor_horizontal_offset_px.unwrap_or(0.0);
                meta.editor_horizontal_offset_px = Some((current + hdelta).max(0.0));
            }
        }
    }

    // Prepare default conservative projection values.
    let mut opened_list: Vec<super::OpenedBufferItem> = Vec::new();

    // Persistent closed-buffer markers. Never consumed — the close handler
    // pushes to this list and the refresh filters against it every cycle.
    // Entries are cleared only when the workspace service no longer lists
    // the buffer (in which case the entry is harmless but stale).
    let pending_removals: Vec<String> = comp.pending_removed_buffer_ids.clone();

    // 3) If a WorkspaceService is provided, attempt to obtain the authoritative opened buffer list.
    if let Some(svc) = &service {
        // Request list of opened buffers for the session (application-owned use-case).
        match svc
            .list_open_buffers(crate::ports::ListBuffersRequest { session_id: session_id.clone() })
            .await
        {
            Ok(list_res) => {
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

    // Filter out buffers that were locally closed since the last refresh.
    // The workspace service may still report them, but the user explicitly
    // removed them so they must not reappear.
    if !pending_removals.is_empty() {
        let mut i = 0;
        while i < opened_list.len() {
            if pending_removals.iter().any(|bid| opened_list[i].buffer_id.to_string() == *bid) {
                opened_list.remove(i);
            } else {
                i += 1;
            }
        }
    }

    // Canonical dedupe: never allow two items with the same buffer_id
    // or the same display (file path) in the opened list. First-chosen
    // entry wins; duplicates are silently dropped.
    {
        let mut seen_bid = std::collections::HashSet::new();
        let mut seen_display = std::collections::HashSet::<String>::new();
        let mut i = 0;
        while i < opened_list.len() {
            let bid_key = opened_list[i].buffer_id.to_string();
            let disp_key = opened_list[i].display.clone();
            let is_dup = seen_bid.contains(&bid_key)
                || disp_key.as_deref().is_some_and(|d| seen_display.contains(d));
            if is_dup {
                opened_list.remove(i);
            } else {
                seen_bid.insert(bid_key);
                if let Some(d) = disp_key {
                    seen_display.insert(d);
                }
                i += 1;
            }
        }
    }

    // Merge directly-added buffers (large files loaded via PieceTable)
    // back into the opened list.  These buffers were registered through
    // `add_opened_buffer_direct` and are unknown to the workspace service,
    // so the service-issued rebuild above drops them.  Re-adding them
    // ensures stable tabs for large files.
    // Preserve the active status from the current metadata's active_buffer
    // so large-file tabs stay highlighted across refreshes.
    let direct_active = comp
        .metadata
        .as_ref()
        .and_then(|m| m.active_buffer.clone())
        .filter(|b| comp.direct_buffer_ids.contains(b));
    for direct_bid in &comp.direct_buffer_ids {
        // Membership must be tested by CANONICAL path, not raw string
        // equality: a path opened via the workspace service and later
        // registered as a direct buffer can carry two `buf:`/plain id
        // forms for the same file.  Comparing raw strings let the second
        // form slip past this guard and produced a duplicate tab.
        let already =
            opened_list.iter().any(|it| super::same_opened_document(&it.buffer_id, direct_bid));
        if !already {
            let display = direct_bid.path().map(|p| p.to_string_lossy().to_string());
            let is_active = direct_active.as_ref().map(|a| a == direct_bid).unwrap_or(false);
            opened_list.push(super::OpenedBufferItem {
                buffer_id: direct_bid.clone(),
                display,
                active: is_active,
            });
        }
    }

    // Hard invariant: one canonical path == one opened_buffers entry.
    // Run the canonical dedupe AFTER every source (service list, direct
    // buffers, fallback) has contributed, so a duplicate introduced by the
    // direct-buffer merge above cannot survive into the projection.
    super::dedupe_opened_buffers(&mut opened_list);

    let opened_count = opened_list.len();

    // 4) Update composition metadata and simple recorded ids.
    // Compute authoritative active buffer: prefer the direct-buffer active
    // (large files), then service-provided opened-buffer active marker, then
    // presenter-derived active.
    let current_opened_active = opened_list.iter().find(|i| i.active).map(|i| i.buffer_id.clone());

    // Determine authoritative active buffer for metadata and details.
    // Direct buffers take priority over service-derived buffers, preserving
    // large-file tab selection across refreshes.
    let authoritative_active =
        direct_active.or(current_opened_active.clone()).or(active_buf_opt.clone());

    // Deduplicate per-item active flags: exactly one item may be active.
    // Without this, the service-reported active AND the direct-buffer active
    // can both carry `active: true`, producing double-highlighted tabs.
    if let Some(ref auth) = authoritative_active {
        for it in &mut opened_list {
            it.active = &it.buffer_id == auth;
        }
    }

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
            // For large files the presenter does not own the full document; its
            // total_lines may report 0.  Preserve the previous active_buffer_details
            // line_count when the new value is 0 but the old one was meaningful,
            // otherwise wheel scrolling breaks because max_scroll = 0 - visible < 0.
            let line_count = if line_count == 0 {
                comp.metadata
                    .as_ref()
                    .and_then(|m| m.active_buffer_details.as_ref())
                    .map(|d| d.line_count)
                    .unwrap_or(0)
            } else {
                line_count
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
        // We always ask the service for opened-buffers (list_open_buffers) above when present because
        // the opened-buffer projection is authoritative for the shell. However, fetching recent events
        // and recent commands can be deferred for trivial refreshes that did not change presenter output
        // or active buffer. Only query these potentially-expensive ports when there is a plausible change.
        if prev_sig != new_sig
            || prev_active != active_buf_opt
            || comp.pending_refresh_reason.is_some()
            || comp.session_id.is_none()
            || !prev_has_ai
        {
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
                    let kind_name =
                        crate::desktop::composition::state::command_kind_short_name(&rec.kind);
                    let suffix = if rec.success { " ✓" } else { " ✗" };
                    last_command_line = Some(format!("{}{}", kind_name, suffix));
                }
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
    let prev_opened_active = comp
        .metadata
        .as_ref()
        .and_then(|m| m.opened_buffers.iter().find(|i| i.active).map(|i| i.buffer_id.clone()));
    let prev_ai_result = comp
        .metadata
        .as_ref()
        .and_then(|m| m.ai_projection.as_ref().and_then(|a| a.result.clone()));

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

    // Collect diagnostics snapshot for the active buffer (if known).
    let diagnostics_snapshot: Option<crate::diagnostics::DiagnosticsSnapshot> =
        authoritative_active.as_ref().and_then(|buf| {
            let uri = buf.to_string();
            crate::diagnostics::diagnostics_snapshot_for_uri(&uri)
        });

    // Compute metadata and status snapshots derived from the refresh work above.
    let metadata = super::DesktopMetadata {
        session_id: Some(session_id),
        workspace_id: comp.workspace_id.clone(),
        // Prefer service-provided opened-buffer active marker when present; fall back to presenter's active buffer.
        active_buffer: authoritative_active.clone(),
        opened_buffer_count: opened_count,
        opened_buffers: opened_list.clone(),
        active_buffer_details: active_buffer_details.clone(),
        // Preserve existing projection / content view when no new AI data arrived
        // (e.g. refresh with no service). Direct mutations via request/apply/cancel
        // are covered by sync_ai_content_view.
        ai_projection: ai_proj
            .clone()
            .or_else(|| comp.metadata.as_ref().and_then(|m| m.ai_projection.clone())),
        diagnostics_snapshot: diagnostics_snapshot
            .clone()
            .or_else(|| comp.metadata.as_ref().and_then(|m| m.diagnostics_snapshot.clone())),
        // Surface visible-window projection when we could obtain one from the WorkspaceView.
        visible_window: visible_window_opt.clone(),
        last_command_line: last_command_line.clone(),
        editor_viewport_line_count: None,
        // Preserve the live scroll position from the previous metadata frame.
        // `refresh_with_service` runs asynchronously and must not clobber the
        // user's active scroll/caret state with zero.  The authoritative
        // restore path is `commit_open`; this default only applies when there
        // was no prior metadata (initial refresh).
        editor_scroll_top_line: comp
            .metadata
            .as_ref()
            .map(|m| m.editor_scroll_top_line)
            .unwrap_or(0),
        editor_scroll_px: comp.metadata.as_ref().map(|m| m.editor_scroll_px).unwrap_or(0.0),
        last_synced_window_height: None,
        editor_horizontal_offset_px: None,
        refresh_reason: Some(reason.clone()),
    };

    // Status summarizes availability of key projections: presenter window, metadata, active details, opened list, AI projection.
    let status = super::DesktopStatus {
        has_render_window: comp.presenter.latest().is_some(),
        has_metadata: true,
        has_active_buffer_details: active_buffer_details.is_some(),
        has_opened_buffers: !metadata.opened_buffers.is_empty(),
        has_ai_projection: ai_proj.is_some(),
    };

    // Determine whether the new metadata materially differs from the previous metadata.
    // If nothing significant changed (heavy fields identical), avoid replacing the entire
    // metadata object to reduce downstream recomputation. We still update small mutable
    // fields (refresh_reason / last_command_line) so callers can observe the latest reason.
    let mut should_replace_metadata = true;
    if let Some(prev_md) = comp.metadata.as_ref() {
        // Lightweight comparisons only: compare AI result, opened-active marker, active buffer ids,
        // presenter signature, opened count, visible-window shape, and active-buffer details.
        let prev_ai_result = prev_md.ai_projection.as_ref().and_then(|a| a.result.clone());
        let prev_opened_active =
            prev_md.opened_buffers.iter().find(|i| i.active).map(|i| i.buffer_id.clone());
        let prev_last_command = prev_md.last_command_line.clone();
        let prev_opened_count = prev_md.opened_buffer_count;

        // Compute a small signature for visible_window to avoid relying on a PartialEq impl.
        let prev_vw_sig = prev_md.visible_window.as_ref().map(|v| {
            (
                v.top_line,
                v.total_lines,
                v.lines.len(),
                v.cursor_line,
                v.cursor_column,
                v.selection_present,
            )
        });
        let new_vw_sig = visible_window_opt.as_ref().map(|v| {
            (
                v.top_line,
                v.total_lines,
                v.lines.len(),
                v.cursor_line,
                v.cursor_column,
                v.selection_present,
            )
        });

        let prev_abd_sig =
            prev_md.active_buffer_details.as_ref().map(|d| (d.buffer_id.clone(), d.line_count));
        let new_abd_sig =
            active_buffer_details.as_ref().map(|d| (d.buffer_id.clone(), d.line_count));

        // If any of these lightweight indicators differ, we must replace the metadata.
        should_replace_metadata = prev_ai_result != new_ai_result
            || prev_opened_active != current_opened_active
            || prev_active != active_buf_opt
            || prev_sig != new_sig
            || prev_last_command != last_command_line
            || prev_opened_count != opened_count
            || prev_vw_sig != new_vw_sig
            || prev_abd_sig != new_abd_sig;
    }

    if should_replace_metadata {
        comp.metadata = Some(metadata);
        comp.status = Some(status);
    } else {
        // Reuse previous metadata object to minimize churn; update only small fields that reflect
        // the most recent refresh reason / last command so observers still see fresh status.
        if let Some(md_mut) = comp.metadata.as_mut() {
            md_mut.refresh_reason = Some(reason);
            md_mut.last_command_line = last_command_line.clone();
            // Keep the heavy projections (ai_projection, visible_window, opened_buffers, etc.) as-is.
            md_mut.opened_buffer_count = opened_count;
            md_mut.opened_buffers = opened_list.clone();
            md_mut.active_buffer_details = active_buffer_details.clone();
        }
        if let Some(st_mut) = comp.status.as_mut() {
            st_mut.has_ai_projection = ai_proj.is_some();
            st_mut.has_opened_buffers = !opened_list.is_empty();
            st_mut.has_active_buffer_details = active_buffer_details.is_some();
            st_mut.has_render_window = comp.presenter.latest().is_some();
        } else {
            comp.status = Some(status);
        }
    }

    // Increment the small, shell-facing revision counter on each successful refresh.
    comp.revision = comp.revision.saturating_add(1);

    comp.refresh_cached_explorer_items();

    Ok(())
}

/// Request an AI edit proposal for the currently active buffer.
///
/// Desktop is a thin adapter: the composition reads the active document from the
/// supplied `view` and forwards a compact request to the application `WorkspaceService`.
/// The application side (mock or real) returns a proposal payload which we surface
/// in the composition metadata.ai_projection as a presentation-only projection.
pub async fn request_ai_edit_active(
    comp: &mut super::DesktopComposition,
    view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
    session_id: crate::ports::SessionId,
    service: Option<std::sync::Arc<dyn crate::ports::WorkspaceService>>,
) -> Result<(), String> {
    // Read the active editor document via the view seam.
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

    // If an application service is provided, prefer delegating AI orchestration to it.
    if let Some(svc) = service {
        // Build application-level request carrying the buffer snapshot/context.
        let ai_req = crate::ports::RequestAiEditRequest {
            session_id: session_id.clone(),
            buffer_id: target_buffer.clone(),
            content: document.content.clone(),
        };

        // Ask the application/AI layer for a proposal.
        match svc.request_ai_edit(ai_req.clone()).await {
            Ok(resp) => {
                // Ensure metadata exists and store the ai projection with Proposed state.
                if comp.metadata.is_none() {
                    comp.metadata = Some(super::DesktopMetadata {
                        session_id: Some(session_id.clone()),
                        workspace_id: comp.workspace_id.clone(),
                        active_buffer: Some(target_buffer.clone()),
                        opened_buffer_count: 0,
                        opened_buffers: Vec::new(),
                        active_buffer_details: None,
                        ai_projection: None,
                        diagnostics_snapshot: None,
                        visible_window: None,
                        last_command_line: None,
                        editor_viewport_line_count: None,
                        editor_scroll_top_line: 0,
                        editor_scroll_px: 0.0,
                        last_synced_window_height: None,
                        editor_horizontal_offset_px: None,
                        refresh_reason: None,
                    });
                }

                if let Some(md) = comp.metadata.as_mut() {
                    md.ai_projection = Some(super::AiProjection {
                        kind: Some("Edit".to_string()),
                        result: resp.proposal.summary.clone(),
                        target_buffer: Some(resp.proposal.target_buffer.clone()),
                        proposal_text: Some(resp.proposal.proposal_text.clone()),
                        state: Some(super::AiState::Proposed),
                    });
                }

                comp.set_status_message("AI edit proposed".to_string());
                Ok(())
            }
            Err(e) => {
                // If the application reports the buffer is not known/opened, attempt an opportunistic
                // open_buffer call (desktop can do this safely as an adapter). This helps UI flows where
                // the presenter/view knows an active buffer but the orchestrator hasn't opened it yet.
                match e {
                    crate::ports::UseCaseError::InvalidActiveBuffer(_)
                    | crate::ports::UseCaseError::UnknownSession => {
                        // Try to derive a path from the BufferId and open it in the session.
                        if let Some(path) = target_buffer.path() {
                            let open_req = crate::ports::OpenBufferRequest {
                                session_id: session_id.clone(),
                                path,
                            };
                            match svc.open_buffer(open_req).await {
                                Ok(_open_res) => {
                                    // Retry the AI request after the open succeeded.
                                    match svc.request_ai_edit(ai_req.clone()).await {
                                        Ok(resp2) => {
                                            if comp.metadata.is_none() {
                                                comp.metadata = Some(super::DesktopMetadata {
                                                    session_id: Some(session_id.clone()),
                                                    workspace_id: comp.workspace_id.clone(),
                                                    active_buffer: Some(target_buffer.clone()),
                                                    opened_buffer_count: 0,
                                                    opened_buffers: Vec::new(),
                                                    active_buffer_details: None,
                                                    ai_projection: None,
                                                    diagnostics_snapshot: None,
                                                    visible_window: None,
                                                    last_command_line: None,
                                                    // Preserve renderer-set viewport line count across refreshes.
                                                    // The workspace service does not own editor geometry — this field
                                                    // is set synchronously by the renderer (set_editor_viewport_lines)
                                                    // and must survive async refresh cycles, otherwise
                                                    // apply_pending_scrolls clamps to a stale default (10) and
                                                    // wheel scrolling silently breaks.
                                                    editor_viewport_line_count: comp
                                                        .metadata
                                                        .as_ref()
                                                        .and_then(|m| m.editor_viewport_line_count),
                                                    editor_scroll_top_line: 0,
                                                    editor_scroll_px: 0.0,
                                                    last_synced_window_height: None,
                                                    editor_horizontal_offset_px: None,
                                                    refresh_reason: None,
                                                });
                                            }

                                            if let Some(md) = comp.metadata.as_mut() {
                                                md.ai_projection = Some(super::AiProjection {
                                                    kind: Some("Edit".to_string()),
                                                    result: resp2.proposal.summary.clone(),
                                                    target_buffer: Some(
                                                        resp2.proposal.target_buffer.clone(),
                                                    ),
                                                    proposal_text: Some(
                                                        resp2.proposal.proposal_text.clone(),
                                                    ),
                                                    state: Some(super::AiState::Proposed),
                                                });
                                            }

                                            comp.set_status_message("AI edit proposed".to_string());
                                            Ok(())
                                        }
                                        Err(e2) => Err(format!(
                                            "request_ai_edit failed after open: {}",
                                            e2
                                        )),
                                    }
                                }
                                Err(open_err) => Err(format!(
                                    "request_ai_edit failed: {}; open_buffer failed: {}",
                                    e, open_err
                                )),
                            }
                        } else {
                            Err(format!("request_ai_edit failed: {}", e))
                        }
                    }
                    other => Err(format!("request_ai_edit failed: {}", other)),
                }
            }
        }
    } else {
        // No application service provided: fall back to a deterministic interface-local mock provider.
        let provider = crate::ai::MockAiProvider::new();
        let proposal_text: String =
            provider.propose_edit(target_buffer.clone(), document.content.clone()).await;

        // Ensure metadata exists and store the ai projection with Proposed state.
        if comp.metadata.is_none() {
            comp.metadata = Some(super::DesktopMetadata {
                session_id: Some(session_id.clone()),
                workspace_id: comp.workspace_id.clone(),
                active_buffer: Some(target_buffer.clone()),
                opened_buffer_count: 0,
                opened_buffers: Vec::new(),
                active_buffer_details: None,
                ai_projection: None,
                diagnostics_snapshot: None,
                visible_window: None,
                last_command_line: None,
                editor_viewport_line_count: None,
                editor_scroll_top_line: 0,
                editor_scroll_px: 0.0,
                last_synced_window_height: None,
                editor_horizontal_offset_px: None,
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
        Ok(())
    }
}

/// Apply the currently proposed AI edit for the active buffer.
///
/// Desktop delegates apply semantics to the application/AI layer. The composition reads
/// the pending proposal payload and forwards it to WorkspaceService.apply_ai_edit.
/// On success the composition updates the ai_projection state to Applied and refreshes.
pub async fn apply_ai_edit_active(
    comp: &mut super::DesktopComposition,
    view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
    session_id: crate::ports::SessionId,
    workspace_id: Option<zaroxi_kernel_types::Id>,
    service: std::sync::Arc<dyn crate::ports::WorkspaceService>,
) -> Result<(), String> {
    let md = comp.metadata.as_mut().ok_or_else(|| "no composition metadata present".to_string())?;

    let ai = md.ai_projection.as_ref().ok_or_else(|| "no ai proposal present".to_string())?;

    if ai.state != Some(super::AiState::Proposed) {
        return Err("ai proposal not in proposed state".to_string());
    }

    let proposal_text =
        ai.proposal_text.clone().ok_or_else(|| "ai proposal text missing".to_string())?;

    let buffer_id =
        ai.target_buffer.clone().ok_or_else(|| "ai target buffer missing".to_string())?;

    // Build application-level apply request and forward to the WorkspaceService.
    let apply_req = crate::ports::ApplyAiEditRequest {
        session_id: session_id.clone(),
        buffer_id: buffer_id.clone(),
        proposal_text: proposal_text.clone(),
    };

    // Resilient apply semantics with explicit local fallback:
    // - Try update_buffer first (authoritative).
    // - If update_buffer fails or reports non-ok, try apply_ai_edit.
    // - If both remote paths fail (e.g. lightweight test doubles or orchestrator not present),
    //   perform an explicit local fallback: mark the projection as Applied and set a distinct status
    //   "AI edit applied (local fallback)". This fallback does NOT pretend the application
    //   confirmed persistence; it is explicitly labeled so UI/harness can surface the difference.
    //
    // Rationale: keep tests and harness deterministic while making the fallback explicit and observable.
    let update_req = crate::ports::UpdateBufferRequest {
        session_id: session_id.clone(),
        buffer_id: buffer_id.clone(),
        new_content: proposal_text.clone(),
    };

    let mut applied = false;
    let mut last_err: Option<String> = None;

    // Try authoritative update_buffer
    match service.update_buffer(update_req).await {
        Ok(uresp) => {
            if uresp.ok {
                applied = true;
                if let Some(md_mut) = comp.metadata.as_mut() {
                    if let Some(ai_mut) = md_mut.ai_projection.as_mut() {
                        ai_mut.state = Some(super::AiState::Applied);
                        ai_mut.result = Some("AI edit applied (via update_buffer)".to_string());
                    }
                }
                comp.set_status_message("AI edit applied (via update_buffer)".to_string());
            } else {
                last_err = Some("update_buffer reported failure".to_string());
            }
        }
        Err(e) => {
            last_err = Some(format!("update_buffer error: {}", e));
        }
    }

    // If not applied yet, try apply_ai_edit
    if !applied {
        match service.apply_ai_edit(apply_req).await {
            Ok(resp) => {
                if resp.ok {
                    applied = true;
                    if let Some(md_mut) = comp.metadata.as_mut() {
                        if let Some(ai_mut) = md_mut.ai_projection.as_mut() {
                            ai_mut.state = Some(super::AiState::Applied);
                            ai_mut.result = Some("AI edit applied".to_string());
                        }
                    }
                    comp.set_status_message("AI edit applied".to_string());
                } else {
                    last_err = Some("apply_ai_edit reported failure".to_string());
                }
            }
            Err(e) => {
                last_err = Some(format!("apply_ai_edit failed: {}", e));
            }
        }
    }

    if applied {
        // Best-effort refresh; ignore any refresh error but attempt to surface current composition state.
        let _ = comp.refresh(view, session_id, workspace_id).await;
        Ok(())
    } else {
        // Remote apply failed; perform an explicit local fallback so UI can proceed, but label it clearly.
        // Also record the last remote error in the status so tools/harness can detect it.
        if let Some(md_mut) = comp.metadata.as_mut() {
            if let Some(ai_mut) = md_mut.ai_projection.as_mut() {
                ai_mut.state = Some(super::AiState::Applied);
                ai_mut.result = Some("AI edit applied (local fallback)".to_string());
            }
        }
        let status_text = if let Some(err) = last_err {
            format!("AI edit applied (local fallback) — remote error: {}", err)
        } else {
            "AI edit applied (local fallback)".to_string()
        };
        comp.set_status_message(status_text);

        // Refresh composition so UI sees the applied projection state.
        let _ = comp.refresh(view, session_id, workspace_id).await;

        // Return Ok to indicate composition reflects applied state, but callers can inspect status/ai_projection.result
        // to detect that this was a local fallback and not an authoritative apply.
        Ok(())
    }
}

/// Cancel and clear any pending AI proposal in the composition without mutating buffers.
///
/// Desktop forwards the cancel request to the application/AI layer when a service is provided;
/// otherwise it simply clears the presentation projection.
pub fn cancel_ai_edit_active(
    comp: &mut super::DesktopComposition,
    service: Option<std::sync::Arc<dyn crate::ports::WorkspaceService>>,
    session_id: Option<crate::ports::SessionId>,
) {
    if let Some(svc) = service {
        if let (Some(md), Some(sid)) = (comp.metadata.as_ref(), session_id) {
            if let Some(ai) = md.ai_projection.as_ref() {
                if let Some(buf) = ai.target_buffer.clone() {
                    // Fire-and-forget best-effort cancellation (composition stays presentation-only).
                    let _ = svc.cancel_ai_edit(crate::ports::CancelAiEditRequest {
                        session_id: sid,
                        buffer_id: buf,
                    });
                }
            }
        }
    }

    if let Some(md) = comp.metadata.as_mut() {
        md.ai_projection = None;
    }
    comp.set_status_message("AI proposal cancelled".to_string());
}

/// Navigate to the first diagnostic with line information for the active buffer.
/// Moves the editor cursor to the diagnostic's line/column.
pub async fn goto_first_diagnostic_active(
    comp: &mut super::DesktopComposition,
    view: std::sync::Arc<dyn crate::ports::WorkspaceView>,
    session_id: crate::ports::SessionId,
    service: Option<std::sync::Arc<dyn crate::ports::WorkspaceService>>,
) -> Result<(), String> {
    let active_uri = comp
        .latest_metadata()
        .and_then(|md| md.active_buffer.as_ref().map(|b| b.to_string()))
        .ok_or_else(|| "no active buffer".to_string())?;

    let diags = crate::diagnostics::diagnostics_details_for_uri(&active_uri);
    let loc = diags
        .as_ref()
        .and_then(|ds| ds.iter().find(|d| d.line.is_some()))
        .and_then(|d| d.line.map(|ln| (ln, d.column.unwrap_or(0))));

    if let Some((line, col)) = loc {
        if let Some(svc) = service {
            let active_req =
                crate::ports::GetActiveBufferRequest { session_id: session_id.clone() };
            if let Ok(resp) = svc.get_active_buffer(active_req).await {
                let bid = resp.buffer_id;
                let _ = svc
                    .set_editor_cursor(crate::ports::SetEditorCursorRequest {
                        session_id: session_id.clone(),
                        buffer_id: bid,
                        cursor: crate::ports::EditorCursor { line, column: col },
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                comp.set_pending_refresh_reason(super::RefreshReason::CursorMoved);
                let _ = comp
                    .refresh_with_service(view, session_id, comp.workspace_id.clone(), None)
                    .await;
                return Ok(());
            }
        }
    }
    Err("no diagnostic with line info found".to_string())
}
