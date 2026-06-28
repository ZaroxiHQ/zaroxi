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

use super::GuiApp;

/// Dispatch a WidgetId activation to DesktopComposition domain actions.
/// Used by the explorer CTA button and by the standard activation handler.
pub(crate) fn dispatch_activation(app: &mut GuiApp, id: &WidgetId) -> Option<ShellWorkContent> {
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
        WidgetId::Button { index }
            if *index == lc::BTN_ID_AI_EXPLAIN
                || *index == lc::BTN_ID_AI_REVIEW
                || *index == lc::BTN_ID_AI_APPLY
                || *index == lc::BTN_ID_AI_REJECT =>
        {
            let _ = service;
            Some(comp.build_work_content())
        }
        WidgetId::TextInput { .. } => None,
        WidgetId::Tab { index } => {
            if super::first_open_trace_enabled() {
                eprintln!("ZAROXI_DEBUG_FIRST_OPEN: activation=tab tab_index={}", *index);
            }
            if std::env::var("ZAROXI_DEBUG_TAB_CLICK").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DEBUG_TAB_CLICK: tab_index={} items_len={}",
                    *index,
                    comp.latest_opened_buffers_summary().items.len(),
                );
            }
            let items = comp.latest_opened_buffers_summary().items;
            if *index < items.len() {
                let entry = items.get(*index)?;
                let buffer_id = entry.buffer_id.clone();
                if std::env::var("ZAROXI_DEBUG_TAB_CLICK").as_deref() == Ok("1") {
                    let is_direct = comp.direct_buffer_ids.iter().any(|b| *b == buffer_id);
                    let disp = entry.display.as_deref().unwrap_or("<none>");
                    eprintln!(
                        "ZAROXI_DEBUG_TAB_CLICK: buffer_id={} display={} is_direct={} items={}",
                        buffer_id,
                        disp,
                        is_direct,
                        items.len(),
                    );
                }
                // Large-file (direct) buffers are not known to the workspace
                // service.  Activate them locally through the composition.
                if comp.direct_buffer_ids.iter().any(|b| *b == buffer_id) {
                    if std::env::var("ZAROXI_DEBUG_TAB_CLICK").as_deref() == Ok("1") {
                        eprintln!(
                            "ZAROXI_DEBUG_TAB_CLICK: direct_buffer activation for={}",
                            buffer_id,
                        );
                    }
                    comp.set_direct_buffer_active(buffer_id.clone());
                    app.tab_state
                        .focus_tab(&crate::gui::window::destination::WorkbenchTabId::Editor);
                    app.rail_selected_index = 0;
                    app.cockpit_status_fingerprint = 0;
                    return Some(comp.build_work_content());
                }
                // Service-backed buffer: clear any previously-active direct
                // buffer so the service-reported active wins in the refresh
                // inside set_active_buffer_and_get_shell_context.
                if let Some(ref mut md) = comp.metadata {
                    if let Some(ref active) = md.active_buffer {
                        if comp.direct_buffer_ids.contains(active) {
                            md.active_buffer = None;
                            for it in &mut md.opened_buffers {
                                it.active = false;
                            }
                        }
                    }
                }
                let result =
                    pollster::block_on(crate::actions::set_active_buffer_and_get_shell_context(
                        comp,
                        service.clone(),
                        view.clone(),
                        session,
                        app.workspace_id,
                        buffer_id.clone(),
                    ));
                if std::env::var("ZAROXI_DEBUG_TAB_CLICK").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_DEBUG_TAB_CLICK: service_activation result={:?} buffer_id={}",
                        result.as_ref().map(|_| "ok").unwrap_or("err"),
                        buffer_id,
                    );
                }
                return result.ok().map(|_| comp.build_work_content());
            }
            // Non-file tab — switch rail destination.
            let non_file_idx = *index - items.len();
            if let Some(wc) = &app.work_content {
                if let Some(nf_tabs) = &wc.editor_non_file_tabs {
                    if let Some((_, kind)) = nf_tabs.get(non_file_idx) {
                        app.rail_selected_index = *kind;
                        return Some(comp.build_work_content());
                    }
                }
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
                app.rail_selected_index = 4;
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
                // Instead of `block_on`'ing the disk read on the UI thread (~1 s
                // for huge files) before `request_open`, resolve the path
                // cheaply, schedule a tokened background read, and return instant
                // loading chrome. `GuiApp::poll_read_results` activates the buffer
                // and calls `request_open` once the read lands. Stale reads (a
                // newer file clicked) are dropped by read token.
                let service = app.workspace_service.clone()?;
                let session = app.session_id.clone()?;
                let item_id = comp.get_explorer_item_id_at(resolved)?;
                let path = comp.maybe_explorer.as_ref()?.get_entry_path(&item_id)?;
                // Dedup: if this file is already open, focus the existing tab
                // instead of opening a duplicate.
                {
                    let summary = comp.latest_opened_buffers_summary();
                    let path_str = path.to_string_lossy();
                    if let Some(existing) =
                        summary.items.iter().find(|it| it.display.as_deref() == Some(&*path_str))
                    {
                        let active = summary.active.as_ref();
                        if active != Some(&existing.buffer_id) {
                            let buffer_id = existing.buffer_id.clone();
                            let service = app.workspace_service.clone()?;
                            let view = app.workspace_view.clone()?;
                            let session = app.session_id.clone()?;
                            let workspace_id = app.workspace_id;
                            let _ = pollster::block_on(
                                crate::actions::set_active_buffer_and_get_shell_context(
                                    comp,
                                    service,
                                    view,
                                    session,
                                    workspace_id,
                                    buffer_id,
                                ),
                            );
                            let wc = comp.build_work_content();
                            app.tab_state.focus_tab(
                                &crate::gui::window::destination::WorkbenchTabId::Editor,
                            );
                            app.rail_selected_index = 0;
                            app.cockpit_status_fingerprint = 0;
                            app.request_open(wc);
                            app.needs_render = true;
                        }
                        return None;
                    }
                }
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
                            self.pending_scroll_frac = 0.0;
                            if let Some(ref mut comp) = self.composition {
                                comp.reset_scroll_state();
                            }
                            let editor_id = WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                            self.interaction.set_scroll_offset(&editor_id, 0.0);
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
