/*!
Widget activation routing and domain action dispatch.

Moved from app.rs to keep the widget-activation match arms
in a focused module while app.rs stays thin.

All explorer CTA activation, tab switching, panel close/open,
and clipboard paste actions live here.
*/

use pollster;
use std::sync::mpsc;
use zaroxi_core_engine_ui::ShellWorkContent;
use zaroxi_core_engine_ui::WidgetId;
use zaroxi_core_engine_ui::layout_constants as lc;

use super::DocumentViewState;
use super::GuiApp;
use super::open_pipeline;

/// Dispatch a WidgetId activation to DesktopComposition domain actions.
/// Used by the explorer CTA button and by the standard activation handler.
pub(crate) fn dispatch_activation(app: &mut GuiApp, id: &WidgetId) -> Option<ShellWorkContent> {
    // Clicking anything other than the AI composer releases its keyboard focus.
    if app.ai_composer_focused && !matches!(id, WidgetId::TextInput { index: 0 }) {
        app.ai_composer_focused = false;
    }
    match id {
        WidgetId::Button { index: lc::BTN_ID_CLOSE_WINDOW } => {
            std::process::exit(0);
        }
        WidgetId::Button { index: lc::BTN_ID_MINIMIZE } => {
            if let Some(z) = app.maybe_window.as_ref() {
                z.window().set_minimized(true);
            }
            return None;
        }
        WidgetId::Button { index: lc::BTN_ID_MAXIMIZE } => {
            if let Some(z) = app.maybe_window.as_ref() {
                let maximized = z.window().is_maximized();
                z.window().set_maximized(!maximized);
            }
            return None;
        }
        WidgetId::Button { index: lc::BTN_ID_EXPLORER_CTA } => {
            super::debug::click_trace("ZAROXI_CLICK: dispatch_activation matched Explorer CTA");
            if app.picker_in_flight {
                super::debug::click_trace(
                    "ZAROXI_CLICK: picker already in flight — ignoring duplicate click",
                );
                return None;
            }
            if let Some(picker) = app.folder_picker.clone() {
                super::debug::click_trace(
                    "ZAROXI_CLICK: spawning picker thread to avoid blocking event loop",
                );
                let (tx, rx) = mpsc::channel();
                app.pending_picker_rx = Some(rx);
                app.picker_in_flight = true;
                std::thread::spawn(move || {
                    let outcome = picker.pick_folder();
                    let _ = tx.send(outcome);
                });
                app.invalidate(super::InvalidationFlags::content());
            } else {
                super::debug::click_trace(
                    "ZAROXI_CLICK: folder_picker is None, cannot open workspace",
                );
            }
            return None;
        }
        // ── Bottom-panel tabs (Terminal / Problems / Output) ──
        // These live in a dedicated Tab index space so they never collide with
        // the editor tab strip. Handled here (before the workspace guard) so the
        // terminal works even when no workspace is open.
        WidgetId::Tab { index } if *index >= lc::BOTTOM_TAB_ID_BASE => {
            app.select_bottom_tab(*index - lc::BOTTOM_TAB_ID_BASE);
            return None;
        }
        // Terminal panel close/restart action (both the header "x" button and
        // the panel-action variant route here). Kills the shell; Enter/Ctrl+`
        // restarts it.
        WidgetId::Button { index: lc::BTN_ID_TERMINAL_CLOSE } => {
            app.close_terminal_action();
            return None;
        }
        WidgetId::PanelAction { header_id, action }
            if *header_id == "terminal" && *action == "close" =>
        {
            app.close_terminal_action();
            return None;
        }
        // ── AI panel: provider setup CTA ──
        // Jumps to the Settings destination where AI providers are configured.
        // Handled before the workspace guard so it works with no workspace open.
        WidgetId::Button { index: lc::BTN_ID_AI_SETUP_PROVIDER } => {
            app.tab_state.open_or_focus_non_file(
                crate::gui::window::destination::WorkbenchTabId::DestinationRoot(
                    crate::gui::window::destination::WorkbenchDestination::Settings,
                ),
            );
            app.invalidate(super::InvalidationFlags::content());
            return None;
        }
        // ── AI panel: session controls (New chat / Clear) ──
        // New chat archives the current conversation; Clear resets it in
        // place. Both reset the live session state and any pending projection.
        WidgetId::Button { index: lc::BTN_ID_AI_NEW_CHAT } => {
            app.ai_new_chat();
            return None;
        }
        WidgetId::Button { index: lc::BTN_ID_AI_CLEAR } => {
            app.ai_clear_conversation();
            return None;
        }
        // ── AI panel: prompt composer focus ──
        WidgetId::TextInput { index: 0 } => {
            app.ai_composer_focused = true;
            app.explorer_search_active = false;
            app.invalidate(super::InvalidationFlags::content());
            return None;
        }
        // ── AI panel: quick actions (Explain / Refactor / Tests / Fix) ──
        WidgetId::Button { index: lc::BTN_ID_AI_EXPLAIN } => {
            app.ai_run_quick_action(super::ai_chat::AiQuickAction::Explain);
            return None;
        }
        WidgetId::Button { index: lc::BTN_ID_AI_REFACTOR } => {
            app.ai_run_quick_action(super::ai_chat::AiQuickAction::Refactor);
            return None;
        }
        WidgetId::Button { index: lc::BTN_ID_AI_TESTS } => {
            app.ai_run_quick_action(super::ai_chat::AiQuickAction::GenerateTests);
            return None;
        }
        WidgetId::Button { index: lc::BTN_ID_AI_FIX } => {
            app.ai_run_quick_action(super::ai_chat::AiQuickAction::FixDiagnostics);
            return None;
        }
        // ── AI panel: edit approval (never auto-applied) ──
        WidgetId::Button { index: lc::BTN_ID_AI_APPLY } => {
            app.ai_apply_proposal();
            return None;
        }
        WidgetId::Button { index: lc::BTN_ID_AI_REJECT } => {
            app.ai_reject_proposal();
            return None;
        }
        _ => {}
    }

    let comp = app.composition.as_mut()?;
    let view = app.workspace_view.as_ref()?;
    let service = app.workspace_service.as_ref()?;
    let session = app.session_id.clone()?;

    match id {
        WidgetId::Button { index: lc::BTN_ID_TERMINAL_CLOSE } => Some(comp.build_work_content()),
        WidgetId::Button { index: lc::BTN_ID_AI_CLOSE } => {
            pollster::block_on(crate::actions::close_command_bar(comp)).ok();
            Some(comp.build_work_content())
        }
        WidgetId::TextInput { .. } => None,
        WidgetId::Tab { index } => {
            if super::first_open_trace_enabled() {
                eprintln!("ZAROXI_DEBUG_FIRST_OPEN: activation=tab tab_index={}", *index);
            }
            // Resolve tab index from EditorGroup (sole authority for file tabs).
            let file_tabs = app.editor_group.visible_tabs();
            if *index < file_tabs.len() {
                let vt = &file_tabs[*index];
                let buffer_id = vt.buffer_id.clone();
                if std::env::var("ZAROXI_DEBUG_TAB_CLICK").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_DEBUG_TAB_CLICK: tab_index={} buffer_id={} display={} editor_group path={}",
                        *index, buffer_id, vt.display, vt.path,
                    );
                }
                app.editor_group.activate_by_path(&vt.path);
                // Large-file (direct/PieceTable) buffers: activate locally.
                let bid = crate::ports::BufferId(buffer_id.clone());
                if comp.direct_buffer_ids.contains(&bid) {
                    comp.set_direct_buffer_active(bid.clone());
                } else {
                    comp.clear_direct_active();
                    let _ = pollster::block_on(
                        crate::actions::set_active_buffer_and_get_shell_context(
                            comp,
                            service.clone(),
                            view.clone(),
                            session,
                            app.workspace_id,
                            bid,
                        ),
                    );
                }
                app.open_intent = Some(open_pipeline::OpenIntent::ActivateExisting);
                app.tab_state.focus_tab(&crate::gui::window::destination::WorkbenchTabId::Editor);
                app.rail_selected_index = 0;
                app.cockpit_status_fingerprint = 0;
                return Some(comp.build_work_content());
            }
            // Tab index beyond the file tabs addresses a non-file workbench
            // tab in the unified strip. Resolve it from the canonical tab
            // state and focus it — never mutate the legacy rail index here
            // (the rail is a derived reflection of `tab_state.active()`).
            let non_file_idx = *index - file_tabs.len();
            let non_file_ids: Vec<crate::gui::window::destination::WorkbenchTabId> = app
                .tab_state
                .entries()
                .iter()
                .filter_map(|e| match e {
                    crate::gui::window::destination::WorkbenchTabEntry::NonFile { id, .. }
                        if !matches!(
                            id,
                            crate::gui::window::destination::WorkbenchTabId::Welcome
                        ) =>
                    {
                        Some(id.clone())
                    }
                    _ => None,
                })
                .collect();
            if let Some(id) = non_file_ids.get(non_file_idx) {
                app.tab_state.open_or_focus_non_file(id.clone());
                return Some(comp.build_work_content());
            }
            None
        }
        WidgetId::PanelAction { header_id, action } => {
            match (*header_id, *action) {
                ("ai_assistant", "close") => {
                    pollster::block_on(crate::actions::close_command_bar(comp)).ok();
                }
                ("terminal", "close") => {}
                _ => {}
            }
            Some(comp.build_work_content())
        }
        WidgetId::ListItem { index } => {
            if *index >= 100 {
                // Canonical: open/focus the Extensions destination tab rather
                // than poking the legacy rail index directly.
                app.tab_state.open_or_focus_non_file(
                    crate::gui::window::destination::WorkbenchTabId::DestinationRoot(
                        crate::gui::window::destination::WorkbenchDestination::Extensions,
                    ),
                );
                return Some(comp.build_work_content());
            }
            if *index >= 10 {
                let comp = app.composition.as_mut()?;
                let explorer_idx = *index - 10;

                let resolve_idx = || -> Option<usize> {
                    let ids = &app.last_explorer_ids;
                    if ids.is_empty() || explorer_idx >= ids.len() {
                        return Some(explorer_idx);
                    }
                    let target_id = ids.get(explorer_idx)?;
                    comp.cached_explorer_items.iter().position(|ev| &ev.id == target_id)
                };
                let resolved = resolve_idx().unwrap_or(explorer_idx);

                if comp.is_explorer_item_dir(resolved) {
                    return app.explorer_actions.as_mut()?.toggle_directory(comp, resolved);
                }
                // ── Phase 8: async file open ──
                // Detect single vs double-click for preview/pinned intent.
                let now = std::time::Instant::now();
                let click_idx = resolved;
                let is_double = app.last_explorer_click.is_some_and(|(t, idx, _)| {
                    now.duration_since(t).as_millis() < 400 && idx == click_idx
                });
                app.last_explorer_click = Some((now, click_idx, app.open_token));
                let service = app.workspace_service.clone()?;
                let session = app.session_id.clone()?;
                let item_id = comp.get_explorer_item_id_at(resolved)?;
                let path = comp.maybe_explorer.as_ref()?.get_entry_path(&item_id)?;
                let path_str = path.to_string_lossy().to_string();

                // ── Membership check: EditorGroup is the sole authority ──
                let already_pinned = app.editor_group.is_pinned(&path_str);
                let already_preview = app.editor_group.is_preview(&path_str);

                if already_pinned {
                    // File is already pinned in EditorGroup. Activate it directly.
                    let bid = crate::ports::BufferId(format!("buf:{}", path_str));
                    if comp.direct_buffer_ids.contains(&bid) {
                        comp.set_direct_buffer_active(bid.clone());
                        app.open_intent = Some(open_pipeline::OpenIntent::ActivateExisting);
                    } else {
                        let view = app.workspace_view.clone()?;
                        let workspace_id = app.workspace_id;
                        let _ = pollster::block_on(
                            crate::actions::set_active_buffer_and_get_shell_context(
                                comp,
                                service.clone(),
                                view,
                                session.clone(),
                                workspace_id,
                                bid,
                            ),
                        );
                        app.open_intent = Some(open_pipeline::OpenIntent::ActivateExisting);
                    }
                    app.tab_state
                        .focus_tab(&crate::gui::window::destination::WorkbenchTabId::Editor);
                    app.rail_selected_index = 0;
                    app.cockpit_status_fingerprint = 0;
                    let wc = comp.build_work_content();
                    app.request_open(wc);
                    app.needs_render = true;
                    if super::doc_lifecycle_trace_enabled() {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: explorer_click idx={} intent=ActivateExisting reason=editor_group_pinned path={}",
                            click_idx, path_str,
                        );
                    }
                    return None;
                }

                let raw_intent = if is_double {
                    if already_preview {
                        // Double-click on previewed file: promote.
                        if super::doc_lifecycle_trace_enabled() {
                            eprintln!(
                                "ZAROXI_DOC_LIFECYCLE: explorer_click idx={} intent=Pinned reason=double_click_preview path={}",
                                click_idx, path_str,
                            );
                        }
                        // Promote immediately — no background read needed.
                        // Same-document preview→pin is a pure tab-membership
                        // transition: mutate ONLY EditorGroup here. Do NOT
                        // touch document/render/scroll state.
                        let _ = app.editor_group.promote_preview_to_pinned();
                        let bid = crate::ports::BufferId(format!("buf:{}", path_str));
                        // Only large/direct (PieceTable-backed) files need a
                        // direct opened-buffer registration to keep their tab.
                        // Normal service-backed files are already in
                        // opened_buffers via the service; registering them as
                        // "direct" here would both pollute direct_buffer_ids and
                        // (via add_opened_buffer_direct) reset the scroll
                        // line_count — the exact preview→pin scroll break.
                        let is_direct_doc = app.doc_buffers.contains_key(path_str.as_str());
                        if is_direct_doc && !comp.direct_buffer_ids.contains(&bid) {
                            comp.add_opened_buffer_direct(
                                bid.clone(),
                                path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()),
                            );
                        }
                        if super::doc_lifecycle_trace_enabled() {
                            let kept_scroll = comp
                                .metadata
                                .as_ref()
                                .map(|m| m.editor_scroll_top_line)
                                .unwrap_or(0);
                            let kept_line_count = comp
                                .metadata
                                .as_ref()
                                .and_then(|m| m.active_buffer_details.as_ref())
                                .map(|d| d.line_count)
                                .unwrap_or(0);
                            eprintln!(
                                "ZAROXI_DOC_LIFECYCLE: promotion_preserve_same_doc path={} kept_scroll={} kept_source={} kept_line_count={}",
                                path_str,
                                kept_scroll,
                                if is_direct_doc { "doc_buffers" } else { "rope" },
                                kept_line_count,
                            );
                        }
                        app.open_intent = Some(open_pipeline::OpenIntent::ActivateExisting);
                        app.tab_state
                            .focus_tab(&crate::gui::window::destination::WorkbenchTabId::Editor);
                        app.rail_selected_index = 0;
                        app.cockpit_status_fingerprint = 0;
                        let wc = comp.build_work_content();
                        app.request_open(wc);
                        app.needs_render = true;
                        return None;
                    }
                    open_pipeline::OpenIntent::Pinned
                } else {
                    open_pipeline::OpenIntent::Preview
                };

                app.open_intent = Some(raw_intent);
                if super::doc_lifecycle_trace_enabled() {
                    eprintln!(
                        "ZAROXI_DOC_LIFECYCLE: explorer_click idx={} intent={:?} is_double={} reason=new_file path={}",
                        click_idx, raw_intent, is_double, path_str,
                    );
                }
                // Instead of `block_on`'ing the disk read on the UI thread (~1 s
                // for huge files) before `request_open`, resolve the path
                // cheaply, schedule a tokened background read, and return instant
                // loading chrome. `GuiApp::poll_read_results` activates the buffer
                // and calls `request_open` once the read lands. Stale reads (a
                // newer file clicked) are dropped by read token.
                let display_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                app.read_token += 1;
                let token = app.read_token;
                // Bump the shared generation synchronously on the click so the
                // worker can skip a still-queued/just-picked-up stale read.
                app.read_generation.store(token, std::sync::atomic::Ordering::Relaxed);
                super::debug::click_trace(&format!(
                    "ZAROXI_CLICK: async open_file path={}",
                    path.display()
                ));
                comp.set_status_message(format!("Loading: {}", display_name));
                if app.read_worker.is_none() {
                    app.read_worker = Some(super::background_read::BackgroundReadWorker::spawn(
                        app.read_generation.clone(),
                    ));
                }
                if let Some(w) = app.read_worker.as_mut() {
                    w.schedule_read(super::background_read::ReadJob {
                        token,
                        service,
                        session_id: session,
                        path: path.clone(),
                    });
                }
                app.read_pending = true;
                app.read_started_at = Some(std::time::Instant::now());
                app.last_upstream_open_prep_ms = 0.0;
                // Phase 11: begin the atomic open-presentation snapshot. The old
                // file / loading shell stays visible until this open's first
                // screenful is shaped, then content + chrome swap together in one
                // coherent atomic first paint (see `poll_read_results` /
                // `finalize_buffer_commit`).
                app.open_present = Some(super::OpenPresentation::begin(
                    token,
                    Some(path.to_string_lossy().to_string()),
                ));
                if super::file_open_trace_enabled() {
                    eprintln!(
                        "ZAROXI_FILE_OPEN_TRACE: read_token={} stage=read_scheduled read_pending=1 file={}",
                        token,
                        path.display(),
                    );
                }
                if super::open_present_trace_enabled() {
                    eprintln!(
                        "ZAROXI_OPEN_PRESENT_TRACE: token={} stage=read_scheduled file={}",
                        token,
                        path.display(),
                    );
                }
                // Instant loading chrome (status + current content). The new
                // file's content commits via `poll_read_results` -> `request_open`
                // when the off-thread read completes.
                // Switch from Welcome to Editor mode immediately so the tab
                // strip reflects the file-editor selection even while the
                // background read is still in flight.
                //
                // Build a loading-chrome work-content with the correct
                // active_file for the new file.  Using `comp.build_work_content()`
                // directly would carry a stale active_file from the previously
                // active document and prevent `handle_actions` from calling
                // `request_open` (the `changed` check would see matching
                // active_files).  The stale active_file would also cause
                // `commit_open` to skip `buffer_changed` processing and reuse
                // the old large_file_mode.
                comp.clear_direct_active();
                // Preserve the current explorer state so the tree does not
                // flicker or rebuild when opening a file.
                let exp_items = comp.format_cached_explorer_items();
                // Snapshot ALL current explorer UI state from work_content so the
                // explorer subtree is byte-identical across the loading commit.
                //
                // The explorer renders from `explorer_panel_items` and the widget
                // tree fingerprint keys on `explorer_panel_items.len()`. Building
                // the loading chrome with `explorer_panel_items = None` blanks the
                // explorer panel for the loading frame and forces a full widget
                // tree rebuild (panel gone → panel back), which is the visible
                // "explorer disappears and comes back" flicker on file open. By
                // carrying the existing panel items/title/empty fields unchanged,
                // opening a file only mutates editor/tab/active-document state and
                // the explorer subtree is left untouched.
                let (
                    exp_scroll,
                    exp_query,
                    exp_active,
                    exp_has_ws,
                    exp_panel_items,
                    exp_panel_title,
                    exp_empty_button,
                    exp_empty_message,
                    exp_ext_sidebar,
                ) = app
                    .work_content
                    .as_ref()
                    .map(|wc| {
                        (
                            wc.explorer_scroll_top,
                            wc.explorer_search_query.clone(),
                            wc.explorer_search_active,
                            wc.explorer_has_workspace,
                            wc.explorer_panel_items.clone(),
                            wc.explorer_panel_title.clone(),
                            wc.explorer_empty_button.clone(),
                            wc.explorer_empty_message.clone(),
                            wc.extension_sidebar_items.clone(),
                        )
                    })
                    .unwrap_or((0, String::new(), false, false, None, None, None, None, None));
                let loading_wc = crate::gui::ShellWorkContent {
                    editor_body: None,
                    editor_tabs: None,
                    editor_breadcrumb: None,
                    explorer_items: exp_items,
                    explorer_panel_items: exp_panel_items,
                    explorer_panel_title: exp_panel_title,
                    explorer_empty_button: exp_empty_button,
                    explorer_empty_message: exp_empty_message,
                    explorer_scroll_top: exp_scroll,
                    explorer_search_query: exp_query,
                    explorer_search_active: exp_active,
                    explorer_has_workspace: exp_has_ws,
                    active_file: Some(format!("buf:{}", path.to_string_lossy())),
                    suppress_empty_state: false,
                    terminal_tabs: None,
                    ai_panel_content: None,
                    ai_show_setup_cta: false,
                    ai_composer_placeholder: None,
                    ai_has_pending_proposal: false,
                    ai_quick_actions: false,
                    syntax_highlights: None,
                    editor_non_file_tabs: None,
                    active_tab_index: None,
                    extension_sidebar_items: exp_ext_sidebar,
                };
                if super::first_open_trace_enabled() {
                    eprintln!(
                        "ZAROXI_DEBUG_FIRST_OPEN: activation=explorer file={} read_token={} preserved_panel_items={}",
                        path.display(),
                        token,
                        loading_wc.explorer_panel_items.as_ref().map(|v| v.len()).unwrap_or(0),
                    );
                }
                app.tab_state.focus_tab(&crate::gui::window::destination::WorkbenchTabId::Editor);
                app.rail_selected_index = 0;
                app.cockpit_status_fingerprint = 0;
                return Some(loading_wc);
            }
            // Rail activation: switch active panel / open command
            match index {
                0 => { /* Explorer — toggle sidebar */ }
                1 => { /* Search */ }
                2 => { /* Source Control */ }
                3 => { /* Debug */ }
                4 => { /* Settings */ }
                5 => { /* Account */ }
                _ => {}
            }
            None
        }
        _ => None,
    }
}

