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

        // Was the document being closed the ACTIVE one?  Only the active
        // document's content lives in the live rope; every other open document
        // is parked in `open_documents` / `doc_buffers`.  This decides whether
        // the live content must be re-bound to the fallback below: closing the
        // active tab leaves the rope holding dead text, whereas closing an
        // inactive tab must leave the active rope untouched.
        let prev_active_canon = self
            .committed_active_file
            .as_deref()
            .map(|s| canonical_path_from_editor_id(s).to_string());
        let closing_active = prev_active_canon.as_deref() == Some(canon.as_str());

        // (2) EditorGroup is the sole tab authority: remove the editor and
        //     let it pick the next active editor (preview → last pinned →
        //     none).  This drives everything downstream.
        let closed_changed = self.editor_group.close(&canon);
        let next_active_path: Option<String> =
            self.editor_group.active_path().map(|p| p.to_string());
        // EditorGroup is the SOLE authority for the active document.  Its close
        // fallback (preview → last pinned) differs from the workspace service's
        // fallback (previous neighbor), so unless we push EditorGroup's choice
        // back into the service, the next `refresh_with_service` resurrects the
        // service's own active buffer.  That service-derived active buffer drives
        // both the explorer highlight and `build_work_content().active_file`, so
        // the drift makes the editor body / explorer highlight disagree with the
        // active tab.  Whether the fallback is a large-file (direct) buffer or a
        // normal service-backed buffer decides which projection we realign.
        let next_is_direct =
            next_active_path.as_deref().map(|p| self.doc_buffers.contains_key(p)).unwrap_or(false);

        let mut opened_removed = false;
        let mut service_unregistered = false;
        let mut service_active_aligned = false;
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

            // ── Realign the active-buffer authority to EditorGroup ──
            // The service's own close fallback may have selected a DIFFERENT
            // document than EditorGroup did.  Force the service (or the direct
            // large-file projection) to the exact document EditorGroup chose so
            // no later refresh can drift the explorer highlight / work content
            // away from the active tab.  Keyed by canonical identity via
            // `buffer_key_from_path`.
            if let Some(next) = next_active_path.as_deref() {
                let next_bid = crate::ports::BufferId(buffer_key_from_path(next));
                if next_is_direct {
                    // Large-file (direct) fallback: activate it in the direct
                    // projection so `refresh_with_service` keeps it authoritative.
                    comp.set_direct_buffer_active(next_bid);
                    service_active_aligned = true;
                } else if let (Some(svc), Some(sid)) = (&self.workspace_service, &sd) {
                    // Normal service-backed fallback: make the service's active
                    // buffer match EditorGroup's choice.
                    if let Ok(resp) = pollster::block_on(svc.set_active_buffer(
                        crate::ports::SetActiveBufferRequest {
                            session_id: sid.clone(),
                            buffer_id: next_bid,
                        },
                    )) {
                        service_active_aligned = resp.ok;
                    }
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

        // ── Identity truth: the closed tab's fallback, and confirmation that
        //    the active-buffer authority (service / direct projection) was
        //    realigned to EditorGroup so the explorer highlight can never
        //    diverge from the active tab after a close. ──
        if std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1")
            || std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1")
        {
            eprintln!(
                "ZAROXI_IDENTITY: close_tab result closed={canon} next_active={} next_is_direct={next_is_direct} service_active_aligned={service_active_aligned}",
                next_active_path.as_deref().unwrap_or("<none>"),
            );
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

        // (7) Reconcile committed_active_file to EditorGroup.active AND
        //     atomically re-bind the LIVE editor content object to the same
        //     fallback document.
        //
        // Path metadata alone is not enough: the closed file's text is still in
        // the live rope, and `build_work_content()` derives `editor_body` from
        // the (now stale) `visible_window`, which still belongs to the closed /
        // previously-active file.  If we only updated `committed_active_file`,
        // the next commit — where `active_file_changed` is false because we just
        // set the path — would keep that stale rope and later stamp it with the
        // fallback's owner, rendering the WRONG file's text under the fallback
        // tab.  So we bind the actual content object here, keyed by canonical
        // path, and drop the stale visible-window text so no foreign body text
        // can survive into the render source.
        match next_active_path.as_deref() {
            None => {
                self.committed_active_file = None;
                self.editor_buffer.replace_content("");
                self.active_rope_owner_path = None;
                self.tab_state
                    .open_or_focus_non_file(super::super::destination::WorkbenchTabId::Welcome);
            }
            Some(next) => {
                self.committed_active_file = Some(buffer_key_from_path(next));
                // Only rebind the live content when we closed the ACTIVE
                // document (the rope now holds dead text).  Closing an inactive
                // tab leaves the active document — and its rope — untouched.
                if closing_active {
                    self.rebind_live_content_to(next);
                }
            }
        }
        // When the active document changed (closed the active tab), drop the
        // stale visible-window text and re-key the active-buffer details to the
        // fallback's live content so `build_work_content()` cannot emit the
        // previous file's text as the new active `editor_body`.  (Closing an
        // inactive tab leaves the active window valid, so leave it alone.)
        if closing_active
            && let Some(ref mut comp) = self.composition
            && let Some(ref mut meta) = comp.metadata
        {
            meta.visible_window = None;
            if let Some(ref mut abd) = meta.active_buffer_details {
                abd.line_count = self.editor_buffer.line_count();
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
            // The live rope's content owner must equal the new active path (or
            // be intentionally cleared when nothing remains).  This is the hard
            // content-identity invariant that path metadata alone cannot give.
            debug_assert!(
                match (next_active_path.as_deref(), self.active_rope_owner_path.as_deref()) {
                    (Some(next), Some(owner)) => super::editor_group::same_document(next, owner),
                    // Owner may legitimately be None when the fallback content
                    // is not resident yet (blank frame → hydrate) or no tab
                    // remains; it must never point at a DIFFERENT document.
                    (_, None) => true,
                    (None, Some(_)) => false,
                },
                "close_transaction: rope owner {:?} does not match next active {:?}",
                self.active_rope_owner_path,
                next_active_path,
            );
        }
    }

    /// Atomically re-bind the LIVE editor content object (`editor_buffer` +
    /// `active_rope_owner_path` + `large_file_mode`) to the canonical `path`,
    /// sourcing the text EXCLUSIVELY from the path-keyed content stores.
    ///
    /// This is the content-identity counterpart to the active-tab metadata:
    /// after a close fallback (or any forced active-doc rebind) the rendered
    /// text MUST come from the store keyed by the new active path — never from
    /// a stale rope left over from the previous/closed file, and never from a
    /// `work_content.editor_body` summary that may have been built for a
    /// different document.  Resolution order:
    ///   1. `open_documents[path]` — a parked normal (Rope) document, checked
    ///      out whole so caret / selection / undo history are exactly restored.
    ///   2. `doc_buffers[path]` — a large-file PieceTable backend; the rope is
    ///      repopulated with the initial viewport window.
    ///   3. neither resident — blank the rope and drop ownership so a temporary
    ///      empty frame shows until the open pipeline hydrates the real file.
    ///      A blank frame is acceptable; foreign content is not.
    pub(crate) fn rebind_live_content_to(&mut self, path: &str) {
        if let Some(stored) = self.open_documents.remove(path) {
            self.editor_buffer = stored;
            self.active_rope_owner_path = Some(path.to_string());
            self.large_file_mode = false;
        } else if let Some(db) = self.doc_buffers.get(path) {
            let end = 200usize.min(db.total_lines());
            let lines: Vec<String> =
                db.lines_in_range(0, end.saturating_sub(1)).into_iter().map(|(_, s)| s).collect();
            if lines.is_empty() {
                self.editor_buffer.replace_content("");
                self.active_rope_owner_path = None;
            } else {
                self.editor_buffer.populate_from_lines(&lines, 0, 0);
                self.active_rope_owner_path = Some(path.to_string());
            }
            self.large_file_mode = true;
        } else {
            // Not resident: blank now; the follow-up request_open/commit_open
            // (or the render owner-guard rehydrate) loads the correct content.
            self.editor_buffer.replace_content("");
            self.active_rope_owner_path = None;
            self.large_file_mode = false;
        }
        if std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1")
            || std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1")
        {
            eprintln!(
                "ZAROXI_IDENTITY: rebind_live_content path={} owner={} large_file_mode={} rope_lines={} content_fingerprint={:#018x}",
                path,
                self.active_rope_owner_path.as_deref().unwrap_or("<none>"),
                self.large_file_mode,
                self.editor_buffer.line_count(),
                self.editor_content_fingerprint(),
            );
        }
    }

    /// Cheap, stable fingerprint of the LIVE editor content (rope) — a FNV-1a
    /// hash of the leading bytes mixed with the char count.  Used to PROVE that
    /// the rendered text object actually belongs to the active path, rather than
    /// trusting the owner-path label alone (the label was the exact thing that
    /// drifted in the wrong-content bug).
    pub(crate) fn editor_content_fingerprint(&self) -> u64 {
        let head = self.editor_buffer.raw_head(512);
        let mut h: u64 = 0xcbf29ce484222325;
        for b in head.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x00000100000001b3);
        }
        h ^= self.editor_buffer.char_count() as u64;
        h
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
            // Canonical rail activation: every destination opens/focuses its
            // stable WorkbenchTabId through the one tab authority.
            let dest = super::super::destination::WorkbenchDestination::from_rail_index(idx);
            self.activate_destination(dest);
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

#[cfg(test)]
mod close_identity_tests {
    use super::super::editor_group::BackendKind;
    use super::super::test_support::make_headless_app;
    use crate::desktop::{DesktopComposition, DesktopMetadata, OpenedBufferItem};
    use crate::ports;
    use crate::ports::BufferId;
    use std::sync::{Arc, Mutex};

    /// Fake WorkspaceService that RECORDS `set_active_buffer` calls and mimics
    /// the real service's close fallback (previous neighbor).  This is exactly
    /// the fallback policy that historically diverged from EditorGroup's
    /// (preview → last pinned), producing the wrong-content / explorer-highlight
    /// drift after a close.
    struct RecordingSvc {
        set_active_calls: Arc<Mutex<Vec<String>>>,
    }
    impl ports::WorkspaceService for RecordingSvc {
        fn boot_workspace(
            &self,
            _req: ports::WorkspaceBootRequest,
        ) -> ports::BoxFuture<'static, Result<ports::WorkspaceBootResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownWorkspace) })
        }
        fn open_buffer(
            &self,
            _req: ports::OpenBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::OpenBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn close_buffer(
            &self,
            _req: ports::CloseBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::CloseBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::CloseBufferResponse { ok: true }) })
        }
        fn list_open_buffers(
            &self,
            _req: ports::ListBuffersRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ListBuffersResponse, ports::UseCaseError>>
        {
            Box::pin(async {
                Ok(ports::ListBuffersResponse { buffer_ids: Vec::new(), active_buffer: None })
            })
        }
        fn set_active_buffer(
            &self,
            req: ports::SetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetActiveBufferResponse, ports::UseCaseError>>
        {
            self.set_active_calls.lock().unwrap().push(req.buffer_id.to_string());
            Box::pin(async { Ok(ports::SetActiveBufferResponse { ok: true }) })
        }
        fn get_active_buffer(
            &self,
            _req: ports::GetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetActiveBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_cursor(
            &self,
            _req: ports::SetEditorCursorRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetEditorCursorResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_editor_selection(
            &self,
            _req: ports::SetSelectionRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetSelectionResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn clear_editor_selection(
            &self,
            _req: ports::ClearSelectionRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ClearSelectionResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn get_editor_state(
            &self,
            _req: ports::GetEditorStateRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetEditorStateResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn set_viewport_state(
            &self,
            _req: ports::SetViewportRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SetViewportResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn scroll_viewport(
            &self,
            _req: ports::ScrollViewportRequest,
        ) -> ports::BoxFuture<'static, Result<ports::ScrollViewportResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn explain_active_buffer(
            &self,
            _req: ports::GetActiveBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::NoActiveBuffer) })
        }
        fn dispatch_command(
            &self,
            _req: ports::DispatchCommandRequest,
        ) -> ports::BoxFuture<'static, Result<ports::DispatchCommandResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn update_buffer(
            &self,
            _req: ports::UpdateBufferRequest,
        ) -> ports::BoxFuture<'static, Result<ports::UpdateBufferResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn apply_text_transaction(
            &self,
            _req: ports::ApplyTextTransactionRequest,
        ) -> ports::BoxFuture<
            'static,
            Result<ports::ApplyTextTransactionResponse, ports::UseCaseError>,
        > {
            Box::pin(async {
                Ok(ports::ApplyTextTransactionResponse {
                    ok: true,
                    state: ports::EditorState {
                        cursor: ports::EditorCursor::zero(),
                        selection: None,
                    },
                    content: None,
                })
            })
        }
        fn get_recent_commands(
            &self,
            _req: ports::GetRecentCommandsRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetRecentCommandsResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::GetRecentCommandsResponse { commands: Vec::new() }) })
        }
        fn get_recent_events(
            &self,
            _req: ports::GetRecentEventsRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetRecentEventsResponse, ports::UseCaseError>>
        {
            Box::pin(async { Ok(ports::GetRecentEventsResponse { events: Vec::new() }) })
        }
        fn get_session_snapshot(
            &self,
            _req: ports::GetSessionSnapshotRequest,
        ) -> ports::BoxFuture<'static, Result<ports::GetSessionSnapshotResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn create_checkpoint(
            &self,
            _req: ports::CreateCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::CreateCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn save_checkpoint(
            &self,
            _req: ports::SaveCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::SaveCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn load_checkpoint(
            &self,
            _req: ports::LoadCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::LoadCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
        fn restore_checkpoint(
            &self,
            _req: ports::RestoreCheckpointRequest,
        ) -> ports::BoxFuture<'static, Result<ports::RestoreCheckpointResponse, ports::UseCaseError>>
        {
            Box::pin(async { Err(ports::UseCaseError::UnknownSession) })
        }
    }

    fn opened(path: &str, active: bool) -> OpenedBufferItem {
        OpenedBufferItem {
            buffer_id: BufferId(format!("buf:{path}")),
            display: path.rsplit('/').next().map(|s| s.to_string()),
            active,
        }
    }

    use crate::gui::window::editor_buf::EditorBufferState;

    fn buf_with(text: &str) -> EditorBufferState {
        let mut b = EditorBufferState::empty();
        b.replace_content(text);
        b
    }

    fn visible_window_for(lines: &[&str]) -> crate::desktop::projections::VisibleWindowBasic {
        crate::desktop::projections::VisibleWindowBasic {
            top_line: 0,
            total_lines: lines.len(),
            lines: lines.iter().map(|s| s.to_string()).collect(),
            cursor_line: Some(0),
            cursor_column: Some(0),
            selection_present: false,
        }
    }

    /// THE regression for the deeper wrong-content bug: closing the ACTIVE tab
    /// must rebind the LIVE editor content object to the fallback file, not just
    /// the path metadata.  We assert the actual rope text fingerprint, not only
    /// the active path — proving `.gitignore`'s tab can never show `Cargo.toml`'s
    /// bytes even though the path metadata reports `.gitignore`.
    #[test]
    fn closing_active_tab_rebinds_live_content_to_fallback_not_stale_text() {
        const CARGO: &str = "[package]\nname = \"zaroxi\"\nedition = \"2024\"";
        const GITIGNORE: &str = "/target\n*.log\n.env";

        let mut app = make_headless_app();
        // Pin Cargo.toml then .gitignore; make Cargo.toml the ACTIVE editor.
        app.editor_group.open_or_activate_pinned(
            "/w/Cargo.toml".into(),
            "buf:/w/Cargo.toml".into(),
            "Cargo.toml".into(),
            BackendKind::Rope,
            true,
        );
        app.editor_group.open_or_activate_pinned(
            "/w/.gitignore".into(),
            "buf:/w/.gitignore".into(),
            ".gitignore".into(),
            BackendKind::Rope,
            true,
        );
        app.editor_group.activate_by_path("/w/Cargo.toml");

        // Live rope holds the ACTIVE file (Cargo.toml). The fallback (.gitignore)
        // is parked in open_documents with its own DISTINCT content.
        app.editor_buffer = buf_with(CARGO);
        app.active_rope_owner_path = Some("/w/Cargo.toml".to_string());
        app.committed_active_file = Some("buf:/w/Cargo.toml".to_string());
        app.open_documents.insert("/w/.gitignore".to_string(), buf_with(GITIGNORE));

        // Composition with a STALE visible_window belonging to Cargo.toml — the
        // exact projection that historically leaked into `.gitignore`'s body.
        let mut comp = DesktopComposition::new();
        comp.metadata = Some(DesktopMetadata {
            active_buffer: Some(BufferId("buf:/w/Cargo.toml".to_string())),
            opened_buffer_count: 2,
            opened_buffers: vec![opened("/w/Cargo.toml", true), opened("/w/.gitignore", false)],
            active_buffer_details: Some(crate::desktop::ActiveBufferDetails {
                buffer_id: BufferId("buf:/w/Cargo.toml".to_string()),
                display: Some("Cargo.toml".to_string()),
                line_count: 3,
            }),
            visible_window: Some(visible_window_for(&["[package]", "name = \"zaroxi\""])),
            ..Default::default()
        });
        app.composition = Some(comp);
        app.session_id = Some(ports::SessionId(zaroxi_kernel_types::Id::new()));
        app.workspace_service =
            Some(Arc::new(RecordingSvc { set_active_calls: Arc::new(Mutex::new(Vec::new())) }));

        let cargo_fp = {
            let mut a = make_headless_app();
            a.editor_buffer = buf_with(CARGO);
            a.editor_content_fingerprint()
        };
        let gitignore_fp = {
            let mut a = make_headless_app();
            a.editor_buffer = buf_with(GITIGNORE);
            a.editor_content_fingerprint()
        };

        // Close the ACTIVE Cargo.toml tab.
        app.close_editor_transactional("buf:/w/Cargo.toml");

        // Path identity moved to the fallback.
        assert_eq!(app.editor_group.active_path(), Some("/w/.gitignore"));
        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/.gitignore"));
        assert_eq!(app.active_rope_owner_path.as_deref(), Some("/w/.gitignore"));

        // CONTENT identity moved too: the live rope now holds .gitignore's bytes,
        // NOT Cargo.toml's.  This is the assertion the previous fix lacked.
        assert!(
            app.editor_buffer.raw_head(64).starts_with("/target"),
            "rope must show .gitignore content after close, got: {:?}",
            app.editor_buffer.raw_head(64),
        );
        assert_eq!(app.editor_content_fingerprint(), gitignore_fp, "rope must equal .gitignore");
        assert_ne!(
            app.editor_content_fingerprint(),
            cargo_fp,
            "rope must NOT still hold Cargo.toml content under the .gitignore tab",
        );

        // The stale Cargo.toml visible-window must be dropped so it cannot be
        // re-emitted as .gitignore's editor_body on the next build_work_content.
        let vw = app
            .composition
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .and_then(|m| m.visible_window.clone());
        assert!(vw.is_none(), "stale visible_window must be cleared on active-tab close");

        // Driving the real commit must NOT reintroduce foreign content.
        app.commit_open();
        assert_eq!(
            app.editor_content_fingerprint(),
            gitignore_fp,
            "commit_open after close must preserve .gitignore content",
        );
        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/.gitignore"));
        assert_eq!(app.active_rope_owner_path.as_deref(), Some("/w/.gitignore"));
    }

    /// Closing an INACTIVE tab must leave the ACTIVE document's live content
    /// untouched (guards the `closing_active` gate so we never blank the wrong
    /// document).
    #[test]
    fn closing_inactive_tab_preserves_active_live_content() {
        const A: &str = "AAAA content line\nsecond";
        const B: &str = "BBBB other file\nsecond";

        let mut app = make_headless_app();
        app.editor_group.open_or_activate_pinned(
            "/w/a.rs".into(),
            "buf:/w/a.rs".into(),
            "a.rs".into(),
            BackendKind::Rope,
            true,
        );
        app.editor_group.open_or_activate_pinned(
            "/w/b.rs".into(),
            "buf:/w/b.rs".into(),
            "b.rs".into(),
            BackendKind::Rope,
            true,
        );
        // a.rs is active (live rope); b.rs is parked.
        app.editor_group.activate_by_path("/w/a.rs");
        app.editor_buffer = buf_with(A);
        app.active_rope_owner_path = Some("/w/a.rs".to_string());
        app.committed_active_file = Some("buf:/w/a.rs".to_string());
        app.open_documents.insert("/w/b.rs".to_string(), buf_with(B));

        let mut comp = DesktopComposition::new();
        comp.metadata = Some(DesktopMetadata {
            active_buffer: Some(BufferId("buf:/w/a.rs".to_string())),
            opened_buffer_count: 2,
            opened_buffers: vec![opened("/w/a.rs", true), opened("/w/b.rs", false)],
            ..Default::default()
        });
        app.composition = Some(comp);
        app.session_id = Some(ports::SessionId(zaroxi_kernel_types::Id::new()));
        app.workspace_service =
            Some(Arc::new(RecordingSvc { set_active_calls: Arc::new(Mutex::new(Vec::new())) }));

        let a_fp = app.editor_content_fingerprint();

        // Close the INACTIVE b.rs.
        app.close_editor_transactional("buf:/w/b.rs");

        // Active identity + content are unchanged.
        assert_eq!(app.editor_group.active_path(), Some("/w/a.rs"));
        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/a.rs"));
        assert_eq!(app.active_rope_owner_path.as_deref(), Some("/w/a.rs"));
        assert_eq!(app.editor_content_fingerprint(), a_fp, "active content must be untouched");
        assert!(app.editor_buffer.raw_head(8).starts_with("AAAA"));
    }

    /// Regression: closing a NON-last active tab must realign the workspace
    /// service's active buffer to EditorGroup's fallback so a later refresh can
    /// never resurrect the service's own (previous-neighbor) choice — which is
    /// what made the explorer highlight / editor body drift away from the
    /// active tab.  EditorGroup is the single source of truth end to end.
    #[test]
    fn closing_middle_active_tab_realigns_service_to_editor_group_fallback() {
        let mut app = make_headless_app();
        let paths = ["/w/a.rs", "/w/b.rs", "/w/c.rs", "/w/d.rs"];
        for p in paths {
            app.editor_group.open_or_activate_pinned(
                p.to_string(),
                format!("buf:{p}"),
                p.to_string(),
                BackendKind::Rope,
                true,
            );
        }
        // Make the MIDDLE editor active (the divergence case).
        app.editor_group.activate_by_path("/w/b.rs");
        assert_eq!(app.editor_group.active_path(), Some("/w/b.rs"));

        // Composition metadata mirrors the open set with b active.
        let mut comp = DesktopComposition::new();
        comp.metadata = Some(DesktopMetadata {
            active_buffer: Some(BufferId("buf:/w/b.rs".to_string())),
            opened_buffer_count: 4,
            opened_buffers: vec![
                opened("/w/a.rs", false),
                opened("/w/b.rs", true),
                opened("/w/c.rs", false),
                opened("/w/d.rs", false),
            ],
            ..Default::default()
        });
        app.composition = Some(comp);
        app.session_id = Some(ports::SessionId(zaroxi_kernel_types::Id::new()));
        app.committed_active_file = Some("buf:/w/b.rs".to_string());

        let calls = Arc::new(Mutex::new(Vec::new()));
        app.workspace_service = Some(Arc::new(RecordingSvc { set_active_calls: calls.clone() }));

        // Close the active middle tab.
        app.close_editor_transactional("buf:/w/b.rs");

        // EditorGroup (sole authority) falls back to the last pinned editor.
        assert_eq!(
            app.editor_group.active_path(),
            Some("/w/d.rs"),
            "EditorGroup fallback must be the last pinned editor",
        );

        // The service was realigned to EditorGroup's choice (d), NOT left on
        // its own previous-neighbor fallback (a).
        let recorded = calls.lock().unwrap();
        assert!(
            recorded.iter().any(|s| s == "buf:/w/d.rs"),
            "service active must be realigned to editor-group fallback /w/d.rs; recorded={recorded:?}",
        );
        assert!(
            !recorded.iter().any(|s| s == "buf:/w/a.rs"),
            "service must NOT be left on the neighbor /w/a.rs; recorded={recorded:?}",
        );

        // Tab identity (committed_active_file) and the projection metadata both
        // follow EditorGroup — so explorer highlight and editor body cannot
        // point at a different document than the active tab.
        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/d.rs"));
        let meta_active = app
            .composition
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .and_then(|m| m.active_buffer.as_ref())
            .map(|b| b.to_string());
        assert_eq!(meta_active.as_deref(), Some("buf:/w/d.rs"));
    }

    /// Closing an INACTIVE tab must leave the active tab (and its identity)
    /// untouched: no service realignment toward the closed tab or its neighbor.
    #[test]
    fn closing_inactive_tab_keeps_active_identity_stable() {
        let mut app = make_headless_app();
        for p in ["/w/a.rs", "/w/b.rs", "/w/c.rs"] {
            app.editor_group.open_or_activate_pinned(
                p.to_string(),
                format!("buf:{p}"),
                p.to_string(),
                BackendKind::Rope,
                true,
            );
        }
        // Active is c (last-opened). Close the inactive a.
        assert_eq!(app.editor_group.active_path(), Some("/w/c.rs"));

        let mut comp = DesktopComposition::new();
        comp.metadata = Some(DesktopMetadata {
            active_buffer: Some(BufferId("buf:/w/c.rs".to_string())),
            opened_buffer_count: 3,
            opened_buffers: vec![
                opened("/w/a.rs", false),
                opened("/w/b.rs", false),
                opened("/w/c.rs", true),
            ],
            ..Default::default()
        });
        app.composition = Some(comp);
        app.session_id = Some(ports::SessionId(zaroxi_kernel_types::Id::new()));
        app.committed_active_file = Some("buf:/w/c.rs".to_string());

        let calls = Arc::new(Mutex::new(Vec::new()));
        app.workspace_service = Some(Arc::new(RecordingSvc { set_active_calls: calls.clone() }));

        app.close_editor_transactional("buf:/w/a.rs");

        // Active identity is unchanged.
        assert_eq!(app.editor_group.active_path(), Some("/w/c.rs"));
        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/c.rs"));
        // Any realignment must target the still-active c, never the closed a.
        let recorded = calls.lock().unwrap();
        assert!(
            !recorded.iter().any(|s| s == "buf:/w/a.rs"),
            "must never realign to the closed tab; recorded={recorded:?}",
        );
    }
}
