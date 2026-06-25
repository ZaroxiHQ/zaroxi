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
            let items = comp.latest_opened_buffers_summary().items;
            if *index < items.len() {
                let entry = items.get(*index)?;
                let buffer_id = entry.buffer_id.clone();
                let result =
                    pollster::block_on(crate::actions::set_active_buffer_and_get_shell_context(
                        comp,
                        service.clone(),
                        view.clone(),
                        session,
                        app.workspace_id,
                        buffer_id,
                    ));
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
                            app.active_tab =
                                crate::gui::window::destination::WorkbenchTabId::Editor;
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
                return Some(comp.build_work_content());
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
