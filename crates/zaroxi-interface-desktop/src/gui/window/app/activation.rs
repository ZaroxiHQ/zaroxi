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
            result.ok().map(|_| comp.build_work_content())
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

                if let Some(ref mut actions) = app.explorer_actions {
                    if comp.is_explorer_item_dir(resolved) {
                        return actions.toggle_directory(comp, resolved);
                    } else {
                        let service = app.workspace_service.clone()?;
                        let view = app.workspace_view.clone()?;
                        let session = app.session_id.clone()?;
                        return actions.open_file(
                            comp,
                            service,
                            view,
                            session,
                            app.workspace_id,
                            resolved,
                        );
                    }
                }
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