// ── Widget action dispatch (moved from app/mod.rs) ──

impl super::GuiApp {
    pub fn dispatch_activation(&mut self, id: &WidgetId) -> Option<ShellWorkContent> {
        dispatch_activation(self, id)
    }

    pub fn handle_actions(&mut self, actions: Vec<zaroxi_core_engine_ui::WidgetAction>) {
        let mut needs_redraw = false;
        let mut content_changed = false;
        for action in actions {
            match action {
                zaroxi_core_engine_ui::WidgetAction::StateNeedsRedraw => {
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::FocusChanged(_prev_focus) => {
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::ScrollOffsetChanged(id, offset) => {
                    let old_offset = self.interaction.get_scroll_offset(&id);
                    let offset_delta = offset - old_offset;
                    self.interaction.set_scroll_offset(&id, offset);
                    if (id == WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR })
                        && offset_delta.abs() > 0.0001
                    {
                        let total_lines = self.editor_buffer.line_count().max(1) as f32;
                        let visible = self
                            .editor_viewport
                            .as_ref()
                            .map(|vp| lc::visible_lines_from_region(vp.content_rect.3) as f32)
                            .unwrap_or(1.0);
                        let max_scroll_lines = (total_lines - visible).max(1.0);
                        let line_delta = (offset_delta * max_scroll_lines).round() as isize;
                        if let Some(ref mut comp) = self.composition {
                            comp.pending_scroll_lines += line_delta;
                            comp.pending_refresh_reason = Some(
                                zaroxi_application_workspace::workspace_view::RefreshReason::CursorMoved,
                            );
                        }
                    }
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::Activated(ref id) => {
                    let content = self
                        .on_widget_activated
                        .as_mut()
                        .and_then(|handler| handler(id))
                        .or_else(|| dispatch_activation(self, id));

                    if let Some(ref wc) = content {
                        let changed = self.work_content.as_ref().is_none_or(|old| {
                            old.explorer_items != wc.explorer_items
                                || old.active_file != wc.active_file
                                || old.editor_body.as_ref().map(|b| &b.lines)
                                    != wc.editor_body.as_ref().map(|b| &b.lines)
                        });
                        if changed {
                            // Detect whether this is a fresh open or
                            // a reactivation of an already-open document.
                            // When the document already has cached state
                            // (parked in open_documents / document_view_states),
                            // skip the scroll/caret reset — the commit_open
                            // checkout path will restore the prior position.
                            //
                            // Also skip reset if a prior commit_open already
                            // restored view state this frame (triggered by an
                            // earlier activation of the same document).
                            let is_reactivation = wc
                                .active_file
                                .as_deref()
                                .and_then(|s| s.strip_prefix("buf:"))
                                .map(|key| {
                                    self.open_documents.contains_key(key)
                                        || self.document_view_states.contains_key(key)
                                })
                                .unwrap_or(false)
                                || self.restored_view_state_this_activation;

                            self.request_open(wc.clone());
                            // When a file opens, switch from Welcome
                            // to Editor mode so file-editor surfaces
                            // (minimap, diff) are visible.
                            if matches!(
                                self.tab_state.active(),
                                super::super::destination::WorkbenchTabId::Welcome
                            ) {
                                self.tab_state
                                    .focus_tab(&super::super::destination::WorkbenchTabId::Editor);
                                self.rail_selected_index = 0;
                                self.cockpit_status_fingerprint = 0;
                            }
                            content_changed = true;
                            // Only reset scroll/caret for genuinely new
                            // opens.  Reactivations restore their prior
                            // viewport via the commit_open checkout path.
                            //
                            // CRITICAL: save the outgoing document's view
                            // state BEFORE resetting scroll to 0.  If we
                            // reset first, commit_open reads `scroll_top=0`
                            // from the cleared metadata and corrupts the
                            // outgoing document's saved state with a
                            // zero-scroll — causing the previous file to
                            // always reopen at the top on the next tab
                            // switch back.
                            if !is_reactivation {
                                let saved_scroll = self
                                    .composition
                                    .as_ref()
                                    .and_then(|c| c.metadata.as_ref())
                                    .map(|m| m.editor_scroll_top_line)
                                    .unwrap_or(0);
                                let outgoing_key = self
                                    .committed_active_file
                                    .as_deref()
                                    .and_then(|s| s.strip_prefix("buf:"))
                                    .map(|s| s.to_string());
                                if let Some(ref prev_key) = outgoing_key {
                                    let vs = DocumentViewState::from_editor_and_scroll(
                                        &self.editor_buffer,
                                        saved_scroll,
                                    );
                                    if super::doc_lifecycle_trace_enabled() {
                                        eprintln!(
                                            "ZAROXI_DOC_LIFECYCLE: early_checkin scroll_saved={} key={}",
                                            saved_scroll, prev_key,
                                        );
                                    }
                                    self.document_view_states.insert(prev_key.clone(), vs);
                                    if !self.large_file_mode {
                                        self.open_documents
                                            .insert(prev_key.clone(), self.editor_buffer.clone());
                                    }
                                }
                                self.pending_scroll_frac = 0.0;
                                if let Some(ref mut comp) = self.composition {
                                    comp.reset_scroll_state();
                                }
                                let editor_id =
                                    WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                                self.interaction.set_scroll_offset(&editor_id, 0.0);
                            }
                            // Clear the guard after consuming it so it
                            // doesn't suppress resets for future opens.
                            self.restored_view_state_this_activation = false;
                        }
                    }
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::HoverChanged(_)
                | zaroxi_core_engine_ui::WidgetAction::Nothing => {}
            }
        }
        if needs_redraw || content_changed {
            self.request_render();
        }
    }
}
