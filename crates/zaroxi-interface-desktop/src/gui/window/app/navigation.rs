/*!
Non-editor pointer routing and settings/workspace navigation for [`GuiApp`].

Owns the left-mouse hit routing (activity rail, destination sidebar,
settings rows/dropdowns, unified tab strip, and editor-surface clicks),
the async folder-picker outcome handling, and live settings application.
*/

use super::*;
use winit::event::ElementState;

impl GuiApp {
    /// Close a file tab by tab / buffer-id string.  Thin wrapper that
    /// normalizes the identity and delegates to the single transactional
    /// close path.  Used by close-button clicks and middle-click.
    pub(crate) fn close_file_tab(&mut self, bid_str: &str) {
        self.close_editor_transactional(bid_str);
    }

    /// The single transactional close flow, shared by mouse close and
    /// `Ctrl+W`.  It resolves the exact canonical document to close and
    /// then mutates EVERY editor-state structure that owns a piece of it,
    /// keyed exclusively by canonical identity so nothing is left behind:
    ///
    /// 1. resolve the canonical active tab/file to close
    /// 2. remove it from `EditorGroup` (sole tab authority)
    /// 3. remove it from `opened_buffers` (+ service release, removal marker)
    /// 4. update `active_buffer` to `EditorGroup.active`
    /// 5. reconcile `committed_active_file` to `EditorGroup.active`
    /// 6. release the closed document's content state (`doc_buffers`,
    ///    `open_documents`, `document_view_states`)
    /// 7. rebuild work content and reconcile / normalize
    /// 8. assert (debug) that no closed path remains active / open / visible
    ///
    /// `target` may be either the canonical path or the `buf:<path>` form;
    /// both resolve to the same document.
    pub(crate) fn close_editor_transactional(&mut self, target: &str) {
        use super::editor_group::{buffer_key_from_path, canonical_path_from_editor_id};
        let canon = canonical_path_from_editor_id(target).to_string();
        let buffer_key = buffer_key_from_path(&canon);

        // (2) EditorGroup is the sole tab authority: remove the editor and
        //     let it pick the next active editor (preview → last pinned →
        //     none).  This drives everything downstream.
        let closed_changed = self.editor_group.close(&canon);
        let next_active_path: Option<String> =
            self.editor_group.active_path().map(|p| p.to_string());

        let mut opened_removed = false;
        let mut service_unregistered = false;
        {
            let Some(ref mut comp) = self.composition else { return };
            let sd = self.session_id.clone();
            if let Some(meta) = comp.metadata.as_mut() {
                // (3) Remove from opened_buffers by canonical identity (not
                //     raw string equality) so `buf:path`/`path` drift can
                //     never leave a stale ghost tab behind.
                let before = meta.opened_buffers.len();
                meta.opened_buffers.retain(|it| {
                    !super::editor_group::same_document(&it.buffer_id.to_string(), &canon)
                });
                opened_removed = meta.opened_buffers.len() != before;
                meta.opened_buffer_count = meta.opened_buffers.len();

                // (4/5) Active follows EditorGroup, the tab authority.
                match next_active_path.as_deref() {
                    None => {
                        meta.active_buffer = None;
                        meta.active_buffer_details = None;
                        meta.visible_window = None;
                    }
                    Some(next) => {
                        let next_key = buffer_key_from_path(next);
                        let next_bid = crate::ports::BufferId(next_key.clone());
                        for it in meta.opened_buffers.iter_mut() {
                            it.active =
                                super::editor_group::same_document(&it.buffer_id.to_string(), next);
                        }
                        let display = meta
                            .opened_buffers
                            .iter()
                            .find(|it| {
                                super::editor_group::same_document(&it.buffer_id.to_string(), next)
                            })
                            .and_then(|it| it.display.clone())
                            .or_else(|| next.rsplit('/').next().map(|s| s.to_string()));
                        meta.active_buffer = Some(next_bid.clone());
                        meta.active_buffer_details = Some(crate::desktop::ActiveBufferDetails {
                            buffer_id: next_bid,
                            display,
                            line_count: meta
                                .active_buffer_details
                                .as_ref()
                                .map(|d| d.line_count)
                                .unwrap_or(0),
                        });
                    }
                }
            }
            comp.pending_removed_buffer_ids.push(buffer_key.clone());
            comp.direct_buffer_ids
                .retain(|b| !super::editor_group::same_document(&b.to_string(), &canon));

            // Release the closed buffer from the workspace service.
            let bid: crate::ports::BufferId = crate::ports::BufferId(buffer_key.clone());
            if let (Some(svc), Some(sid)) = (&self.workspace_service, &sd)
                && let Ok(resp) =
                    pollster::block_on(svc.close_buffer(crate::ports::CloseBufferRequest {
                        session_id: sid.clone(),
                        buffer_id: bid.clone(),
                    }))
            {
                service_unregistered = resp.ok;
                if resp.ok && std::env::var("ZAROXI_DEBUG_MEMORY").as_deref() == Ok("1") {
                    eprintln!("ZAROXI_MEMORY: closed buffer {bid}");
                }
            }
        }

        // (6) Release the closed document's content state.  ALL of these
        //     maps are keyed by canonical path — the previous code removed
        //     by the raw `buf:` tab id and silently leaked every entry,
        //     which is what let closed files bleed content and grow RAM.
        let doc_buffer_removed = self.doc_buffers.remove(&canon).is_some();
        let open_doc_removed = self.open_documents.remove(&canon).is_some();
        let view_state_removed = self.document_view_states.remove(&canon).is_some();
        // If the live rope belonged to the closed file, drop that ownership so
        // its content can never be re-presented under the next active tab.
        if self.active_rope_owner_path.as_deref() == Some(canon.as_str()) {
            self.active_rope_owner_path = None;
        }
        if self.owner_reload_attempted_for.as_deref() == Some(canon.as_str()) {
            self.owner_reload_attempted_for = None;
        }
        if doc_buffer_removed && std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1") {
            eprintln!(
                "ZAROXI_DOC_LIFECYCLE: unregister path={canon} backend=piece_table reason=tab_closed"
            );
        }
        if self.doc_buffers.is_empty() {
            self.large_file_mode = false;
        }

