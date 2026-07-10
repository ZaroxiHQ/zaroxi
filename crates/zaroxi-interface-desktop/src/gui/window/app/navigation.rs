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
        // stale visible-window text so `build_work_content()` cannot emit the
        // previous file's text as the new active `editor_body`.  (Closing an
        // inactive tab leaves the active window valid, so leave it alone.)
        // NOTE: `active_buffer_details.line_count` and the fallback's scroll are
        // already re-keyed to the fallback's own content by
        // `rebind_live_content_to` above — do NOT overwrite them here (that would
        // clobber the restored large-file total and the restored scroll).
        if closing_active
            && let Some(ref mut comp) = self.composition
            && let Some(ref mut meta) = comp.metadata
        {
            meta.visible_window = None;
        }
        if let Some(ref mut comp) = self.composition {
            let mut wc = comp.build_work_content();
            // ── Content + view-state authority guard (active close) ──
            // `rebind_live_content_to` above is the SOLE authority for the
            // fallback's content AND view state on an active close: it restored
            // the rope bytes, rope ownership, caret, and scroll from the
            // fallback's own resident stores (or disk, or blank). When the
            // fallback is NOT resident the rope is blank, and the
            // `build_work_content()` `editor_body` here is a stale re-projection
            // of the CLOSED file (cleared visible_window / presenter snippet)
            // that `commit_open` would materialize into the blank rope, stamping
            // foreign bytes under the fallback identity. Strip it so `commit_open`
            // takes the no-materialize loading path; the async pipeline hydrates
            // the real content. (When the fallback IS resident the rope already
            // has content — `editor_body` is kept but `commit_open`'s
            // `rope_already_owns_active` / large-file reactivation guards prevent
            // it from overwriting the restored viewport.)
            // A blank frame is acceptable; foreign content is not.
            if closing_active && self.editor_buffer.char_count() == 0 {
                wc.editor_body = None;
            }
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
    /// AND restore that document's saved VIEW STATE (scroll + caret), sourcing
    /// everything EXCLUSIVELY from the path-keyed resident stores.
    ///
    /// This is the content-identity AND view-state-identity counterpart to the
    /// active-tab metadata: after a close fallback the rendered text MUST come
    /// from the store keyed by the new active path — never from a stale rope
    /// left over from the closed file, and never from a `work_content.editor_body`
    /// summary built for a different document — and the viewport MUST resume at
    /// the fallback's last scroll/caret rather than snapping to top-of-file.
    /// This close-driven reactivation is a real view-state event even though
    /// `commit_open` will skip content rematerialization (`active_file_changed`
    /// is false because the close pre-set `committed_active_file`); if the
    /// checkout path there is skipped, the restore MUST still happen here.
    ///
    /// Resident state beats disk every time.  Resolution order:
    ///   1. `open_documents[path]` — a parked normal (Rope) document, checked
    ///      out whole so rope + caret + selection + undo history are exactly
    ///      restored; scroll comes from `document_view_states[path]`.
    ///   2. `doc_buffers[path]` — a large-file PieceTable backend; the rope is
    ///      repopulated with a window that COVERS the saved scroll position and
    ///      the caret is placed from the saved view state.
    ///   3. File on disk (normal size) — synchronously read the raw bytes so
    ///      the fallback tab's own content appears immediately after a close.
    ///      Guards on file size (< LARGE_THRESHOLD) so we never block on huge
    ///      files; those remain blank until the async pipeline hydrates them.
    ///   4. neither resident — blank the rope and drop ownership so a temporary
    ///      empty frame shows until the open pipeline hydrates the real file.
    ///      A blank frame is acceptable; foreign content is not.
    pub(crate) fn rebind_live_content_to(&mut self, path: &str) {
        // Consume the saved view state (scroll + caret) for this path. It is the
        // authoritative resume position for the fallback and is applied below
        // regardless of which backend supplies the bytes. Removing it (rather
        // than peeking) prevents a stale entry from leaking, mirroring the
        // `commit_open` checkout which also removes on restore.
        let saved_view = self.document_view_states.remove(path);
        let saved_scroll = saved_view.as_ref().map(|v| v.scroll_top).unwrap_or(0);
        let saved_caret_line = saved_view.as_ref().map(|v| v.caret_line).unwrap_or(0);
        let saved_caret_col = saved_view.as_ref().map(|v| v.caret_col).unwrap_or(0);
        let mut resident_source = "none";
        let mut large_total_lines: Option<usize> = None;

        if let Some(stored) = self.open_documents.remove(path) {
            // Normal resident doc: the parked full state already carries the
            // exact caret/selection/undo history, so keep it intact — rebuilding
            // the rope would drop undo history. Only the scroll (which lives in
            // composition metadata, not the buffer) needs restoring, below.
            self.editor_buffer = stored;
            self.active_rope_owner_path = Some(path.to_string());
            self.large_file_mode = false;
            resident_source = "open_documents";
        } else if let Some(db) = self.doc_buffers.get(path) {
            // Large resident doc: rebuild a rope window that COVERS the saved
            // scroll position (not just the first screenful) so the viewport
            // resumes where the user left it, then place the caret.
            let total = db.total_lines();
            large_total_lines = Some(total);
            let end = saved_scroll.saturating_add(200).min(total).max(1);
            let lines: Vec<String> =
                db.lines_in_range(0, end.saturating_sub(1)).into_iter().map(|(_, s)| s).collect();
            if lines.is_empty() {
                self.editor_buffer.replace_content("");
                self.active_rope_owner_path = None;
            } else {
                let cl = saved_caret_line.min(lines.len().saturating_sub(1));
                self.editor_buffer.populate_from_lines(&lines, cl, saved_caret_col);
                self.active_rope_owner_path = Some(path.to_string());
            }
            self.large_file_mode = true;
            resident_source = "doc_buffers";
        } else if let Ok(meta) = std::fs::metadata(path)
            && meta.len() < zaroxi_core_editor_largefile::DocumentBuffer::LARGE_THRESHOLD
        {
            match std::fs::read_to_string(path) {
                Ok(text) => {
                    let lines: Vec<String> = {
                        let mut v: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
                        if let Some(last) = v.last()
                            && last.is_empty()
                        {
                            v.pop();
                        }
                        v
                    };
                    if lines.is_empty() {
                        self.editor_buffer.replace_content("");
                    } else {
                        // Apply the saved caret when we have view state (a file
                        // that was resident-then-evicted); otherwise top-of-file.
                        let cl = saved_caret_line.min(lines.len().saturating_sub(1));
                        self.editor_buffer.populate_from_lines(&lines, cl, saved_caret_col);
                    }
                    self.active_rope_owner_path = Some(path.to_string());
                    self.large_file_mode = false;
                    resident_source = "disk";
                }
                Err(_) => {
                    self.editor_buffer.replace_content("");
                    self.active_rope_owner_path = None;
                    self.large_file_mode = false;
                }
            }
        } else {
            // Not resident: blank now; the follow-up request_open/commit_open
            // (or the render owner-guard rehydrate) loads the correct content.
            self.editor_buffer.replace_content("");
            self.active_rope_owner_path = None;
            self.large_file_mode = false;
        }

        // ── Restore the live viewport scroll into composition metadata ──
        // Scroll position is owned by `DesktopMetadata`, not the rope, so the
        // buffer restore above never touches it. `commit_open` will NOT restore
        // it either (its checkout is gated on `active_file_changed`, which is
        // false for a close-driven fallback), so this is the ONLY place the
        // fallback's scroll is re-established. Also refresh the active-buffer
        // line count so scroll clamping uses the true document length (for large
        // files that is the PieceTable total, not the loaded rope window).
        let content_resident = self.active_rope_owner_path.is_some();
        if let Some(ref mut comp) = self.composition
            && let Some(ref mut meta) = comp.metadata
        {
            // Only project a non-zero scroll when the content is actually
            // resident; a blank fallback frame must stay at the top.
            let effective_scroll = if content_resident { saved_scroll } else { 0 };
            meta.editor_scroll_top_line = effective_scroll;
            meta.editor_scroll_px = effective_scroll as f32 * lc::LINE_HEIGHT;
            if let Some(ref mut abd) = meta.active_buffer_details {
                abd.line_count =
                    large_total_lines.unwrap_or_else(|| self.editor_buffer.line_count());
            }
        }
        // Mark that this activation restored a saved view state so the tab-strip
        // / activation path does not reset the scroll back to zero.
        self.restored_view_state_this_activation = content_resident && saved_view.is_some();

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
            eprintln!(
                "ZAROXI_VIEWSTATE: fallback_reactivate path={} content_source={} resident_view_state={} restore_allowed={} scroll={} caret_line={} caret_col={} view_state_restored={}",
                path,
                resident_source,
                saved_view.is_some(),
                content_resident,
                if content_resident { saved_scroll } else { 0 },
                saved_caret_line,
                saved_caret_col,
                self.restored_view_state_this_activation,
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
        // ── Minimap click/drag navigation ─────────────────────────────────
        // Intercept pointer events inside the minimap rail so clicking or
        // dragging the rail scrolls the editor viewport. A press jumps the
        // viewport to the clicked position; CursorMoved scrubs the viewport
        // continuously; release stops the drag.
        if let Some((mx, my, mw, mh)) = self.minimap_hit_rect
            && x >= mx
            && x < mx + mw
            && y >= my
            && y < my + mh
        {
            match state {
                ElementState::Pressed => {
                    self.minimap_dragging = true;
                    self.minimap_jump_to(y, my, mh);
                    return;
                }
                ElementState::Released => {
                    if self.minimap_dragging {
                        self.minimap_dragging = false;
                        return;
                    }
                    self.minimap_jump_to(y, my, mh);
                }
            }
        }
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

    /// Jump the editor viewport to the document line corresponding to the
    /// minimap pointer position `y` inside the minimap rail `(my..my+mh)`.
    pub(super) fn minimap_jump_to(&mut self, cursor_y: f32, rail_y: f32, rail_h: f32) {
        let frac = ((cursor_y - rail_y) / rail_h.max(1.0)).clamp(0.0, 1.0);
        let total = self.editor_buffer.total_lines();
        let visible = self
            .composition
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .and_then(|m| m.editor_viewport_line_count)
            .unwrap_or(10)
            .max(1);
        let top = zaroxi_core_editor_minimap::top_line_for_fraction(frac, visible, total);
        if let Some(ref mut comp) = self.composition
            && let Some(ref mut meta) = comp.metadata
        {
            meta.editor_scroll_top_line = top;
            meta.editor_scroll_px = top as f32 * lc::LINE_HEIGHT;
        }
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

    /// commit_open MUST NOT materialize stale `editor_body` bytes under the
    /// active tab's identity when the live rope already authoritatively owns
    /// that tab's content.  This is the isolated content-identity guard: path
    /// and bytes MUST come from the same source.
    ///
    /// The scenario models the exact post-close state where
    /// `close_editor_transactional` rebound the rope to the fallback's OWN
    /// content (resident / disk-loaded) and then `build_work_content()`
    /// produced a stale `editor_body` from the stale presenter snapshot (old
    /// file's content), which `request_open` fed into the open pipeline.  A
    /// language mismatch marks `buffer_changed`, which historically forced a
    /// re-materialization of that foreign `editor_body` over the correct rope.
    #[test]
    fn commit_open_does_not_stamp_new_path_on_stale_bytes() {
        const A_CONTENT: &str = "[package]\nname = \"zaroxi\"\nedition = \"2024\"";
        const B_CONTENT: &str = "/target\n*.log\n.env";

        let mut app = make_headless_app();

        // Two pinned tabs: Cargo.toml (A) and b.rs (B). B is active.
        app.editor_group.open_or_activate_pinned(
            "/w/Cargo.toml".into(),
            "buf:/w/Cargo.toml".into(),
            "Cargo.toml".into(),
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
        app.editor_group.activate_by_path("/w/b.rs");

        // State AFTER close_editor_transactional's rebind: the live rope
        // already authoritatively holds the fallback (B) content, keyed by B's
        // path.  A stale `current_language` from the previous file forces
        // `buffer_changed` on the next commit.
        app.committed_active_file = Some("buf:/w/b.rs".to_string());
        app.active_rope_owner_path = Some("/w/b.rs".to_string());
        app.editor_buffer = buf_with(B_CONTENT);
        app.current_language = zaroxi_core_platform_syntax::language::LanguageId::PlainText;

        // The stale work content `build_work_content()` would produce after the
        // close — `visible_window` was cleared, so the single line comes from
        // the (stale) presenter snapshot that still holds file A's content.
        // `active_file` correctly points at B.
        let stale_body = zaroxi_core_engine_ui::ContentView {
            title: "b.rs".to_string(),
            subtitle: "buf:/w/b.rs".to_string(),
            lines: A_CONTENT.lines().map(|s| s.to_string()).collect(),
            cursor_line: 0,
            cursor_col: 0,
            selection: None,
        };
        let wc = crate::gui::ShellWorkContent {
            editor_body: Some(stale_body),
            active_file: Some("buf:/w/b.rs".to_string()),
            ..Default::default()
        };
        app.request_open(wc);

        let a_fp = {
            let mut a = make_headless_app();
            a.editor_buffer = buf_with(A_CONTENT);
            a.editor_content_fingerprint()
        };
        let b_fp = {
            let mut a = make_headless_app();
            a.editor_buffer = buf_with(B_CONTENT);
            a.editor_content_fingerprint()
        };

        app.commit_open();

        // Identity invariant: the live rope must still hold B's bytes, NOT
        // file A's bytes stamped under B's path label.
        let head = app.editor_buffer.raw_head(512);
        assert!(
            head.starts_with("/target"),
            "commit_open must preserve the fallback's own bytes — rope head: {:?}",
            head,
        );
        assert_eq!(app.editor_content_fingerprint(), b_fp, "rope must equal B content");
        assert_ne!(
            app.editor_content_fingerprint(),
            a_fp,
            "rope fingerprint must not match the closed file's bytes",
        );
        assert_eq!(app.active_rope_owner_path.as_deref(), Some("/w/b.rs"));
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

    /// THE bad case, end to end: close the ACTIVE tab whose fallback is NOT
    /// resident in any in-memory store.  The rebind must source the fallback's
    /// content from its OWN disk file — never inherit the closed file's live
    /// rope, and never let `commit_open` stamp a stale single-line snippet
    /// under the fallback's identity.  Uses real temp files so the disk-read
    /// rebind path is exercised.
    #[test]
    fn closing_active_tab_with_nonresident_fallback_loads_own_content_from_disk() {
        const A_CONTENT: &str = "AAAA active file line one\nAAAA line two\nAAAA line three";
        const B_CONTENT: &str = "BBBB fallback own line one\nBBBB line two\nBBBB line three";

        // Real temp files on disk keyed by their canonical paths.
        let dir = std::env::temp_dir().join(format!(
            "zaroxi_close_identity_{}_{}",
            std::process::id(),
            zaroxi_kernel_types::Id::new(),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path_a = dir.join("a.rs");
        let path_b = dir.join("b.rs");
        std::fs::write(&path_a, A_CONTENT).unwrap();
        std::fs::write(&path_b, B_CONTENT).unwrap();
        let a = path_a.to_string_lossy().to_string();
        let b = path_b.to_string_lossy().to_string();

        let mut app = make_headless_app();
        app.editor_group.open_or_activate_pinned(
            a.clone(),
            format!("buf:{a}"),
            "a.rs".into(),
            BackendKind::Rope,
            true,
        );
        app.editor_group.open_or_activate_pinned(
            b.clone(),
            format!("buf:{b}"),
            "b.rs".into(),
            BackendKind::Rope,
            true,
        );
        // a.rs is the ACTIVE (live rope) document; b.rs is a pinned tab whose
        // content is NOT parked anywhere (non-resident fallback).
        app.editor_group.activate_by_path(&a);
        app.editor_buffer = buf_with(A_CONTENT);
        app.active_rope_owner_path = Some(a.clone());
        app.committed_active_file = Some(format!("buf:{a}"));

        // Stale visible_window belonging to A — the exact projection that used
        // to leak into the fallback's editor_body.
        let mut comp = DesktopComposition::new();
        comp.metadata = Some(DesktopMetadata {
            active_buffer: Some(BufferId(format!("buf:{a}"))),
            opened_buffer_count: 2,
            opened_buffers: vec![
                OpenedBufferItem {
                    buffer_id: BufferId(format!("buf:{a}")),
                    display: Some("a.rs".to_string()),
                    active: true,
                },
                OpenedBufferItem {
                    buffer_id: BufferId(format!("buf:{b}")),
                    display: Some("b.rs".to_string()),
                    active: false,
                },
            ],
            active_buffer_details: Some(crate::desktop::ActiveBufferDetails {
                buffer_id: BufferId(format!("buf:{a}")),
                display: Some("a.rs".to_string()),
                line_count: 3,
            }),
            visible_window: Some(visible_window_for(&[
                "AAAA active file line one",
                "AAAA line two",
            ])),
            ..Default::default()
        });
        app.composition = Some(comp);
        app.session_id = Some(ports::SessionId(zaroxi_kernel_types::Id::new()));
        app.workspace_service =
            Some(Arc::new(RecordingSvc { set_active_calls: Arc::new(Mutex::new(Vec::new())) }));

        let a_fp = {
            let mut x = make_headless_app();
            x.editor_buffer = buf_with(A_CONTENT);
            x.editor_content_fingerprint()
        };

        // Close the ACTIVE a.rs tab. Fallback is the non-resident b.rs.
        app.close_editor_transactional(&format!("buf:{a}"));

        // Path identity moved to the fallback.
        assert_eq!(app.editor_group.active_path(), Some(b.as_str()));
        assert_eq!(app.committed_active_file.as_deref(), Some(format!("buf:{b}").as_str()));

        // CONTENT identity: the live rope holds b.rs's OWN bytes read from
        // disk — not a.rs's bytes, and not a single-line snippet.
        assert!(
            app.editor_buffer.raw_head(64).starts_with("BBBB"),
            "rope must show b.rs's own content after close, got: {:?}",
            app.editor_buffer.raw_head(64),
        );
        assert_eq!(app.active_rope_owner_path.as_deref(), Some(b.as_str()));
        assert!(
            app.editor_buffer.line_count() >= 3,
            "fallback must render its full content, not collapse to one line (lines={})",
            app.editor_buffer.line_count(),
        );
        assert_ne!(
            app.editor_content_fingerprint(),
            a_fp,
            "rope must NOT still hold a.rs content under the b.rs tab",
        );

        // Driving the real commit must NOT reintroduce foreign content.
        app.commit_open();
        assert!(
            app.editor_buffer.raw_head(64).starts_with("BBBB"),
            "commit_open after close must preserve b.rs content, got: {:?}",
            app.editor_buffer.raw_head(64),
        );
        assert_ne!(app.editor_content_fingerprint(), a_fp);
        assert_eq!(app.active_rope_owner_path.as_deref(), Some(b.as_str()));

        std::fs::remove_dir_all(&dir).ok();
    }
}

/// Syntax-identity continuity across the close/fallback lifecycle.
///
/// These guard the regression where closing a tab correctly preserved the
/// fallback file's CONTENT (rope bytes) but left its SYNTAX pipeline bound to
/// the previous document — so the fallback rendered as plain text even though
/// its bytes were right.  Content ownership and syntax ownership are separate:
/// even when rope rematerialization is skipped, the syntax state must be valid
/// for the active file.
#[cfg(test)]
mod syntax_continuity_tests {
    use super::super::editor_group::BackendKind;
    use super::super::test_support::make_headless_app;
    use super::GuiApp;
    use crate::desktop::{DesktopComposition, DesktopMetadata, OpenedBufferItem};
    use crate::gui::window::editor_buf::EditorBufferState;
    use crate::ports;
    use crate::ports::BufferId;
    use std::path::Path;
    use zaroxi_core_platform_syntax::language::LanguageId;

    const RUST_A: &str = "fn alpha() -> u32 {\n    let value = 1;\n    value + 2\n}\n";
    const RUST_B: &str = "pub struct Beta {\n    field: i64,\n}\nimpl Beta { fn n(&self) {} }\n";

    fn buf_with(text: &str) -> EditorBufferState {
        let mut b = EditorBufferState::empty();
        b.replace_content(text);
        b
    }

    fn opened(path: &str, active: bool) -> OpenedBufferItem {
        OpenedBufferItem {
            buffer_id: BufferId(format!("buf:{path}")),
            display: path.rsplit('/').next().map(|s| s.to_string()),
            active,
        }
    }

    /// Mirror of the render-side span gate (`redraw.rs`): spans color the frame
    /// ONLY when they exist, describe the current buffer version, AND belong to
    /// the active file.  A `true` result means the editor renders with syntax;
    /// `false` means it falls through to plain text.
    fn syntax_would_render(app: &GuiApp) -> bool {
        app.latest_spans.is_some()
            && app.latest_spans_version == app.editor_buffer.buffer_version
            && app.latest_spans_owner.as_deref() == app.committed_active_file.as_deref()
    }

    fn spans_len(app: &GuiApp) -> usize {
        app.latest_spans.as_ref().map(|s| s.len()).unwrap_or(0)
    }

    /// Build a two-tab app where `active` is the live (rope) document and
    /// `fallback` is parked in `open_documents`, with a VALID syntax snapshot
    /// already computed for `active` (owner + version aligned).  The active
    /// file's language is detected from its path exactly as the open pipeline
    /// would (`.rs` → `Dynamic("rust")`), so the precondition matches runtime.
    fn app_with_active_and_parked_fallback(
        active_path: &str,
        active_text: &str,
        fallback_path: &str,
        fallback_text: &str,
    ) -> GuiApp {
        let mut app = make_headless_app();
        app.editor_group.open_or_activate_pinned(
            active_path.into(),
            format!("buf:{active_path}"),
            active_path.rsplit('/').next().unwrap().into(),
            BackendKind::Rope,
            true,
        );
        app.editor_group.open_or_activate_pinned(
            fallback_path.into(),
            format!("buf:{fallback_path}"),
            fallback_path.rsplit('/').next().unwrap().into(),
            BackendKind::Rope,
            true,
        );
        app.editor_group.activate_by_path(active_path);

        app.editor_buffer = buf_with(active_text);
        app.active_rope_owner_path = Some(active_path.to_string());
        app.committed_active_file = Some(format!("buf:{active_path}"));
        app.current_language = LanguageId::from_path(Path::new(active_path));
        // A valid syntax snapshot for the ACTIVE file (proves the fallback later
        // OWNS its own fresh snapshot rather than inheriting this one).
        app.schedule_background_parse();
        assert!(syntax_would_render(&app), "precondition: active file must start syntax-verified");

        app.open_documents.insert(fallback_path.to_string(), buf_with(fallback_text));

        let mut comp = DesktopComposition::new();
        comp.metadata = Some(DesktopMetadata {
            active_buffer: Some(BufferId(format!("buf:{active_path}"))),
            opened_buffer_count: 2,
            opened_buffers: vec![opened(active_path, true), opened(fallback_path, false)],
            ..Default::default()
        });
        app.composition = Some(comp);
        app.session_id = Some(ports::SessionId(zaroxi_kernel_types::Id::new()));
        app
    }

    /// Closing the ACTIVE tab must leave the fallback file syntax-verified —
    /// its own spans, its own owner, its own version, its own language — NOT
    /// plain text and NOT the closed file's spans.
    #[test]
    fn closing_active_tab_preserves_syntax_for_fallback_file() {
        let mut app = app_with_active_and_parked_fallback("/w/a.rs", RUST_A, "/w/b.rs", RUST_B);
        let closed_spans_owner = app.latest_spans_owner.clone();
        let fallback_lang = LanguageId::from_path(Path::new("/w/b.rs"));
        assert_ne!(
            fallback_lang,
            LanguageId::PlainText,
            "fixture: b.rs must be a recognized language"
        );

        app.close_editor_transactional("buf:/w/a.rs");
        app.commit_open();

        // Content identity moved to the fallback.
        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/b.rs"));
        assert!(app.editor_buffer.raw_head(32).starts_with("pub struct Beta"));

        // Syntax identity moved too: language, owner, version all realigned.
        assert_eq!(app.current_language, fallback_lang, "syntax language must follow fallback");
        assert_ne!(app.current_language, LanguageId::PlainText);
        assert!(
            syntax_would_render(&app),
            "fallback must render WITH syntax (owner={:?} committed={:?} spans_v={} buf_v={})",
            app.latest_spans_owner,
            app.committed_active_file,
            app.latest_spans_version,
            app.editor_buffer.buffer_version,
        );
        assert!(
            spans_len(&app) > 0,
            "fallback must have non-empty highlight spans, not plain text"
        );
        assert_eq!(app.latest_spans_owner.as_deref(), Some("buf:/w/b.rs"));
        assert_ne!(
            app.latest_spans_owner, closed_spans_owner,
            "spans owner must NOT still be the closed file",
        );
    }

    /// Closing a DIFFERENT-language active tab must rebind syntax to the
    /// fallback's language (exercises the `buffer_changed` cache-reset path).
    #[test]
    fn closing_active_tab_rebinds_language_for_fallback_file() {
        // Active is plain text, fallback is Rust: the language must transition to
        // the fallback's detected language and the fallback ends syntax-verified.
        let mut app = app_with_active_and_parked_fallback(
            "/w/data.txt",
            "just plain text\nno syntax here\n",
            "/w/keep.rs",
            RUST_B,
        );

        app.close_editor_transactional("buf:/w/data.txt");
        app.commit_open();

        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/keep.rs"));
        assert_eq!(app.current_language, LanguageId::from_path(Path::new("/w/keep.rs")));
        assert_ne!(app.current_language, LanguageId::PlainText);
        assert!(syntax_would_render(&app));
        assert!(spans_len(&app) > 0, "Rust fallback must have non-empty spans");
        assert_eq!(app.latest_spans_owner.as_deref(), Some("buf:/w/keep.rs"));
    }

    /// Closing an INACTIVE tab must NOT disturb the active file's syntax state.
    #[test]
    fn closing_inactive_tab_does_not_clear_active_syntax() {
        let mut app = app_with_active_and_parked_fallback("/w/a.rs", RUST_A, "/w/b.rs", RUST_B);
        let before_owner = app.latest_spans_owner.clone();
        let before_version = app.latest_spans_version;
        let before_len = spans_len(&app);
        let active_lang = app.current_language;

        // Close the INACTIVE fallback tab; active a.rs stays live.
        app.close_editor_transactional("buf:/w/b.rs");
        app.commit_open();

        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/a.rs"));
        assert_eq!(app.current_language, active_lang);
        assert_ne!(app.current_language, LanguageId::PlainText);
        assert!(syntax_would_render(&app), "active syntax must remain verified");
        assert!(spans_len(&app) > 0);
        assert_eq!(app.latest_spans_owner, before_owner, "active spans owner must be unchanged");
        assert_eq!(app.latest_spans_version, before_version, "active spans version unchanged");
        assert_eq!(spans_len(&app), before_len, "active spans set unchanged");
        assert!(app.editor_buffer.raw_head(16).starts_with("fn alpha"));
    }

    /// The core of this regression: when the close-time rebind leaves the rope
    /// ALREADY owning the fallback's bytes (so `commit_open` skips
    /// rematerialization — the previous content-identity optimization), syntax
    /// must STILL be recomputed for the fallback.  We prove rematerialization
    /// was skipped by checking the rope bytes were never rebuilt, yet the
    /// syntax snapshot is freshly owned by the fallback.
    #[test]
    fn fallback_activation_recomputes_syntax_when_rope_materialization_is_skipped() {
        // Same language on both sides so `buffer_changed` stays FALSE — the only
        // thing that can refresh syntax is the new syntax-continuity guard.
        let mut app = app_with_active_and_parked_fallback("/w/a.rs", RUST_A, "/w/b.rs", RUST_B);

        app.close_editor_transactional("buf:/w/a.rs");

        // After close (pre-commit): rope already authoritatively owns fallback B.
        assert_eq!(app.active_rope_owner_path.as_deref(), Some("/w/b.rs"));
        assert!(app.editor_buffer.raw_head(32).starts_with("pub struct Beta"));
        let rope_fp_after_rebind = app.editor_content_fingerprint();
        let buf_version_after_rebind = app.editor_buffer.buffer_version;

        app.commit_open();

        // Rematerialization was skipped: the rope object was NOT rebuilt (same
        // bytes, same version as the close-time rebind produced).
        assert_eq!(
            app.editor_content_fingerprint(),
            rope_fp_after_rebind,
            "rope bytes must be untouched by commit_open (materialization skipped)",
        );
        assert_eq!(
            app.editor_buffer.buffer_version, buf_version_after_rebind,
            "buffer_version must not bump — no rematerialization happened",
        );

        // ...yet syntax was still recomputed and re-owned for the fallback.
        assert!(
            syntax_would_render(&app),
            "syntax must be recomputed even when rope materialization is skipped",
        );
        assert!(spans_len(&app) > 0);
        assert_eq!(app.latest_spans_owner.as_deref(), Some("buf:/w/b.rs"));
        assert_eq!(app.latest_spans_version, app.editor_buffer.buffer_version);
    }

    /// Plain text must be used ONLY when language detection truly fails — never
    /// as an accidental global fallback after a close.  A recognized-language
    /// fallback stays colored; an unknown-extension fallback is plain (empty
    /// spans) but still correctly owned/versioned (so it is a deliberate
    /// plain-text state, not a stale/rejected-spans state).
    #[test]
    fn plain_text_is_only_used_when_language_detection_truly_fails() {
        // Fallback has NO recognized language (unknown extension).
        let mut app = app_with_active_and_parked_fallback(
            "/w/a.rs",
            RUST_A,
            "/w/notes.unknownext",
            "arbitrary\nunstructured\ncontent\n",
        );

        app.close_editor_transactional("buf:/w/a.rs");
        app.commit_open();

        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/notes.unknownext"));
        // Detection truly failed → PlainText, and NO colored spans.
        assert_eq!(app.current_language, LanguageId::PlainText);
        assert_eq!(spans_len(&app), 0, "unknown language must produce no colored spans");
        // But the (empty) syntax snapshot is still correctly OWNED + versioned:
        // this is a deliberate plain-text state, not a stale-spans rejection.
        assert!(app.latest_spans.is_some(), "an (empty) snapshot must be recorded for the owner");
        assert_eq!(app.latest_spans_owner.as_deref(), Some("buf:/w/notes.unknownext"));
        assert_eq!(app.latest_spans_version, app.editor_buffer.buffer_version);

        // Control: a recognized-language fallback in the same harness IS colored,
        // proving plain-text is not a global downgrade.
        let mut app2 = app_with_active_and_parked_fallback("/w/a.rs", RUST_A, "/w/keep.rs", RUST_B);
        app2.close_editor_transactional("buf:/w/a.rs");
        app2.commit_open();
        assert_ne!(app2.current_language, LanguageId::PlainText);
        assert!(spans_len(&app2) > 0, "recognized language must stay colored");
    }
}

/// View-state continuity across the close/fallback lifecycle.
///
/// Layered on top of the content-identity and syntax-identity fixes: the bytes
/// and highlighting are correct, but closing a tab must ALSO make the fallback
/// resume its exact previous session view state (scroll + caret + viewport)
/// from RESIDENT memory rather than reopening at top-of-file.  Resident state
/// beats disk every time.
#[cfg(test)]
mod view_state_continuity_tests {
    use super::super::editor_group::BackendKind;
    use super::super::test_support::make_headless_app;
    use super::{DocumentViewState, GuiApp};
    use crate::desktop::{DesktopComposition, DesktopMetadata, OpenedBufferItem};
    use crate::gui::window::editor_buf::EditorBufferState;
    use crate::ports;
    use crate::ports::BufferId;

    fn lines_text(n: usize, tag: &str) -> String {
        (0..n).map(|i| format!("{tag} line {i}")).collect::<Vec<_>>().join("\n")
    }

    fn buf_with_caret(text: &str, caret_line: usize, caret_col: usize) -> EditorBufferState {
        let mut b = EditorBufferState::empty();
        b.replace_content(text);
        b.set_caret_line_col(caret_line, caret_col);
        b
    }

    fn opened(path: &str, active: bool) -> OpenedBufferItem {
        OpenedBufferItem {
            buffer_id: BufferId(format!("buf:{path}")),
            display: path.rsplit('/').next().map(|s| s.to_string()),
            active,
        }
    }

    fn meta_scroll(app: &GuiApp) -> usize {
        app.composition
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .map(|m| m.editor_scroll_top_line)
            .unwrap_or(usize::MAX)
    }

    /// Build A (active, live) + B (parked normal doc) with B holding a saved
    /// NON-ZERO scroll/caret in `document_view_states` and a parked buffer whose
    /// caret matches.  `a_scroll` is the active file's live scroll.
    fn app_active_a_parked_b(
        a_scroll: usize,
        b_scroll: usize,
        b_caret_line: usize,
        b_caret_col: usize,
    ) -> GuiApp {
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
        app.editor_group.activate_by_path("/w/a.rs");

        // Active A is live in the rope with its own caret.
        app.editor_buffer = buf_with_caret(&lines_text(40, "AAAA"), 2, 1);
        app.active_rope_owner_path = Some("/w/a.rs".to_string());
        app.committed_active_file = Some("buf:/w/a.rs".to_string());

        // B parked in memory with a distinct caret + saved view state.
        app.open_documents.insert(
            "/w/b.rs".to_string(),
            buf_with_caret(&lines_text(40, "BBBB"), b_caret_line, b_caret_col),
        );
        app.document_view_states.insert(
            "/w/b.rs".to_string(),
            DocumentViewState {
                caret_line: b_caret_line,
                caret_col: b_caret_col,
                scroll_top: b_scroll,
            },
        );

        let mut comp = DesktopComposition::new();
        comp.metadata = Some(DesktopMetadata {
            active_buffer: Some(BufferId("buf:/w/a.rs".to_string())),
            opened_buffer_count: 2,
            opened_buffers: vec![opened("/w/a.rs", true), opened("/w/b.rs", false)],
            active_buffer_details: Some(crate::desktop::ActiveBufferDetails {
                buffer_id: BufferId("buf:/w/a.rs".to_string()),
                display: Some("a.rs".to_string()),
                line_count: 40,
            }),
            editor_scroll_top_line: a_scroll,
            ..Default::default()
        });
        app.composition = Some(comp);
        app.session_id = Some(ports::SessionId(zaroxi_kernel_types::Id::new()));
        app
    }

    /// Closing the ACTIVE tab must restore the fallback's saved scroll AND
    /// caret, not reset to top-of-file defaults.
    #[test]
    fn closing_active_tab_restores_fallback_scroll_and_caret() {
        let mut app = app_active_a_parked_b(3, 12, 15, 4);

        app.close_editor_transactional("buf:/w/a.rs");
        app.commit_open();

        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/b.rs"));
        assert_eq!(app.active_rope_owner_path.as_deref(), Some("/w/b.rs"));
        // View state resumed from resident memory (NOT top-of-file).
        assert_eq!(meta_scroll(&app), 12, "fallback scroll must resume at saved position");
        assert_eq!(app.editor_buffer.caret_line(), 15, "fallback caret line must resume");
        assert_eq!(app.editor_buffer.caret_col(), 4, "fallback caret col must resume");
        assert!(
            app.editor_buffer.raw_head(8).starts_with("BBBB"),
            "content must be the fallback's"
        );
    }

    /// Closing the active tab must reuse RESIDENT state and never reload from
    /// disk.  The paths intentionally do not exist on disk, so any disk fallback
    /// would blank the buffer — proving reuse by asserting non-blank content.
    #[test]
    fn closing_active_tab_reuses_resident_state_without_disk_reload() {
        let mut app = app_active_a_parked_b(0, 8, 9, 2);
        // Sanity: the fallback file is NOT on disk.
        assert!(!std::path::Path::new("/w/b.rs").exists());

        app.close_editor_transactional("buf:/w/a.rs");
        // The restore path fires during the close-time rebind (commit_open later
        // consumes/clears the flag, so capture it here).
        let restore_fired = app.restored_view_state_this_activation;
        app.commit_open();

        // Resident content + view state were reused (no disk, no blank frame).
        assert!(app.editor_buffer.char_count() > 0, "resident content must be reused, not blanked");
        assert!(app.editor_buffer.raw_head(8).starts_with("BBBB"));
        assert_eq!(meta_scroll(&app), 8);
        assert_eq!(app.editor_buffer.caret_line(), 9);
        assert!(restore_fired, "restore path must have fired during close");
        // The saved view-state entry was consumed (not leaked).
        assert!(!app.document_view_states.contains_key("/w/b.rs"));
    }

    /// Closing an INACTIVE tab must leave the ACTIVE tab's view state untouched.
    #[test]
    fn closing_inactive_tab_preserves_active_view_state() {
        let mut app = app_active_a_parked_b(7, 12, 15, 4);
        let a_caret_line = app.editor_buffer.caret_line();
        let a_caret_col = app.editor_buffer.caret_col();
        let a_fp = app.editor_content_fingerprint();

        // Close the INACTIVE b.rs; active a.rs stays live.
        app.close_editor_transactional("buf:/w/b.rs");
        app.commit_open();

        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/a.rs"));
        assert_eq!(app.active_rope_owner_path.as_deref(), Some("/w/a.rs"));
        assert_eq!(meta_scroll(&app), 7, "active scroll must be untouched by inactive close");
        assert_eq!(app.editor_buffer.caret_line(), a_caret_line, "active caret untouched");
        assert_eq!(app.editor_buffer.caret_col(), a_caret_col);
        assert_eq!(app.editor_content_fingerprint(), a_fp, "active content untouched");
    }

    /// Specifically covers the content-materialization optimization introduced by
    /// the earlier fixes: the rope is NOT rebuilt (bytes/version stable), yet the
    /// fallback's scroll and caret are still restored from resident view state.
    #[test]
    fn fallback_activation_preserves_view_state_when_content_materialization_is_skipped() {
        let mut app = app_active_a_parked_b(0, 20, 22, 6);

        app.close_editor_transactional("buf:/w/b.rs" /* inactive */);
        // (Sanity: closing inactive should not have moved active.)
        assert_eq!(app.committed_active_file.as_deref(), Some("buf:/w/a.rs"));

        // Now the real case: rebuild the fixture and close the ACTIVE tab.
        let mut app = app_active_a_parked_b(0, 20, 22, 6);
        app.close_editor_transactional("buf:/w/a.rs");

        // After the close rebind, the rope already owns the fallback bytes.
        let fp_after_rebind = app.editor_content_fingerprint();
        let version_after_rebind = app.editor_buffer.buffer_version;
        assert_eq!(app.active_rope_owner_path.as_deref(), Some("/w/b.rs"));
        assert_eq!(meta_scroll(&app), 20, "scroll restored at rebind time");
        assert_eq!(app.editor_buffer.caret_line(), 22);

        app.commit_open();

        // commit_open must NOT rematerialize (bytes + version unchanged)...
        assert_eq!(app.editor_content_fingerprint(), fp_after_rebind, "no rematerialization");
        assert_eq!(app.editor_buffer.buffer_version, version_after_rebind, "no version bump");
        // ...and the restored view state must survive commit_open.
        assert_eq!(meta_scroll(&app), 20, "scroll must survive commit_open");
        assert_eq!(app.editor_buffer.caret_line(), 22, "caret must survive commit_open");
    }

    /// Large-file fallback must also restore scroll (viewport window must cover
    /// the saved scroll position) and caret from resident view state.
    #[test]
    fn large_file_fallback_restores_scroll_state() {
        // Create a real temp file large enough (>= LARGE_THRESHOLD, 1 MiB) that
        // `commit_open`'s file-size recheck keeps `large_file_mode` on and
        // `DocumentBuffer::open` selects the PieceTable backend — so this
        // exercises the true large-file fallback path.
        let dir = std::env::temp_dir().join(format!(
            "zaroxi_vs_large_{}_{}",
            std::process::id(),
            zaroxi_kernel_types::Id::new(),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path_b = dir.join("big.rs");
        // ~1.6 MiB: 20000 lines of an 80-char body.
        let big = (0..20_000)
            .map(|i| format!("LARGE line {i:06} {}", "x".repeat(60)))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            big.len() as u64 >= zaroxi_core_editor_largefile::DocumentBuffer::LARGE_THRESHOLD,
            "fixture must exceed the large-file threshold",
        );
        std::fs::write(&path_b, &big).unwrap();
        let b = path_b.to_string_lossy().to_string();

        let mut app = make_headless_app();
        app.editor_group.open_or_activate_pinned(
            "/w/a.rs".into(),
            "buf:/w/a.rs".into(),
            "a.rs".into(),
            BackendKind::Rope,
            true,
        );
        app.editor_group.open_or_activate_pinned(
            b.clone(),
            format!("buf:{b}"),
            "big.rs".into(),
            BackendKind::PieceTable,
            true,
        );
        app.editor_group.activate_by_path("/w/a.rs");

        app.editor_buffer = buf_with_caret(&lines_text(40, "AAAA"), 1, 0);
        app.active_rope_owner_path = Some("/w/a.rs".to_string());
        app.committed_active_file = Some("buf:/w/a.rs".to_string());

        // Fallback B is a large (doc_buffers) document with saved view state.
        let doc = zaroxi_core_editor_largefile::DocumentBuffer::open(&path_b).unwrap();
        let total = doc.total_lines();
        app.doc_buffers.insert(b.clone(), doc);
        app.document_view_states
            .insert(b.clone(), DocumentViewState { caret_line: 30, caret_col: 2, scroll_top: 25 });

        let mut comp = DesktopComposition::new();
        comp.metadata = Some(DesktopMetadata {
            active_buffer: Some(BufferId("buf:/w/a.rs".to_string())),
            opened_buffer_count: 2,
            opened_buffers: vec![opened("/w/a.rs", true), opened(&b, false)],
            active_buffer_details: Some(crate::desktop::ActiveBufferDetails {
                buffer_id: BufferId("buf:/w/a.rs".to_string()),
                display: Some("a.rs".to_string()),
                line_count: 40,
            }),
            ..Default::default()
        });
        app.composition = Some(comp);
        app.session_id = Some(ports::SessionId(zaroxi_kernel_types::Id::new()));

        app.close_editor_transactional("buf:/w/a.rs");
        app.commit_open();

        assert_eq!(app.committed_active_file.as_deref(), Some(format!("buf:{b}").as_str()));
        assert!(app.large_file_mode, "fallback must be in large-file mode");
        assert_eq!(app.active_rope_owner_path.as_deref(), Some(b.as_str()));
        // Scroll + caret resumed from resident view state.
        assert_eq!(meta_scroll(&app), 25, "large-file fallback scroll must resume");
        assert_eq!(app.editor_buffer.caret_line(), 30, "large-file fallback caret must resume");
        // The rope window must COVER the saved scroll position (not just line 0).
        assert!(
            app.editor_buffer.line_count() >= 25,
            "rope window must cover the saved scroll (lines={})",
            app.editor_buffer.line_count(),
        );
        // active_buffer_details.line_count must reflect the true document total,
        // not the loaded rope window, so scroll clamping stays correct.
        let abd_lines = app
            .composition
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .and_then(|m| m.active_buffer_details.as_ref())
            .map(|d| d.line_count)
            .unwrap_or(0);
        assert_eq!(abd_lines, total, "large-file line_count must be the document total");

        std::fs::remove_dir_all(&dir).ok();
    }
}
