/*!
Non-editor pointer routing and settings/workspace navigation for [`GuiApp`].

Owns the left-mouse hit routing (activity rail, destination sidebar,
settings rows/dropdowns, unified tab strip, and editor-surface clicks),
the async folder-picker outcome handling, and live settings application.
*/

use super::*;
use winit::event::ElementState;

impl GuiApp {
    /// Route a left mouse press/release to rail / sidebar / settings / tab
    /// strip / editor-surface behaviour.
    pub(super) fn on_mouse_left(&mut self, state: ElementState) {
        let (x, y) = match self.interaction.cursor_pos_f32() {
            Some(pos) => pos,
            None => {
                debug::click_trace("ZAROXI_CLICK: MouseInput — cursor_pos is None, skipping");
                return;
            }
        };
        debug::click_trace_fmt!(
            "ZAROXI_CLICK: MouseInput state={:?} x={:.1} y={:.1} btn_rect={:?}",
            state,
            x,
            y,
            self.explorer_button_rect
        );
        // Unified tab-strip click (file tabs + non-file workbench tabs).
        // Close hits remove the tab; tab hits focus it (file tabs switch
        // the active buffer, non-file tabs become the active tab).
        if let ElementState::Released = state {
            // ── Tab overflow arrows: scroll the strip, never select a tab ──
            if let Some((ax, ay, aw, ah)) = self.tab_arrow_left_rect
                && x >= ax
                && x < ax + aw
                && y >= ay
                && y < ay + ah
            {
                self.tab_state.scroll_offset = (self.tab_state.scroll_offset
                    - zaroxi_interface_widgets::WORKBENCH_TAB_W * 2.0)
                    .max(0.0);
                self.cockpit_status_fingerprint = 0;
                self.needs_render = true;
                return;
            }
            if let Some((ax, ay, aw, ah)) = self.tab_arrow_right_rect
                && x >= ax
                && x < ax + aw
                && y >= ay
                && y < ay + ah
            {
                self.tab_state.scroll_offset += zaroxi_interface_widgets::WORKBENCH_TAB_W * 2.0;
                self.cockpit_status_fingerprint = 0;
                self.needs_render = true;
                return;
            }
            let action = self.tab_hit_rects.iter().find_map(|h| {
                if let Some((cx, cy, cw, ch)) = h.close_rect
                    && x >= cx
                    && x < cx + cw
                    && y >= cy
                    && y < cy + ch
                {
                    debug::zft(
                        "hit_close",
                        format_args!(
                            "id={:?}  close_rect=({cx:.0},{cy:.0},{cw:.0},{ch:.0})  \
                                 body_rect=({:.0},{:.0},{:.0},{:.0})",
                            h.id, h.rect.0, h.rect.1, h.rect.2, h.rect.3,
                        ),
                    );
                    return Some((true, h.id.clone()));
                }
                let (tx, ty, tw, th) = h.rect;
                if x >= tx && x < tx + tw && y >= ty && y < ty + th {
                    debug::zft(
                        "hit_body",
                        format_args!("id={:?}  rect=({tx:.0},{ty:.0},{tw:.0},{th:.0})", h.id,),
                    );
                    return Some((false, h.id.clone()));
                }
                None
            });
            if action.is_none() {
                debug::zft(
                    "hit_none",
                    format_args!(
                        "click=({x:.0},{y:.0})  tab_hit_rects_count={}  scroll={:.1}",
                        self.tab_hit_rects.len(),
                        self.tab_state.scroll_offset,
                    ),
                );
            }
            if let Some((is_close, id)) = action {
                debug::zft(
                    "click_action_resolved",
                    format_args!(
                        "is_close={is_close}  id={:?}  \
                             is_editor={}  is_filebuffer={}  \
                             tab_active={:?}",
                        id,
                        id.is_editor(),
                        id.is_file_buffer(),
                        self.tab_state.active(),
                    ),
                );
                if is_close {
                    if let super::super::destination::WorkbenchTabId::FileBuffer(ref bid_str) = id {
                        debug::zft(
                            "close_begin",
                            format_args!(
                                "clicked={bid_str}  tab_active={:?}  meta_active={:?}",
                                self.tab_state.active(),
                                self.composition.as_ref().and_then(|c| c
                                    .metadata
                                    .as_ref()
                                    .and_then(|m| m.active_buffer.as_ref().map(|b| b.to_string()))),
                            ),
                        );
                        let mut needs_wc_rebuild = false;
                        // Track whether the last file was just closed
                        // so the tab-mode switch can be gated correctly.
                        let mut opened_empty_after_close = false;
                        if let Some(ref mut comp) = self.composition {
                            if let Some(meta) = comp.metadata.as_mut() {
                                if let Some(pos) = meta
                                    .opened_buffers
                                    .iter()
                                    .position(|it| it.buffer_id.to_string() == *bid_str)
                                {
                                    let was_active = meta
                                        .active_buffer
                                        .as_ref()
                                        .map(|a| a.to_string() == *bid_str)
                                        .unwrap_or(false);
                                    meta.opened_buffers.remove(pos);
                                    meta.opened_buffer_count = meta.opened_buffers.len();
                                    comp.pending_removed_buffer_ids.push(bid_str.clone());

                                    // Release the buffer from the workspace service
                                    // to free its content from memory.
                                    let bid: crate::ports::BufferId =
                                        crate::ports::BufferId(bid_str.clone());
                                    if let (Some(svc), Some(sid)) =
                                        (&self.workspace_service, &self.session_id)
                                    {
                                        if let Ok(resp) = pollster::block_on(svc.close_buffer(
                                            crate::ports::CloseBufferRequest {
                                                session_id: sid.clone(),
                                                buffer_id: bid.clone(),
                                            },
                                        )) {
                                            if resp.ok {
                                                if std::env::var("ZAROXI_DEBUG_MEMORY").as_deref()
                                                    == Ok("1")
                                                {
                                                    eprintln!("ZAROXI_MEMORY: closed buffer {bid}");
                                                }
                                            }
                                        }
                                    }

                                    if was_active {
                                        needs_wc_rebuild = true;
                                        if meta.opened_buffers.is_empty() {
                                            meta.active_buffer = None;
                                            meta.active_buffer_details = None;
                                            meta.visible_window = None;
                                            opened_empty_after_close = true;
                                            self.tab_state.open_or_focus_non_file(
                                                super::super::destination::WorkbenchTabId::Welcome,
                                            );
                                        } else {
                                            let new_idx = if pos > 0 { pos - 1 } else { 0 };
                                            let fallback = &meta.opened_buffers[new_idx];
                                            meta.active_buffer = Some(fallback.buffer_id.clone());
                                            meta.active_buffer_details =
                                                Some(crate::desktop::ActiveBufferDetails {
                                                    buffer_id: fallback.buffer_id.clone(),
                                                    display: fallback.display.clone(),
                                                    line_count: 0,
                                                });
                                            meta.opened_buffers[new_idx].active = true;
                                        }
                                        // Try to sync-fetch the fallback's visible lines
                                        // so build_work_content produces real content.
                                        if let Some(ref bid) = meta.active_buffer {
                                            if let Some(ref view) = self.workspace_view {
                                                if let Some(ref session) = self.session_id {
                                                    let req =
                                                        crate::ports::GetVisibleLinesRequest {
                                                            session_id: session.clone(),
                                                            buffer_id: bid.clone(),
                                                        };
                                                    if let Ok(resp) = pollster::block_on(
                                                        view.get_visible_lines(req),
                                                    ) {
                                                        let mut lines_vec: Vec<String> =
                                                            Vec::with_capacity(
                                                                resp.window.lines.len(),
                                                            );
                                                        for vl in resp.window.lines.iter() {
                                                            lines_vec.push(vl.text.clone());
                                                        }
                                                        let cursor_info = resp
                                                            .window
                                                            .lines
                                                            .iter()
                                                            .find(|vl| vl.is_cursor_line);
                                                        meta.visible_window =
                                                                    Some(crate::desktop::projections::VisibleWindowBasic {
                                                                        top_line: resp.window.top_line as usize,
                                                                        total_lines: resp.window.total_lines as usize,
                                                                        lines: lines_vec,
                                                                        cursor_line: cursor_info.map(|vl| vl.line_number as usize),
                                                                        cursor_column: cursor_info.and_then(|vl| vl.cursor_column.map(|c| c as usize)),
                                                                        selection_present: resp.window.lines.iter().any(|vl| vl.selection_intersects),
                                                                    });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if needs_wc_rebuild {
                                let wc = comp.build_work_content();
                                self.request_open(wc);
                                // Hard-invalidate editor text caches so
                                // stale shaped text from the closed file
                                // cannot survive into the next present.
                                self.cached_editor_data = None;
                                self.cached_editor_lines_hash = 0;
                                self.cached_editor_spans_version = 0;
                            }
                        }
                        // After closing a file tab, switch to file-editor
                        // mode — but only when at least one file remains.
                        // If the last file was just closed, Welcome is
                        // already active via open_or_focus_non_file above.
                        if !opened_empty_after_close {
                            self.tab_state
                                .focus_tab(&super::super::destination::WorkbenchTabId::Editor);
                        }
                        self.rail_selected_index =
                            self.tab_state.active().destination().rail_index();
                        self.cockpit_status_fingerprint = 0;
                        self.needs_render = true;
                        debug::zft(
                            "close_end",
                            format_args!(
                                "tab_active={:?}  meta_active={:?}  visible_window={}",
                                self.tab_state.active(),
                                self.composition.as_ref().and_then(|c| c
                                    .metadata
                                    .as_ref()
                                    .and_then(|m| m.active_buffer.as_ref().map(|b| b.to_string()))),
                                self.composition
                                    .as_ref()
                                    .and_then(|c| c.metadata.as_ref())
                                    .map(|m| m.visible_window.is_some())
                                    .unwrap_or(false),
                            ),
                        );
                    } else {
                        self.close_tab(&id);
                        // If closing Welcome left no tabs at all,
                        // recreate Welcome immediately.
                        if self.tab_state.entries().is_empty() {
                            self.tab_state.open_or_focus_non_file(
                                super::super::destination::WorkbenchTabId::Welcome,
                            );
                        }
                    }
                } else if id.is_editor() {
                    self.tab_state.focus_tab(&super::super::destination::WorkbenchTabId::Editor);
                    self.rail_selected_index = 0;
                    self.cockpit_status_fingerprint = 0;
                    self.needs_render = true;
                    if let Some(ref comp) = self.composition {
                        let summary = comp.latest_opened_buffers_summary();
                        let active_bi = summary
                            .active
                            .as_ref()
                            .and_then(|ab| summary.items.iter().position(|it| &it.buffer_id == ab))
                            .unwrap_or(0);
                        self.handle_actions(vec![zaroxi_core_engine_ui::WidgetAction::Activated(
                            zaroxi_core_engine_style::WidgetId::Tab { index: active_bi },
                        )]);
                    }
                } else if id.is_file_buffer() {
                    debug::zft(
                        "click_branch_file_body",
                        format_args!(
                            "ENTER  clicked={:?}  tab_active={:?}  meta_active={:?}",
                            id,
                            self.tab_state.active(),
                            self.composition.as_ref().and_then(|c| c
                                .metadata
                                .as_ref()
                                .and_then(|m| m.active_buffer.as_ref().map(|b| b.to_string()))),
                        ),
                    );
                    debug::zft(
                        "focus_click",
                        format_args!(
                            "clicked={:?}  tab_active_before={:?}",
                            id,
                            self.tab_state.active(),
                        ),
                    );
                    self.tab_state.focus_tab(&super::super::destination::WorkbenchTabId::Editor);
                    self.rail_selected_index = 0;
                    self.cockpit_status_fingerprint = 0;
                    self.needs_render = true;
                    if let Some(ref comp) = self.composition {
                        let summary = comp.latest_opened_buffers_summary();
                        let bid_str = match &id {
                            super::super::destination::WorkbenchTabId::FileBuffer(s) => s.as_str(),
                            _ => "",
                        };
                        if let Some(idx) =
                            summary.items.iter().position(|it| it.buffer_id.to_string() == bid_str)
                        {
                            debug::zft(
                                "focus_dispatch",
                                format_args!(
                                    "bid={bid_str}  item_idx={idx}  items_count={}",
                                    summary.items.len(),
                                ),
                            );
                            self.handle_actions(vec![
                                zaroxi_core_engine_ui::WidgetAction::Activated(
                                    zaroxi_core_engine_style::WidgetId::Tab { index: idx },
                                ),
                            ]);
                        } else {
                            debug::zft("focus_nomatch", format_args!("bid={bid_str}"));
                        }
                    }
                } else {
                    debug::zft(
                        "click_branch_non_file",
                        format_args!(
                            "id={:?}  id_is_editor={}  id_is_filebuffer={}",
                            id,
                            id.is_editor(),
                            id.is_file_buffer(),
                        ),
                    );
                    self.open_or_focus_tab(id);
                }
                return;
            }
        }
        // Rail item click: navigation intent — open/focus the
        // destination's root tab (Explorer focuses the file editor).
        if let ElementState::Released = state
            && let Some(idx) = self.rail_hovered_index
        {
            let dest = super::super::destination::WorkbenchDestination::from_rail_index(idx);
            let target = if dest.is_explorer() {
                super::super::destination::WorkbenchTabId::Editor
            } else {
                super::super::destination::WorkbenchTabId::DestinationRoot(dest)
            };
            self.open_or_focus_tab(target);
            return;
        }
        // Destination sidebar row click (Extensions list / Settings
        // categories): open or focus the corresponding detail tab.
        if let ElementState::Released = state {
            use super::super::destination::{WorkbenchDestination as D, WorkbenchTabId as T};
            let dest = self.tab_state.active().destination();
            if matches!(dest, D::Extensions | D::Settings) {
                let hit = self
                    .sidebar_row_hit_rects
                    .iter()
                    .position(|&(rx, ry, rw, rh)| x >= rx && x < rx + rw && y >= ry && y < ry + rh);
                if let Some(row) = hit {
                    let target = match dest {
                        D::Extensions => super::super::destination::extension_entries()
                            .get(row)
                            .map(|e| T::ExtensionDetail(e.id.clone())),
                        D::Settings => Some(T::SettingsSection(row)),
                        _ => None,
                    };
                    if let Some(t) = target {
                        self.open_or_focus_tab(t);
                        return;
                    }
                }
            }
        }
        // Settings panel row click: dropdown trigger toggles open,
        // dropdown option dispatches action + closes, toggle rows
        // dispatch action. Click outside any dropdown closes it.
        if let ElementState::Released = state {
            let dest = self.tab_state.active().destination();
            if matches!(dest, super::super::destination::WorkbenchDestination::Settings) {
                let hit = self.settings_hit_rects.iter().find(|h| {
                    let (rx, ry, rw, rh) = h.rect;
                    x >= rx && x < rx + rw && y >= ry && y < ry + rh
                });
                if let Some(h) = hit {
                    if h.is_option {
                        self.apply_settings_action(h.action.clone());
                        self.settings_dropdown.close();
                        self.cached_settings_popup = None;
                        self.needs_render = true;
                        return;
                    }
                    if h.is_trigger {
                        if let Some(ri) = h.row_index {
                            self.settings_dropdown.toggle(ri);
                        }
                        self.cached_settings_popup = None;
                        self.cockpit_status_fingerprint = 0;
                        self.needs_render = true;
                        return;
                    }
                    self.apply_settings_action(h.action.clone());
                    return;
                }
                if self.settings_dropdown.open_row.is_some() {
                    self.settings_dropdown.close();
                    self.cached_settings_popup = None;
                    self.cockpit_status_fingerprint = 0;
                    self.needs_render = true;
                }
            }
        }
        // Explorer search box focus: clicking the box grabs keyboard
        // focus; clicking anywhere else releases it (the filter itself
        // persists until cleared with Escape).
        if let ElementState::Released = state {
            let in_search = self
                .explorer_search_rect
                .is_some_and(|(sx, sy, sw, sh)| x >= sx && x < sx + sw && y >= sy && y < sy + sh);
            if in_search {
                if !self.explorer_search_active {
                    self.explorer_search_active = true;
                    self.explorer_caret_blink_epoch = Instant::now();
                    self.explorer_search_sel = None;
                    self.invalidate(InvalidationFlags::content());
                }
                return;
            } else if self.explorer_search_active {
                self.explorer_search_active = false;
                self.invalidate(InvalidationFlags::content());
            }
        }
        let actions = match state {
            ElementState::Pressed => {
                if let Some(ref mut tree) = self.widget_tree {
                    self.interaction.on_pointer_down(
                        tree,
                        x,
                        y,
                        zaroxi_core_engine_ui::PointerButton::Primary,
                    )
                } else {
                    Vec::new()
                }
            }
            ElementState::Released => {
                let mut explorer_activated = false;
                if let Some((bx, by, bw, bh)) = self.explorer_button_rect {
                    if x >= bx && x <= bx + bw && y >= by && y <= by + bh {
                        explorer_activated = true;
                        debug::click_trace_fmt!(
                            "ZAROXI_CLICK: RELEASE hit CTA rect=({:.1},{:.1},{:.1},{:.1}) click=({:.1},{:.1})",
                            bx,
                            by,
                            bw,
                            bh,
                            x,
                            y
                        );
                    } else {
                        debug::click_trace_fmt!(
                            "ZAROXI_CLICK: RELEASE outside CTA rect=({:.1},{:.1},{:.1},{:.1}) click=({:.1},{:.1})",
                            bx,
                            by,
                            bw,
                            bh,
                            x,
                            y
                        );
                    }
                } else {
                    debug::click_trace_fmt!(
                        "ZAROXI_CLICK: RELEASE btn_rect is None click=({:.1},{:.1})",
                        x,
                        y
                    );
                }
                if explorer_activated {
                    let id = zaroxi_core_engine_ui::WidgetId::button(lc::BTN_ID_EXPLORER_CTA);
                    debug::click_trace("ZAROXI_CLICK: dispatching Activated(Explorer CTA)");
                    self.handle_actions(vec![zaroxi_core_engine_ui::WidgetAction::Activated(id)]);
                    Vec::new()
                } else if let Some(ref mut tree) = self.widget_tree {
                    self.interaction.on_pointer_up(
                        tree,
                        x,
                        y,
                        zaroxi_core_engine_ui::PointerButton::Primary,
                    )
                } else {
                    Vec::new()
                }
            }
        };
        self.handle_actions(actions);

        if let ElementState::Pressed = state {
            editor_interaction::init_selection_from_click(self);
        }
        if let ElementState::Released = state {
            self.editor_buffer.end_selection();
        }
    }

    /// Apply a settings action and propagate to live systems.
    /// Theme changes update `theme_mode` so the next frame renders with the
    /// new palette; font changes update the preference so the renderer can
    /// resolve the next font load.
    pub(crate) fn apply_settings_action(&mut self, action: zaroxi_domain_settings::SettingsAction) {
        match &action {
            zaroxi_domain_settings::SettingsAction::SetTheme(pref) => {
                self.settings.theme = *pref;
                self.theme_mode = match pref {
                    zaroxi_domain_settings::ThemePreference::System => {
                        zaroxi_interface_theme::theme::ZaroxiTheme::System
                    }
                    zaroxi_domain_settings::ThemePreference::Dark => {
                        zaroxi_interface_theme::theme::ZaroxiTheme::Dark
                    }
                    zaroxi_domain_settings::ThemePreference::Light => {
                        zaroxi_interface_theme::theme::ZaroxiTheme::Light
                    }
                };
            }
            zaroxi_domain_settings::SettingsAction::SetFont(pref) => {
                self.settings.font = pref.clone();
            }
            zaroxi_domain_settings::SettingsAction::SetTelemetry(enabled) => {
                self.settings.telemetry.enabled = *enabled;
            }
        }
        self.cached_settings_popup = None;
        self.cockpit_status_fingerprint = 0;
        self.needs_render = true;
    }

    pub fn process_picker_result(&mut self) {
        if !self.picker_in_flight {
            return;
        }
        if let Some(ref rx) = self.pending_picker_rx
            && let Ok(outcome) = rx.try_recv()
        {
            self.pending_picker_rx = None;
            self.picker_in_flight = false;
            match outcome {
                PickerOutcome::Selected(path) => {
                    debug::click_trace_fmt!(
                        "ZAROXI_PICKER: thread result=Selected({})",
                        path.display()
                    );
                    debug::click_trace_fmt!(
                        "ZAROXI_DIAG: picker Selected({}) — composition exists={} explorer_actions exists={}",
                        path.display(),
                        self.composition.is_some(),
                        self.explorer_actions.is_some()
                    );
                    if let Some(ref mut actions) = self.explorer_actions {
                        let comp = match self.composition.as_mut() {
                            Some(c) => c,
                            None => {
                                debug::click_trace(
                                    "ZAROXI_DIAG: composition is None — cannot open workspace",
                                );
                                return;
                            }
                        };
                        let service = match self.workspace_service.clone() {
                            Some(s) => s,
                            None => {
                                debug::click_trace("ZAROXI_DIAG: workspace_service is None");
                                return;
                            }
                        };
                        let view = match self.workspace_view.clone() {
                            Some(v) => v,
                            None => {
                                debug::click_trace("ZAROXI_DIAG: workspace_view is None");
                                return;
                            }
                        };
                        debug::click_trace_fmt!(
                            "ZAROXI_DIAG: calling open_workspace with path={}",
                            path.display()
                        );
                        let pre_root = comp.workspace_root_path.clone();
                        let pre_items = comp.cached_explorer_items.len();
                        debug::click_trace_fmt!(
                            "ZAROXI_DIAG: BEFORE open_workspace — root={:?} cached_items={}",
                            pre_root,
                            pre_items
                        );
                        let content = actions.open_workspace(
                            comp,
                            service,
                            view,
                            &mut self.session_id,
                            &mut self.workspace_id,
                            path,
                        );
                        let post_root = comp.workspace_root_path.clone();
                        let post_items = comp.cached_explorer_items.len();
                        debug::click_trace_fmt!(
                            "ZAROXI_DIAG: AFTER open_workspace — root={:?} cached_items={} content_is_some={}",
                            post_root,
                            post_items,
                            content.is_some()
                        );
                        if let Some(ref wc) = content {
                            debug::click_trace_fmt!(
                                "ZAROXI_DIAG: work_content — empty_button={:?} panel_items_count={}",
                                wc.explorer_empty_button,
                                wc.explorer_panel_items.as_ref().map_or(0, |v| v.len())
                            );
                        }
                        if let Some(wc) = content {
                            self.request_open(wc);
                            self.last_widget_tree_fingerprint = None;
                            self.pending_scroll_frac = 0.0;
                            if let Some(ref mut comp) = self.composition {
                                comp.reset_scroll_state();
                            }
                            let editor_id = WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                            self.interaction.set_scroll_offset(&editor_id, 0.0);
                            self.request_render();
                        } else {
                            debug::click_trace(
                                "ZAROXI_DIAG: open_workspace returned None — explorer stays empty",
                            );
                        }
                    }
                }
                PickerOutcome::Cancelled => {
                    debug::click_trace("ZAROXI_PICKER: thread result=Cancelled");
                    let wc = if let Some(ref mut comp) = self.composition {
                        comp.set_status_message("No folder selected".to_string());
                        comp.build_work_content()
                    } else {
                        return;
                    };
                    self.request_open(wc);
                    self.last_widget_tree_fingerprint = None;
                    self.request_render();
                }
                PickerOutcome::Unavailable { reason, .. } => {
                    debug::click_trace_fmt!("ZAROXI_PICKER: thread result=Unavailable({})", reason);
                    let wc = if let Some(ref mut comp) = self.composition {
                        let msg = if reason.len() > 90 {
                            "Workspace picker unavailable — see log for details".to_string()
                        } else {
                            format!("Workspace picker unavailable: {}", reason)
                        };
                        comp.set_status_message(msg);
                        comp.build_work_content()
                    } else {
                        return;
                    };
                    self.request_open(wc);
                    self.last_widget_tree_fingerprint = None;
                    self.request_render();
                }
            }
        }
    }
}