        // ── Close-release truth (memory/state, not just UI) ──
        if std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1")
            || std::env::var("ZAROXI_DEBUG_MEMORY").as_deref() == Ok("1")
        {
            eprintln!(
                "ZAROXI_DOC_LIFECYCLE: close_release path={canon} editor_removed={closed_changed} opened_removed={opened_removed} doc_buffer_removed={doc_buffer_removed} open_doc_removed={open_doc_removed} view_state_removed={view_state_removed} service_unregistered={service_unregistered}",
            );
            // The only caches that intentionally survive a close are the
            // per-ACTIVE-file syntax/render caches (line_syntax_cache,
            // cached_editor_data, latest_spans). They are keyed to the active
            // document and are fully reset by commit_open on the next file
            // switch (buffer_changed branch), so they are bounded to a single
            // document and never accumulate per closed file. Report if the
            // closed path itself somehow still holds resident content.
            if self.doc_buffers.contains_key(&canon) || self.open_documents.contains_key(&canon) {
                eprintln!(
                    "ZAROXI_DOC_LIFECYCLE: close_release_resident_cache path={canon} reason=unexpected_retained_content",
                );
            }
        }

        // Keep the workbench tab state in sync (non-authoritative mirror).
        self.tab_state
            .close_tab(&super::super::destination::WorkbenchTabId::FileBuffer(buffer_key.clone()));

        if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
            eprintln!(
                "ZAROXI_VISIBLE_TAB_MODEL: close_transaction closed={canon} next_active={} editor_removed={} {}",
                next_active_path.as_deref().unwrap_or("<none>"),
                closed_changed,
                self.editor_group.diagnostic_line(),
            );
        }

        // (7) Reconcile committed_active_file to EditorGroup.active and
        //     rebuild the work content for the fallback document.
        match next_active_path.as_deref() {
            None => {
                self.committed_active_file = None;
                self.editor_buffer.replace_content("");
                self.tab_state
                    .open_or_focus_non_file(super::super::destination::WorkbenchTabId::Welcome);
            }
            Some(next) => {
                self.committed_active_file = Some(buffer_key_from_path(next));
            }
        }
        if let Some(ref mut comp) = self.composition {
            let wc = comp.build_work_content();
            self.request_open(wc);
            self.tab_state.focus_tab(&super::super::destination::WorkbenchTabId::Editor);
            self.rail_selected_index = 0;
        }
        self.rail_selected_index = self.tab_state.active().destination().rail_index();
        self.cockpit_status_fingerprint = 0;
        self.needs_render = true;

        // (8) Debug assertion: the closed path must not survive anywhere.
        #[cfg(debug_assertions)]
        {
            debug_assert!(
                !self
                    .editor_group
                    .all_ids()
                    .iter()
                    .any(|id| super::editor_group::same_document(id, &canon)),
                "close_transaction: {canon} still present in EditorGroup",
            );
            debug_assert!(
                !self.doc_buffers.contains_key(&canon),
                "close_transaction: {canon} still present in doc_buffers",
            );
        }
    }

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
                    - zaroxi_interface_widgets::FILE_TAB_W * 2.0)
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
                self.tab_state.scroll_offset += zaroxi_interface_widgets::FILE_TAB_W * 2.0;
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
                        debug::zft("close_begin", format_args!("clicked={bid_str}"));
                        self.close_file_tab(bid_str);
                    } else {
                        self.close_tab(&id);
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
                    // Editor click: dispatch Tab activation for the active editor.
                    let tabs = self.editor_group.visible_tabs();
                    let active_idx = tabs.iter().position(|t| t.is_active).unwrap_or(0);
                    self.handle_actions(vec![zaroxi_core_engine_ui::WidgetAction::Activated(
                        zaroxi_core_engine_style::WidgetId::Tab { index: active_idx },
                    )]);
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
                    self.tab_state.focus_tab(&super::super::destination::WorkbenchTabId::Editor);
                    self.rail_selected_index = 0;
                    self.cockpit_status_fingerprint = 0;
                    self.needs_render = true;
                    let bid_str = match &id {
                        super::super::destination::WorkbenchTabId::FileBuffer(s) => s,
                        _ => "",
                    };
                    // Resolve index from EditorGroup (sole tab authority).
                    let tabs = self.editor_group.visible_tabs();
                    if let Some(idx) = tabs.iter().position(|t| t.buffer_id == bid_str) {
                        debug::zft(
                            "focus_dispatch",
                            format_args!(
                                "bid={bid_str}  item_idx={idx}  tabs_count={}",
                                tabs.len(),
                            ),
                        );
                        self.handle_actions(vec![zaroxi_core_engine_ui::WidgetAction::Activated(
                            zaroxi_core_engine_style::WidgetId::Tab { index: idx },
                        )]);
                    } else {
                        debug::zft("focus_nomatch", format_args!("bid={bid_str}"));
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
