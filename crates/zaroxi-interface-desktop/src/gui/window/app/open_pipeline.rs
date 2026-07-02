/*!
Document open pipeline for [`GuiApp`]: request -> commit -> (background
read / rope build) -> parse -> present/settle. Keeps the open flow coherent
across `background_read`, `background_open`, and `background_parse`, plus the
atomic first-paint [`OpenPresentation`] bookkeeping.
*/

use super::*;

/// Explicit open intent driven by the explorer or tab strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenIntent {
    /// Single-click in explorer — reuse the single shared preview tab.
    Preview,
    /// Double-click in explorer — create a persistent pinned tab.
    Pinned,
    /// Click on an already-open pinned tab — activate existing, restore view state.
    ActivateExisting,
}

/// Phase 11 — atomic first-paint open presentation.
///
/// Tracks one open's path from the explorer click to the single, coherent first
/// paint of the new file. The old file (or loading shell) stays visible until the
/// new file's first visible screenful is shaped, at which point editor content
/// **and** chrome swap together in one frame (`presented`). There is exactly one
/// first-paint settle per open: the head preview no longer performs a separate
/// visible swap (it would race the chrome and re-settle the top viewport), so the
/// Full activation is the only thing the user ever sees swap in.
#[derive(Clone)]
pub struct OpenPresentation {
    /// Read token this presentation belongs to (newest wins; stale snapshots are
    /// dropped). For non-read opens (workspace open / tab switch) this mirrors
    /// the open token instead.
    pub token: u64,
    /// Target file path/label, for chrome-coherence checks and tracing.
    pub path: Option<String>,
    /// When the open was first requested (click / schedule). Drives
    /// `time_to_present_ms`.
    pub started_at: Instant,
    /// When the first-screenful snapshot finished shaping (atomic frame done).
    pub snapshot_ready_at: Option<Instant>,
    /// Whether the atomic first paint has been presented yet.
    pub presented: bool,
    /// Whether a head preview was produced for this open (telemetry only — it no
    /// longer drives a separate visible swap).
    pub used_head_preview: bool,
    /// Whether a produced head preview was folded into the single first paint
    /// rather than presented separately (always true when `used_head_preview`).
    pub promoted_head_preview: bool,
    /// Visible logical rows the first paint covered.
    pub first_viewport_lines: usize,
    /// Count of top-of-viewport re-shapes observed AFTER the atomic present — a
    /// success-criterion guard: this must stay 0 absent a real resize/edit.
    pub top_repaints_after_present: u32,
}

impl OpenPresentation {
    pub(crate) fn begin(token: u64, path: Option<String>) -> Self {
        Self {
            token,
            path,
            started_at: Instant::now(),
            snapshot_ready_at: None,
            presented: false,
            used_head_preview: false,
            promoted_head_preview: false,
            first_viewport_lines: 0,
            top_repaints_after_present: 0,
        }
    }
}

impl GuiApp {
    /// Reconcile `committed_active_file` from [`EditorGroup::active`].
    ///
    /// Call after any EditorGroup mutation (open_preview, open_pinned,
    /// promote, close, activate).  Ensures the render source key and
    /// status/tab-strip always reflect the editor-group's active
    /// editor identity.  Never sets a file path without also updating
    /// EditorGroup first — this function IS the reconciliation, not a
    /// competing authority.
    pub(crate) fn reconcile_active_file_from_editor_group(&mut self) {
        let eg_active = self.editor_group.active_path().map(|s| s.to_string());
        if eg_active.is_none() {
            // No file editor active; leave committed_active_file as-is
            // (it may be None for welcome / non-file states).
            return;
        }
        let desired = format!("buf:{}", eg_active.as_ref().unwrap());
        if self.committed_active_file.as_deref() != Some(&desired) {
            if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_VISIBLE_TAB_MODEL: reconcile active_file {} -> {}",
                    self.committed_active_file.as_deref().unwrap_or("<none>"),
                    desired,
                );
            }
            self.committed_active_file = Some(desired);
        }
    }

    /// Post-commit invariant: the visible active tab, `EditorGroup.active`,
    /// `committed_active_file`, and `metadata.active_buffer` must all resolve
    /// to the SAME canonical document.  `EditorGroup.active` is the sole
    /// authority; any disagreement is a structured invariant violation and
    /// is self-healed by reconciling every other surface to EditorGroup.
    ///
    /// Returns `true` when everything already agreed (no heal needed).
    pub(crate) fn assert_active_consistency(&mut self) -> bool {
        use super::editor_group::{
            buffer_key_from_path, canonical_path_from_editor_id, same_document,
        };

        let eg_active: Option<String> = self.editor_group.active_path().map(|p| p.to_string());
        let visible_active: Option<String> = self
            .editor_group
            .visible_tabs()
            .into_iter()
            .find(|t| t.is_active)
            .map(|t| canonical_path_from_editor_id(&t.path).to_string());
        let committed: Option<String> = self
            .committed_active_file
            .as_deref()
            .map(|s| canonical_path_from_editor_id(s).to_string());
        let active_buffer: Option<String> = self
            .composition
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .and_then(|m| m.active_buffer.as_ref())
            .map(|b| canonical_path_from_editor_id(&b.to_string()).to_string());

        let agree = |a: &Option<String>, b: &Option<String>| match (a, b) {
            (Some(x), Some(y)) => same_document(x, y),
            (None, None) => true,
            _ => false,
        };

        // EditorGroup is authority. Everything is measured against it.
        let consistent = agree(&eg_active, &visible_active)
            && agree(&eg_active, &committed)
            && agree(&eg_active, &active_buffer);

        let debug_tabs = std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1");
        if debug_tabs {
            eprintln!(
                "ZAROXI_VISIBLE_TAB_MODEL: active_consistency visible={} committed={} active_buffer={} editor_group={} consistent={}",
                visible_active.as_deref().unwrap_or("<none>"),
                committed.as_deref().unwrap_or("<none>"),
                active_buffer.as_deref().unwrap_or("<none>"),
                eg_active.as_deref().unwrap_or("<none>"),
                consistent,
            );
        }

        if consistent {
            return true;
        }

        // ── Self-heal: reconcile every surface to EditorGroup.active ──
        if let Some(ref canon) = eg_active {
            let desired_committed = buffer_key_from_path(canon);
            if self.committed_active_file.as_deref() != Some(&desired_committed) {
                self.committed_active_file = Some(desired_committed.clone());
            }
            let desired_bid = crate::ports::BufferId(buffer_key_from_path(canon));
            if let Some(ref mut comp) = self.composition {
                if let Some(ref mut meta) = comp.metadata {
                    for it in meta.opened_buffers.iter_mut() {
                        it.active = same_document(&it.buffer_id.to_string(), canon);
                    }
                    meta.active_buffer = Some(desired_bid);
                }
            }
            if debug_tabs {
                eprintln!("ZAROXI_VISIBLE_TAB_MODEL: active_consistency_healed -> {canon}");
            }
        }
        false
    }

    pub(crate) fn request_open(&mut self, wc: ShellWorkContent) {
        self.open_token += 1;
        let token = self.open_token;
        self.file_switch_count += 1;
        self.activation_seq += 1;
        let seq = self.activation_seq;
        // Supersede any not-yet-committed open: its heavy load never runs.
        if let Some((stale_token, _)) = self.pending_open.take()
            && file_open_trace_enabled()
        {
            eprintln!(
                "ZAROXI_FILE_OPEN_TRACE: token={} stage=cancelled cancelled=1 superseded_by={} commit_skipped_stale=1 t_ms=0.00",
                stale_token, token,
            );
        }
        if doc_lifecycle_trace_enabled() {
            eprintln!(
                "ZAROXI_DOC_LIFECYCLE: request_open seq={} active_file={:?} prev_active_file={:?}",
                seq, wc.active_file, self.committed_active_file,
            );
        }
        if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
            let path =
                wc.active_file.as_deref().and_then(|s| s.strip_prefix("buf:")).unwrap_or("<none>");
            eprintln!(
                "ZAROXI_VISIBLE_TAB_MODEL: request_open token={} path={} is_loading_chrome={} open_intent={:?} {}",
                token,
                path,
                wc.editor_body.is_none(),
                self.open_intent,
                self.editor_group.diagnostic_line(),
            );
        }
        // Loading state only when the active file actually changes (not for a
        // status-message refresh of the same file).
        self.visible_loading_state =
            wc.active_file.as_deref() != self.committed_active_file.as_deref();
        self.open_request_at = Some(std::time::Instant::now());
        if file_open_trace_enabled() {
            let path = wc.active_file.clone().unwrap_or_else(|| "<none>".to_string());
            eprintln!(
                "ZAROXI_FILE_OPEN_TRACE: token={} stage=start cancelled=0 superseded_by=- file_switch_count={} pending_open_requests=1 upstream_open_prep_ms={:.2} file={}",
                token, self.file_switch_count, self.last_upstream_open_prep_ms, path,
            );
        }
        // Stage A instant chrome ack: explorer selection / title / status
        // reflect the new file immediately. The editor keeps showing the
        // previous content (a brief "loading" state) until the commit
        // materializes the new buffer on the next frame.
        self.work_content = Some(wc.clone());
        self.pending_open = Some((token, wc));
        self.invalidate(InvalidationFlags::content());
    }

    /// Whether a file at the given canonical path (no `buf:` prefix) is
    /// already open in a persistent (pinned) buffer tab.
    fn is_path_in_opened_buffers(&self, canonical_path: &str) -> bool {
        self.composition
            .as_ref()
            .and_then(|c| c.metadata.as_ref())
            .map(|m| {
                m.opened_buffers.iter().any(|b| {
                    b.buffer_id.to_string().strip_prefix("buf:").unwrap_or("") == canonical_path
                })
            })
            .unwrap_or(false)
    }

    /// Stages B–E — commit the newest pending open. Runs once per frame from the
    /// redraw loop. Does the heavy work (buffer materialization, large-file
    /// decision, background syntax kickoff, open burst) for the *latest* token
    /// only; superseded requests were already dropped in `request_open`, so no
    /// stale buffer is ever materialized or committed.
    pub(crate) fn commit_open(&mut self) {
        let (token, wc) = match self.pending_open.take() {
            Some(p) => p,
            None => return,
        };
        // ── Guard: suppress stale recomputation ──
        // If another activation request arrived after this one was queued,
        // the superseding request already dropped this pending_open via
        // `request_open`.  The remaining case is a redundant activation
        // of the same file that arrived after the identical commit already
        // ran (e.g. tab-click on the already-active tab).  Skip it.
        if self.last_committed_activation_seq >= self.activation_seq {
            if doc_lifecycle_trace_enabled() {
                eprintln!(
                    "ZAROXI_DOC_LIFECYCLE: commit_open_skipped reason=stale_recomputation last_seq={} current_seq={}",
                    self.last_committed_activation_seq, self.activation_seq,
                );
            }
            return;
        }
        if doc_lifecycle_trace_enabled() {
            eprintln!(
                "ZAROXI_DOC_LIFECYCLE: commit_open_begin seq={} prev_active={:?} new_active={:?}",
                self.activation_seq, self.committed_active_file, wc.active_file,
            );
        }
        // Capture the OUTGOING document identity + backend BEFORE large_file_mode
        // is recomputed for the incoming file. Used by the per-document
        // checkout/checkin below to park the active document's edits + history.
        let prev_large_file_mode = self.large_file_mode;
        let prev_active_file = self.committed_active_file.clone();
        // Record for status-model latency probes.
        let now = std::time::Instant::now();
        self.last_open_started_at = Some(now);
        self.last_focus_change_at = Some(now);
        let ev_start = std::time::Instant::now();
        // ── Phase 1 language detection (single source of truth) ──
        // The active file path determines the language used by the background
        // parser.  There is no hardcoded language anywhere in the pipeline.
        let detected_language = wc
            .active_file
            .as_deref()
            .map(|p| LanguageId::from_path(Path::new(p)))
            .unwrap_or(LanguageId::PlainText);

        // When the file being shown changes (or its detected language changes),
        // drop spans from the previous buffer so stale highlights are never
        // reused, and discard any pending worker result for the old buffer.
        //
        // Also trigger a content-change when transitioning from loading state
        // (editor_body=None) to ready state (editor_body=Some).  Without this,
        // the loading→ready transition after a background read would see the
        // same `active_file` path and skip rope population, leaving the editor
        // permanently empty unless the user clicks the tab again.
        let body_loading_to_ready =
            self.work_content.as_ref().and_then(|old| old.editor_body.as_ref()).is_none()
                && wc.editor_body.is_some();
        let buffer_changed = self.committed_active_file.as_deref() != wc.active_file.as_deref()
            || detected_language != self.current_language
            || body_loading_to_ready;
        self.current_language = detected_language;

        // Recompute large-file mode from the ACTUAL file metadata on every
        // commit.  Must run unconditionally — NOT gated on buffer_changed —
        // because the explorer click path returns `comp.build_work_content()`
        // which may carry a stale active_file from the previous document.
        // Without this, a medium file can inherit large_file_mode from the
        // preceding large file and render the wrong content.
        self.large_file_mode = wc
            .active_file
            .as_deref()
            .map(|s| s.strip_prefix("buf:").unwrap_or(s))
            .and_then(|path_str| {
                std::fs::metadata(path_str).ok().map(|m| {
                    m.len() >= zaroxi_core_editor_largefile::DocumentBuffer::LARGE_THRESHOLD
                })
            })
            .unwrap_or(false);

        // ── Unified per-document view-state lifecycle ──
        //
        // Two independent stores, both keyed by canonical file path:
        //   document_view_states — view metadata (caret + scroll), shared by
        //       both backends.  Lightweight — no rope content.
        //   open_documents — full EditorBufferState clone for NORMAL files
        //       only (rope + caret + undo/redo).  Large files use the
        //       PieceTable in doc_buffers as their canonical source and
        //       repopulate the rope on demand.
        //
        // On every tab switch:
        //   Check IN from prev_key: save view state + content into stores.
        //   Check OUT to new_key:    restore view state + content from stores.
        //
        // The scroll position lives separately in DesktopMetadata and MUST
        // be restored explicitly after checkout, because refresh_with_service
        // always rebuilds metadata with editor_scroll_top_line = 0.
        let new_doc_key: Option<String> =
            wc.active_file.as_deref().map(|s| s.strip_prefix("buf:").unwrap_or(s).to_string());
        let active_file_changed = prev_active_file.as_deref() != wc.active_file.as_deref();
        // A TRUE content-owner switch: the canonical active path actually
        // changed (not a same-path preview→pin, not a status refresh). Bump
        // the content-generation epoch so every editor render artifact keyed
        // by it is invalidated — geometry/hash matches can never resurrect the
        // previous owner's payload. Same-path transitions leave it untouched
        // so scroll/view state is preserved.
        let canon = |s: Option<&str>| s.map(|v| v.strip_prefix("buf:").unwrap_or(v).to_string());
        let true_owner_switch = canon(prev_active_file.as_deref())
            != canon(wc.active_file.as_deref())
            && wc.active_file.is_some();
        if true_owner_switch {
            self.content_generation = self.content_generation.wrapping_add(1);
            if std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_TABS: editor_owner_switch old={} new={} reason=commit_open gen={}",
                    prev_active_file.as_deref().unwrap_or("<none>"),
                    wc.active_file.as_deref().unwrap_or("<none>"),
                    self.content_generation,
                );
            }
        }
        let mut restored_from_store = false;
        // ── Check IN: save outgoing document ──
        // If handle_actions already saved the outgoing view state (early
        // check-in triggered by reset_scroll_state), reuse that saved
        // entry instead of re-reading the now-zeroed metadata scroll.
        let prev_key: Option<String> =
            prev_active_file.as_deref().map(|s| s.strip_prefix("buf:").unwrap_or(s).to_string());
        // Detect whether handle_actions already saved the outgoing
        // view state (early check-in before reset_scroll_state).
        let early_checkin_scroll: Option<usize> = {
            let k = prev_key.as_deref().unwrap_or("");
            self.document_view_states.get(k).map(|vs| vs.scroll_top)
        };
        if active_file_changed && let Some(prev_key) = prev_key {
            let has_content = self.editor_buffer.char_count() > 0;
            if has_content
                || self.open_documents.contains_key(&prev_key)
                || self.document_view_states.contains_key(&prev_key)
            {
                // ── Unified view state: save for ALL backends ──
                // Prefer the scroll saved by the early check-in in
                // handle_actions; otherwise read from live metadata.
                let scroll_top = early_checkin_scroll.unwrap_or_else(|| {
                    self.composition
                        .as_ref()
                        .and_then(|c| c.metadata.as_ref())
                        .map(|m| m.editor_scroll_top_line)
                        .unwrap_or(0)
                });
                let early_saved = early_checkin_scroll.is_some();
                let vs = DocumentViewState::from_editor_and_scroll(&self.editor_buffer, scroll_top);
                if doc_lifecycle_trace_enabled() {
                    eprintln!(
                        "ZAROXI_DOC_LIFECYCLE: checkin path={} kind={} caret={} scroll={} early_checkin={}",
                        prev_key,
                        if prev_large_file_mode { "piece_table" } else { "rope" },
                        vs.caret_line,
                        vs.scroll_top,
                        early_saved,
                    );
                }
                if prev_large_file_mode {
                    // Large file: trim rope to free RAM.  PieceTable holds
                    // canonical content; repopulation happens on demand.
                    let keep = self.editor_buffer.line_count().min(1usize);
                    let rope_before_trim = self.editor_buffer.line_count();
                    if let Some(db) = self.doc_buffers.get(&prev_key)
                        && db.total_lines() > 0
                    {
                        let stub = db.lines_in_range(0, keep.saturating_sub(1));
                        let lines: Vec<String> = stub.into_iter().map(|(_, s)| s).collect();
                        let cl = vs.caret_line.min(lines.len().saturating_sub(1));
                        self.editor_buffer.populate_from_lines(&lines, cl, vs.caret_col);
                        if doc_lifecycle_trace_enabled() {
                            let pt_total = db.total_lines();
                            eprintln!(
                                "ZAROXI_DOC_LIFECYCLE: trim path={} reason=inactive lines_after={} pt_total={} rope_before_trim={} source=piece_table",
                                prev_key,
                                lines.len(),
                                pt_total,
                                rope_before_trim,
                            );
                        }
                    }
                } else {
                    // Normal file: full EditorBufferState clone.
                    if doc_lifecycle_trace_enabled() {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: checkin_content path={} dirty={} chars={} version={}",
                            prev_key,
                            self.editor_buffer.is_dirty(),
                            self.editor_buffer.char_count(),
                            self.editor_buffer.buffer_version,
                        );
                    }
                    self.open_documents.insert(prev_key.clone(), self.editor_buffer.clone());
                }
                self.document_view_states.insert(prev_key.clone(), vs);
            }
        }
        // ── Editor-group membership handling ─────────────────────────
        // Only consume the open intent when this commit carries actual
        // editor content (not the loading-chrome placeholder that is
        // immediately superseded by the background-read result).
        let is_loading_chrome = wc.editor_body.is_none();
        let intent = if is_loading_chrome { None } else { self.open_intent.take() };
        let incoming_path = new_doc_key.clone();
        if let Some(ref act_path) = incoming_path {
            let backend_kind = if self.large_file_mode {
                editor_group::BackendKind::PieceTable
            } else {
                editor_group::BackendKind::Rope
            };
            // Canonical transport/lookup key: always `buf:<canonical_path>`,
            // never the raw `wc.active_file` (which may or may not carry the
            // prefix).  This guarantees EditorEntry.buffer_id is 1:1 with the
            // opened_buffers projection and the close/tab id, so identity can
            // never drift between the `buf:path` and plain `path` forms.
            let buffer_id = editor_group::buffer_key_from_path(act_path);
            // Tab title: always use the file basename.  Never show
            // "untitled" or workspace-service display text for a real
            // file path.  The basename is derived from the act_path
            // (canonical, guaranteed to be a real file path).
            let display = act_path.rsplit('/').next().unwrap_or(act_path).to_string();
            let backend_ready = !is_loading_chrome;

            if doc_lifecycle_trace_enabled() {
                eprintln!(
                    "ZAROXI_DOC_LIFECYCLE: commit_open_intent path={} intent={:?} editor_group={} is_loading_chrome={}",
                    act_path,
                    intent,
                    self.editor_group.diagnostic_line(),
                    is_loading_chrome,
                );
            }
            if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
                let ob_tabs: Vec<String> = self
                    .composition
                    .as_ref()
                    .and_then(|c| c.metadata.as_ref())
                    .map(|m| {
                        m.opened_buffers
                            .iter()
                            .map(|b| {
                                b.buffer_id
                                    .to_string()
                                    .strip_prefix("buf:")
                                    .unwrap_or(&b.buffer_id.to_string())
                                    .to_string()
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                eprintln!(
                    "ZAROXI_VISIBLE_TAB_MODEL: commit_open_intent path={} intent={:?} {} opened_buffers={:?} is_loading_chrome={} backend_kind={:?}",
                    act_path,
                    intent,
                    self.editor_group.diagnostic_line(),
                    ob_tabs,
                    is_loading_chrome,
                    backend_kind,
                );
            }

            // ── Loading-chrome instant EditorGroup update ─────────────
            // When a file is opened and the first commit carries only the
            // loading placeholder (editor_body=None), update EditorGroup
            // IMMEDIATELY so the tab strip reflects the new file even
            // while the background read is in flight.  The intent is NOT
            // consumed here — it remains for the real-content commit
            // which will set backend_ready=true on the same editor entry.
            if is_loading_chrome {
                if let Some(peek_intent) = self.open_intent {
                    match peek_intent {
                        OpenIntent::Preview => {
                            let old_preview: Option<String> =
                                self.editor_group.preview_path().map(|s| s.to_string());
                            if let Some(ref prev_path) = old_preview {
                                if *prev_path != *act_path {
                                    let saved_scroll = self
                                        .composition
                                        .as_ref()
                                        .and_then(|c| c.metadata.as_ref())
                                        .map(|m| m.editor_scroll_top_line)
                                        .unwrap_or(0);
                                    editor_group::EditorGroup::save_view_state_for(
                                        &mut self.document_view_states,
                                        &prev_path,
                                        saved_scroll,
                                        &self.editor_buffer,
                                    );
                                }
                            }
                            let _ = self.editor_group.open_preview(
                                act_path.clone(),
                                buffer_id.clone(),
                                display.clone(),
                                backend_kind,
                                false, // loading=true → backend_ready=false
                            );
                            if doc_lifecycle_trace_enabled() {
                                eprintln!(
                                    "ZAROXI_DOC_LIFECYCLE: loading_preview_set path={} display={} backend_kind={:?}",
                                    act_path, display, backend_kind,
                                );
                            }
                        }
                        OpenIntent::Pinned => {
                            let _ = self.editor_group.open_or_activate_pinned(
                                act_path.clone(),
                                buffer_id.clone(),
                                display.clone(),
                                backend_kind,
                                false,
                            );
                        }
                        OpenIntent::ActivateExisting => {
                            self.editor_group.activate_by_path(act_path);
                        }
                    }
                }
                // Fall through: intent NOT consumed, real-content commit
                // below will set backend_ready=true.
            }

            match intent {
                Some(OpenIntent::Preview) => {
                    // Save outgoing preview state before replacing.
                    let old_preview: Option<String> =
                        self.editor_group.preview_path().map(|s| s.to_string());
                    if let Some(ref prev_path) = old_preview {
                        if *prev_path != *act_path {
                            let saved_scroll = self
                                .composition
                                .as_ref()
                                .and_then(|c| c.metadata.as_ref())
                                .map(|m| m.editor_scroll_top_line)
                                .unwrap_or(0);
                            editor_group::EditorGroup::save_view_state_for(
                                &mut self.document_view_states,
                                &prev_path,
                                saved_scroll,
                                &self.editor_buffer,
                            );
                            if doc_lifecycle_trace_enabled() {
                                eprintln!(
                                    "ZAROXI_DOC_LIFECYCLE: preview_save old={} scroll={}",
                                    prev_path, saved_scroll,
                                );
                            }
                        }
                    }
                    // Replace the loading entry (or create fresh) with
                    // backend_ready=true.
                    let result = self.editor_group.replace_preview(
                        act_path.clone(),
                        buffer_id,
                        display.clone(),
                        backend_kind,
                        backend_ready,
                    );
                    if result.is_some() {
                        if doc_lifecycle_trace_enabled() {
                            eprintln!(
                                "ZAROXI_DOC_LIFECYCLE: preview_set path={} display={} backend_kind={:?}",
                                act_path, display, backend_kind,
                            );
                        }
                    } else if doc_lifecycle_trace_enabled() {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: preview_skipped path={} reason=already_pinned",
                            act_path,
                        );
                    }
                }
                Some(OpenIntent::Pinned) => {
                    let was_preview = self.editor_group.is_preview(act_path);
                    let id = self.editor_group.open_or_activate_pinned(
                        act_path.clone(),
                        buffer_id,
                        display.clone(),
                        backend_kind,
                        backend_ready,
                    );
                    if doc_lifecycle_trace_enabled() {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: pinned_open path={} id={} was_preview={} backend_kind={:?}",
                            act_path, id, was_preview, backend_kind,
                        );
                    }
                    // For large files that were preview-only (not yet in
                    // opened_buffers), register now so the pinned tab
                    // survives tab switches.
                    {
                        let in_opened = self.is_path_in_opened_buffers(act_path);
                        if !in_opened {
                            if let Some(ref mut comp) = self.composition {
                                let disp = act_path.rsplit('/').next().map(|s| s.to_string());
                                let bid = crate::ports::BufferId(format!("buf:{}", act_path));
                                comp.add_opened_buffer_direct(bid.clone(), disp);
                                if doc_lifecycle_trace_enabled() {
                                    eprintln!(
                                        "ZAROXI_DOC_LIFECYCLE: pinned_register path={} was_preview={}",
                                        act_path, was_preview,
                                    );
                                }
                            }
                        }
                    }
                }
                Some(OpenIntent::ActivateExisting) | None => {
                    // Normal activation or tab click — just activate via
                    // editor_group.
                    self.editor_group.activate_by_path(act_path);
                }
            }
            // Ensure committed_active_file follows EditorGroup.active.
            self.reconcile_active_file_from_editor_group();
        }

        // ── Check OUT: restore incoming document ──
        // Only run when the active file actually changed.  Same-file
        // transitions (preview→pinned promotion, refresh reactivation,
        // tab clicks on already-active tabs) must NOT trigger a checkout
        // and must preserve the live scroll/caret/rope state.
        let mut vs_from_store: Option<DocumentViewState> = None;
        if active_file_changed {
            if let Some(key) = new_doc_key.as_deref() {
                vs_from_store = self.document_view_states.remove(key);
            }
        }
        if let Some(ref vs) = vs_from_store {
            if doc_lifecycle_trace_enabled() {
                eprintln!(
                    "ZAROXI_DOC_LIFECYCLE: checkout_view path={} kind={} caret={} scroll={}",
                    new_doc_key.as_deref().unwrap_or("?"),
                    if self.large_file_mode { "piece_table" } else { "rope" },
                    vs.caret_line,
                    vs.scroll_top,
                );
            }
        }

        // Backend-specific content restoration (only on actual file switch).
        if active_file_changed {
            if self.large_file_mode {
                // Large file: always rebuild rope from the canonical PieceTable.
                let caret_line = vs_from_store.as_ref().map_or(0, |v| v.caret_line);
                let caret_col = vs_from_store.as_ref().map_or(0, |v| v.caret_col);
                let scroll_top = vs_from_store.as_ref().map_or(0, |v| v.scroll_top);
                let found = if let Some(key) = new_doc_key.as_deref()
                    && let Some(db) = self.doc_buffers.get(key)
                {
                    let total = db.total_lines();
                    let end_needed = scroll_top.saturating_add(200).min(total).max(1);
                    let vp = db.lines_in_range(0, end_needed.saturating_sub(1));
                    let lines: Vec<String> = vp.into_iter().map(|(_, s)| s).collect();
                    if !lines.is_empty() {
                        self.editor_buffer.populate_from_lines(&lines, caret_line, caret_col);
                        // Rope now holds this large file's viewport window.
                        self.active_rope_owner_path = Some(key.to_string());
                    }
                    if doc_lifecycle_trace_enabled() {
                        let rope_after = self.editor_buffer.line_count();
                        let ws = self.editor_buffer.window_start_line;
                        let is_full_window = lines.len() >= total.min(200);
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: checkout_authority path={} source=piece_table_full total_lines={} loaded_lines={} rope_after={} window_start={} rope_full_window={} caret={} scroll={} cached_view={} viewport_slice_used={}",
                            key,
                            total,
                            lines.len(),
                            rope_after,
                            ws,
                            is_full_window,
                            caret_line,
                            scroll_top,
                            vs_from_store.is_some(),
                            !is_full_window,
                        );
                    }
                    true
                } else {
                    if doc_lifecycle_trace_enabled() {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: checkout_miss path={} reason=doc_buffers_missing backend=piece_table",
                            new_doc_key.as_deref().unwrap_or("?"),
                        );
                    }
                    // Content-ownership guard: the incoming large file has no
                    // backing content and the rope still holds the OUTGOING
                    // file's text.  Never present another file's content under
                    // this file's identity — clear the rope so the viewport is
                    // empty until the correct PieceTable hydrates.
                    self.editor_buffer.replace_content("");
                    // Rope holds no valid document until hydration lands.
                    self.active_rope_owner_path = None;
                    false
                };
                if found {
                    restored_from_store = vs_from_store.is_some();
                }
            } else {
                // Normal file: FULL editor-buffer restore from open_documents.
                // HARD GATE: never attempt open_documents lookup for large files.
                if let Some(key) = new_doc_key.as_deref()
                    && let Some(stored) = self.open_documents.remove(key)
                {
                    if doc_lifecycle_trace_enabled() {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: checkout_authority path={} source=open_documents dirty={} version={}",
                            key,
                            stored.is_dirty(),
                            stored.buffer_version,
                        );
                    }
                    self.editor_buffer = stored;
                    restored_from_store = true;
                    // Rope now holds this normal document's full content.
                    self.active_rope_owner_path = Some(key.to_string());
                } else if doc_lifecycle_trace_enabled() && new_doc_key.is_some() {
                    eprintln!(
                        "ZAROXI_DOC_LIFECYCLE: checkout_fresh path={} authority=none will_rebuild_from_disk",
                        new_doc_key.as_deref().unwrap_or("?"),
                    );
                }
            }
        }

        // ── Apply restored view state to composition metadata ──
        // Must run AFTER content restoration so the editor_buffer caret
        // is already correct.  Overrides the zero scroll set by the
        // refresh_with_service metadata rebuild.
        // Also sets the guard flag so later code can detect that this
        // commit already dealt with a reactivated document.
        if let Some(ref vs) = vs_from_store {
            if let Some(ref mut comp) = self.composition {
                if let Some(ref mut meta) = comp.metadata {
                    meta.editor_scroll_top_line = vs.scroll_top;
                    meta.editor_scroll_px = vs.scroll_top as f32 * lc::LINE_HEIGHT;
                    if doc_lifecycle_trace_enabled() {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: scroll_restored key={} scroll={}",
                            new_doc_key.as_deref().unwrap_or("?"),
                            vs.scroll_top,
                        );
                    }
                }
            }
            self.restored_view_state_this_activation = true;
        } else {
            // Not a reactivation: clear any stale flag from a previous cycle.
            self.restored_view_state_this_activation = false;
        }

        if buffer_changed {
            self.latest_spans = None;
            self.latest_spans_version = 0;
            self.latest_spans_owner = None;
            // The retained full-coverage baseline belongs to the previous file.
            self.last_good_highlight = None;
            if let Some(ref mut worker) = self.parse_worker {
                worker.clear_result();
            }
            // Trim retained editor caches on file switch to bound RSS.
            self.line_syntax_cache.clear();
            self.cached_line_hashes.clear();
            self.editor_retained_bytes = 0;
            // Invalidate the shaped editor-data cache so the next frame
            // rebuilds with the new file's content.  Without this,
            // the cache can return stale content from the previous file
            // when the content hash + spans version happen to match.
            self.cached_editor_data = None;
            self.cached_editor_lines_hash = 0;
            self.cached_editor_spans_version = 0;
            self.cached_editor_active_file = None;
            // Reset per-file cockpit state.
            self.cockpit_minimap_symbols.clear();

            self.cockpit_symbols_version = 0;
            self.cockpit_diff_hunks.clear();
            self.cockpit_diff_version = 0;
            self.cockpit_retained_bytes = 0;
            // Evict cold shape-cache entries so the new file's glyphs
            // don't compete with stale entries from the previous file.
            if let Some(ref core) = self.render_core {
                if let Some(tr) = core.text_renderer() {
                    tr.evict_shaped_cold(512);
                }
            }
        }

        let mut backgrounded = false;
        // ── Shared first-open materialization gate (Rope + PieceTable) ──
        // Large files hydrate the rope unconditionally from `doc_buffers` in the
        // block further below (the canonical PieceTable rebuild). Normal,
        // Rope-backed files have no such unconditional hydration, so they must
        // (re)materialize the rope HERE whenever real content arrives for the
        // active document.
        //
        // This must NOT be gated on `buffer_changed` alone. The explorer click
        // path commits an instant *loading* placeholder (`editor_body = None`)
        // which clears the rope to a single empty line AND sets
        // `committed_active_file` to the new file. When the off-thread read then
        // lands and `request_open` is called with the real body, the follow-up
        // `commit_open` sees a matching `committed_active_file` (and the dead
        // `body_loading_to_ready` guard never fires because `request_open`
        // already overwrote `work_content`), so `buffer_changed` is FALSE and the
        // empty placeholder rope would survive until a second tab click. The
        // `char_count() == 0` clause materializes the real content on that
        // loading→ready transition, mirroring the large-file unconditional
        // hydration so normal and large files share one first-open contract.
        let body_has_content = wc.editor_body.as_ref().map_or(false, |b| !b.lines.is_empty());
        let needs_rope_materialize = !restored_from_store
            && wc.editor_body.is_some()
            && (buffer_changed
                || (!self.large_file_mode
                    && body_has_content
                    && self.editor_buffer.char_count() == 0));
        if first_open_trace_enabled() {
            eprintln!(
                "ZAROXI_DEBUG_FIRST_OPEN: commit token={} kind={} buffer_changed={} body_present={} body_has_content={} rope_char_count={} needs_materialize={} active_file={:?}",
                token,
                if self.large_file_mode { "large" } else { "normal" },
                buffer_changed,
                wc.editor_body.is_some(),
                body_has_content,
                self.editor_buffer.char_count(),
                needs_rope_materialize,
                wc.active_file,
            );
        }
        if needs_rope_materialize && let Some(ref body) = wc.editor_body {
            // Loading→ready upgrade for the SAME active document (buffer_changed
            // is false): the per-file cache/spans reset in the `buffer_changed`
            // block above did NOT run, so stale highlight spans from the previous
            // content are still "latest" and would paint the freshly materialized
            // text with the wrong colors. Drop them here so the editor renders
            // plain text until a fresh parse of the new content lands.
            if !buffer_changed && !self.large_file_mode {
                self.latest_spans = None;
                self.latest_spans_version = 0;
                self.latest_spans_owner = None;
                // Materialized content replaces the previous body; its baseline
                // no longer applies.
                self.last_good_highlight = None;
                if let Some(ref mut worker) = self.parse_worker {
                    worker.clear_result();
                }
                self.line_syntax_cache.clear();
                self.cached_line_hashes.clear();
                self.cached_editor_data = None;
                self.cached_editor_lines_hash = 0;
                self.cached_editor_spans_version = 0;
            }
            let open_bytes: usize = body.lines.iter().map(|l| l.len()).sum();
            if self.large_file_mode {
                // Large file: populate rope from doc_buffers below
                // (unconditional block after this one).
                backgrounded = true;
                if self.large_file_mode
                    && std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1")
                {
                    eprintln!(
                        "ZAROXI_DEBUG_LARGE_FILE: large_file_mode ON lines={} bytes={} backgrounded=true rope_lines={}",
                        body.lines.len(),
                        body.lines.iter().map(|l| l.len()).sum::<usize>(),
                        self.editor_buffer.line_count(),
                    );
                }
            } else if Self::should_background_open(&body.lines) {
                // ── Heavy file: materialize the rope OFF the UI thread ──
                // The UI thread does only cheap bookkeeping here; the editor keeps
                // showing the previous content (loading) until the worker's rope
                // lands in `poll_open_results`. The open burst and committed token
                // are deferred to that commit-on-ready point.
                backgrounded = true;
                if self.open_worker.is_none() {
                    self.open_worker = Some(background_open::BackgroundOpenWorker::spawn());
                }
                if let Some(ref mut w) = self.open_worker {
                    w.schedule_open(background_open::OpenJob {
                        token,
                        lines: body.lines.clone(),
                        cursor_line: body.cursor_line,
                        cursor_col: body.cursor_col,
                    });
                }
                self.background_open_pending = true;
                self.open_worker_started_at = Some(std::time::Instant::now());
                if perf_trace_enabled() || pipeline_trace_enabled() {
                    eprintln!(
                        "ZAROXI_OPEN_TRACE: token={} lines={} bytes={} open_buffer_ms=0.00 load_mode=background",
                        token,
                        body.lines.len(),
                        open_bytes,
                    );
                }
                if file_open_trace_enabled() {
                    eprintln!(
                        "ZAROXI_OPEN_WORKER_TRACE: token={} started=1 finished=0 cancelled=0 chunks=0 ms=0.00 background_open_pending=1",
                        token,
                    );
                }
            } else {
                // ── Small / medium file: synchronous (sub-frame) build ──
                let open_t = std::time::Instant::now();
                self.editor_buffer.populate_from_lines(
                    &body.lines,
                    body.cursor_line,
                    body.cursor_col,
                );
                // Rope now holds this normal document's freshly materialized text.
                self.active_rope_owner_path = new_doc_key.clone();
                let open_buffer_ms = open_t.elapsed().as_secs_f32() * 1000.0;
                // Materializing real content is always a content change for the
                // editor (even on the loading→ready transition where
                // `buffer_changed` is false), so arm the first-paint / parse
                // burst unconditionally here.
                self.finalize_buffer_commit(true);
                if first_open_trace_enabled() {
                    eprintln!(
                        "ZAROXI_DEBUG_FIRST_OPEN: materialized_rope token={} lines={} rope_lines={} rope_chars={} buffer_changed={}",
                        token,
                        body.lines.len(),
                        self.editor_buffer.line_count(),
                        self.editor_buffer.char_count(),
                        buffer_changed,
                    );
                }
                if perf_trace_enabled() || pipeline_trace_enabled() {
                    // load_mode: 'degraded' large files render plain + viewport-only;
                    // 'full' files get background syntax.
                    let load_mode = if self.large_file_mode { "degraded" } else { "full" };
                    eprintln!(
                        "ZAROXI_OPEN_TRACE: token={} lines={} bytes={} open_buffer_ms={:.2} load_mode={}",
                        token,
                        body.lines.len(),
                        open_bytes,
                        open_buffer_ms,
                        load_mode,
                    );
                }
            }
        }
        // ── Restored-document finalization ──
        // We checked out an already-open normal document from the in-memory
        // store, so no rebuild from disk content ran. Re-arm the first paint and
        // a fresh syntax parse for the restored text. `finalize_buffer_commit`
        // no longer touches the dirty baseline, so the parked dirty state +
        // undo/redo history are preserved exactly as they were.
        // ── Restored-document finalization ──
        // We checked out an already-open normal document from the in-memory
        // store, so no rebuild from disk content ran. Re-arm the first paint and
        // a fresh syntax parse for the restored text. `finalize_buffer_commit`
        // no longer touches the dirty baseline, so the parked dirty state +
        // undo/redo history are preserved exactly as they were. The view-model
        // (`work_content.editor_body`) is reconciled to the rope at the end of
        // this commit for all edited/restored documents.
        if restored_from_store {
            self.finalize_buffer_commit(true);
        }
        // ── Loading-state commit: buffer changed but no content yet ──
        // The explorer click path returns a ShellWorkContent with
        // editor_body=None as instant loading chrome.  The rope must be
        // cleared so the renderer does not fall back to the previous
        // file's rope content.  Without this, the editor shows the old
        // file's text under the new file's tab label. A restored document is
        // never cleared — its edited content is the source of truth.
        if buffer_changed && wc.editor_body.is_none() && !restored_from_store {
            if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
                let path = wc
                    .active_file
                    .as_deref()
                    .and_then(|s| s.strip_prefix("buf:"))
                    .unwrap_or("<none>");
                eprintln!(
                    "ZAROXI_VISIBLE_TAB_MODEL: loading_commit path={} intent_consumed=false {}",
                    path,
                    self.editor_group.diagnostic_line(),
                );
            }
            // Clear the rope to a single empty line so the renderer
            // shows a clean empty editor instead of old file content.
            self.editor_buffer.replace_content("");
            // Rope holds no document during the loading placeholder.
            self.active_rope_owner_path = None;
            self.visible_loading_state = true;
        }
        // ── Unconditional large-file materialization ──
        // For PieceTable-backed files on FIRST open (not a reactivation),
        // the rope must be populated from doc_buffers.  When the document
        // was already open and the view state was restored above via the
        // checkout path, this block is skipped — the rope was already
        // populated covering the restored scroll position.
        //
        // The initial window is viewport-sized (~200 lines, enough for the
        // first visible screenful plus overscan).  The rope is extended
        // on demand when the user scrolls past the current window via
        // `repopulate_large_file_rope`, which fetches additional lines from
        // the PieceTable without resetting the rope start (line 0).
        if self.large_file_mode
            && !restored_from_store
            && let Some(ref body) = wc.editor_body
        {
            if let Some(ref active_path) = wc.active_file
                && let Some(path) = active_path.strip_prefix("buf:")
                && let Some(db) = self.doc_buffers.get(path)
            {
                let total = db.total_lines();
                // Initial window: viewport-sized, not capped at 100.
                // The rope-extend path on scroll handles demand past this.
                let initial = 200usize.min(total);
                let vp = db.lines_in_range(0, initial.saturating_sub(1));
                let lines: Vec<String> = vp.into_iter().map(|(_, s)| s).collect();
                if !lines.is_empty() {
                    self.editor_buffer.populate_from_lines(
                        &lines,
                        body.cursor_line,
                        body.cursor_col,
                    );
                    // Rope now holds this large file's initial viewport window.
                    self.active_rope_owner_path = Some(path.to_string());
                }
                // Schedule initial syntax parse on the viewport slice.
                self.finalize_buffer_commit(true);
                if doc_lifecycle_trace_enabled() {
                    let ws = self.editor_buffer.window_start_line;
                    eprintln!(
                        "ZAROXI_DOC_LIFECYCLE: first_open_hydrate path={} source=piece_table_full window_start={} total={} loaded={} rope_after={}",
                        path,
                        ws,
                        total,
                        lines.len(),
                        self.editor_buffer.line_count(),
                    );
                }
            }
            if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DEBUG_LARGE_FILE: hydrating_rope_from_doc_buffers rope_lines={} doc_hit={} buff_changed={}",
                    self.editor_buffer.line_count(),
                    self.doc_buffers.contains_key(
                        wc.active_file
                            .as_deref()
                            .and_then(|s| s.strip_prefix("buf:"))
                            .unwrap_or("")
                    ),
                    buffer_changed,
                );
            }
        }
        if doc_lifecycle_trace_enabled() {
            eprintln!(
                "ZAROXI_DOC_LIFECYCLE: active_doc_changed prev={:?} new={:?} committed_before={:?} large_file_mode={} doc_buf_hit={} open_doc_hit={}",
                prev_active_file,
                wc.active_file.as_deref(),
                self.committed_active_file.as_deref(),
                self.large_file_mode,
                wc.active_file
                    .as_deref()
                    .and_then(|s| s.strip_prefix("buf:"))
                    .is_some_and(|p| self.doc_buffers.contains_key(p)),
                wc.active_file
                    .as_deref()
                    .and_then(|s| s.strip_prefix("buf:"))
                    .is_some_and(|p| self.open_documents.contains_key(p)),
            );
        }
        if self.large_file_mode
            && doc_lifecycle_trace_enabled()
            && let Some(path) = self
                .committed_active_file
                .as_deref()
                .or(wc.active_file.as_deref())
                .and_then(|s| s.strip_prefix("buf:"))
        {
            let rope_lines = self.editor_buffer.line_count();
            let pt_lines = self.doc_buffers.get(path).map(|db| db.total_lines()).unwrap_or(0);
            eprintln!(
                "ZAROXI_DOC_LIFECYCLE: large_file_state path={} rope_lines={} piece_table_total={} large_file_mode={} restored={}",
                path, rope_lines, pt_lines, self.large_file_mode, restored_from_store,
            );
        }
        self.committed_active_file = wc.active_file.clone();
        // ── Active-ownership enforcement ───────────────────────────
        // EditorGroup is the sole authority for the active file editor.
        // After ALL mutations above (intent processing, checkout,
        // hydration, view-model), reconcile committed_active_file to
        // match EditorGroup.active.  This runs unconditionally — even
        // for same-file transitions where content restore was skipped.
        // Without this, wc.active_file (from build_work_content, which
        // reads potentially-stale workspace-service metadata) can
        // overwrite the correct EditorGroup-driven value.
        self.reconcile_active_file_from_editor_group();
        // Post-commit invariant: visible tab / EditorGroup.active /
        // committed_active_file / active_buffer must all agree.  Self-heals
        // to EditorGroup.active on any drift so no downstream consumer can
        // read a stale cross-file identity.
        self.assert_active_consistency();
        self.last_committed_activation_seq = self.activation_seq;
        // ── View-model reconciliation for edited/restored normal documents ──
        // When the rope was NOT (re)built from `wc.editor_body` this commit
        // (`needs_rope_materialize == false`) but the active normal document has
        // unsaved edits, the freshly built `work_content.editor_body.lines` carry
        // the document's stale on-disk text. Bring them in line with the
        // authoritative rope so any `body.lines` consumer sees the edited content.
        // The renderer reads the rope directly, so this is belt-and-braces; it is
        // skipped on the common fresh-open path (rope already equals body).
        if !self.large_file_mode
            && !needs_rope_materialize
            && self.editor_buffer.is_dirty()
            && self.work_content.as_ref().and_then(|w| w.editor_body.as_ref()).is_some()
        {
            let lines = self.editor_buffer.lines_expanded();
            let cursor_line = self.editor_buffer.caret_line();
            let cursor_col = self.editor_buffer.caret_vis_col();
            if let Some(body) = self.work_content.as_mut().and_then(|w| w.editor_body.as_mut()) {
                body.lines = lines;
                body.cursor_line = cursor_line;
                body.cursor_col = cursor_col;
            }
        }
        if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
            eprintln!(
                "ZAROXI_DEBUG_LARGE_FILE: commit_open token={} backgrounded={} large_file_mode={} committed_active_file={:?} visible_loading={}",
                token,
                backgrounded,
                self.large_file_mode,
                self.committed_active_file,
                self.visible_loading_state,
            );
        }
        // Large files: the content lives in doc_buffers, not the rope.
        // The mapped doc is already ready in doc_buffers by the time this
        // commit runs (Mapped handler inserts before calling request_open).
        if self.large_file_mode {
            self.committed_open_token = token;
            self.visible_loading_state = false;
            self.background_open_pending = false;
        } else if !backgrounded {
            // Synchronous / no-op commit: this token's buffer is ready now.
            self.committed_open_token = token;
            self.visible_loading_state = false;
            self.background_open_pending = false;
        }
        // `work_content` was already set to this same `wc` by `request_open`.
        if !backgrounded {
            perf_event(
                "open_document",
                ev_start,
                &format!(
                    "token={} lines={} large_file={} lang={:?}",
                    token,
                    self.editor_buffer.line_count(),
                    self.large_file_mode,
                    self.current_language,
                ),
            );
            if file_open_trace_enabled() {
                let ttv =
                    self.open_request_at.map(|t| t.elapsed().as_secs_f32() * 1000.0).unwrap_or(0.0);
                eprintln!(
                    "ZAROXI_FILE_OPEN_TRACE: token={} stage=viewport cancelled=0 superseded_by=- time_to_first_viewport_ms={:.2}",
                    token, ttv,
                );
            }
        }
    }

    /// Shared finalization after a buffer becomes current (synchronous open or
    /// background open commit-on-ready): set the saved baseline, arm the open
    /// burst for a real buffer change, and kick off background syntax for
    /// non-large files.
    fn finalize_buffer_commit(&mut self, buffer_changed: bool) {
        // The dirty baseline is owned by `EditorBufferState`: a fresh load via
        // `populate_from_lines` / `install_rope` already marked the content saved
        // (clean) and cleared stale undo history, and a restore from the document
        // store deliberately keeps its parked dirty state. So this function must
        // NOT touch the saved baseline.
        // Enter open-settling so the next frame shapes the freshly-visible
        // viewport in one burst. Only for a genuine buffer change.
        self.open_settling = buffer_changed;
        self.open_burst_frames = 0;
        // Arm the one-time first-screenful budget for the first frame after this
        // commit so the visible rows shape at once rather than trickling.
        self.open_first_screenful_pending = buffer_changed;
        // Phase 11: arm the single atomic first-paint frame. That frame shapes the
        // entire visible screenful in one pass (large one-shot budget) so the new
        // file is presented coherently — content + chrome swap together — instead
        // of trickling rows and re-settling the top viewport across frames.
        self.open_atomic_first_paint = buffer_changed;
        // Maintain the open-presentation snapshot (tracing + atomic-present gate).
        // A read-initiated open already began one in `dispatch_activation`; a
        // direct open (workspace open / tab switch) begins one here. Either way
        // this is the moment the new buffer becomes current and the first paint is
        // being staged.
        if buffer_changed {
            let path = self.work_content.as_ref().and_then(|w| w.active_file.clone());
            match self.open_present.as_mut() {
                Some(present) if !present.presented => {
                    present.promoted_head_preview = present.used_head_preview;
                    // Adopt the committed chrome identity (active_file) so the
                    // present-time chrome-coherence check compares like-for-like.
                    present.path = path;
                }
                _ => {
                    self.open_present = Some(OpenPresentation::begin(self.open_token, path));
                }
            }
            if open_present_trace_enabled()
                && let Some(present) = self.open_present.as_ref()
            {
                eprintln!(
                    "ZAROXI_OPEN_PRESENT_TRACE: token={} stage=snapshot_building used_head_preview={} promoted_head_preview={} path={}",
                    present.token,
                    present.used_head_preview as u8,
                    present.promoted_head_preview as u8,
                    present.path.as_deref().unwrap_or("<none>"),
                );
            }
        }
        // ── Eager syntax readiness at activation (open / checkout) ──
        // Normal (non-huge) files are highlighted SYNCHRONOUSLY here so the very
        // first visible frame for this document is already coloured: the spans
        // are tied to the active `buffer_version`, which the strict render-side
        // span gate requires. This is what makes syntax immediate on open and
        // prevents the plain-text flash when a tab is checked out — syntax now
        // follows the document through the same activation contract as its text.
        // Large / huge files keep the off-thread (degraded) path so an expensive
        // full parse never runs on the UI thread.
        if self.should_eager_highlight() {
            let applied = self.schedule_background_parse();
            if syntax_trace_enabled() {
                eprintln!(
                    "ZAROXI_SYNTAX_TRACE: finalize mode=eager_sync applied={} buffer_version={} spans_version={} path={:?}",
                    applied,
                    self.editor_buffer.buffer_version,
                    self.latest_spans_version,
                    self.committed_active_file,
                );
            }
        } else {
            // Spawn the background parse worker for off-thread syntax highlighting.
            if self.parse_worker.is_none() {
                self.parse_worker = Some(background_parse::BackgroundParseWorker::spawn(
                    Arc::clone(&self.parser_pool),
                ));
            }
            // Schedule the off-thread tree-sitter parse. For large files the rope
            // holds only the viewport window so `to_string()` is viewport-scoped.
            if let Some(ref mut worker) = self.parse_worker {
                let text = self.editor_buffer.to_string();
                let version = self.editor_buffer.buffer_version;
                let language = self.current_language;
                if first_open_trace_enabled() {
                    eprintln!(
                        "ZAROXI_DEBUG_FIRST_OPEN: schedule_parse version={} text_len={} lang={:?} kind={}",
                        version,
                        text.len(),
                        language,
                        if self.large_file_mode { "large" } else { "normal" },
                    );
                }
                worker.schedule_parse(background_parse::BufferSnapshot {
                    version,
                    text,
                    language,
                    owner: self.committed_active_file.clone(),
                });
            }
            if syntax_trace_enabled() {
                eprintln!(
                    "ZAROXI_SYNTAX_TRACE: finalize mode=deferred_async large_file={} huge={} buffer_version={} path={:?}",
                    self.large_file_mode,
                    self.is_huge_file(),
                    self.editor_buffer.buffer_version,
                    self.committed_active_file,
                );
            }
        }
    }

    /// Commit-on-ready: install a completed background-open rope for the winning
    /// token. Stale results (a newer open superseded this one) are dropped so no
    /// old content ever flashes in. No-op when no result is pending.
    pub(crate) fn poll_open_results(&mut self) {
        let result = match self.open_worker.as_mut().and_then(|w| w.take_result()) {
            Some(r) => r,
            None => return,
        };
        if result.token != self.open_token {
            // Superseded by a newer open: drop without committing.
            if file_open_trace_enabled() {
                eprintln!(
                    "ZAROXI_OPEN_WORKER_TRACE: token={} started=1 finished=1 cancelled=1 chunks={} ms={:.2} stale_open_dropped=1 winning_token={}",
                    result.token,
                    result.chunks,
                    result.build_us as f32 / 1000.0,
                    self.open_token,
                );
            }
            // If this was the in-flight job we were waiting on and nothing newer
            // is pending in the worker, clear the pending flag.
            if self.open_worker.as_ref().map(|w| w.latest_token()).unwrap_or(0) <= result.token {
                self.background_open_pending = false;
            }
            return;
        }

        // Winning token: install the materialized rope (cheap on the UI thread).
        // Phase 5: this commit must stay cheap — the heavy viewport shaping is
        // NOT done here; it happens progressively over later frames under the
        // strict open shape budget.
        let commit_t = std::time::Instant::now();
        self.editor_buffer.install_rope(result.rope, result.cursor_line, result.cursor_col);
        // Background rope for the winning token has landed: the rope now owns
        // the active document's content.
        self.active_rope_owner_path = self
            .committed_active_file
            .as_deref()
            .map(|s| super::editor_group::canonical_path_from_editor_id(s).to_string());
        let install_rope_ms = commit_t.elapsed().as_secs_f32() * 1000.0;
        self.finalize_buffer_commit(true);
        let commit_ms = commit_t.elapsed().as_secs_f32() * 1000.0;
        self.committed_open_token = result.token;
        self.background_open_pending = false;
        self.visible_loading_state = false;
        let commit_latency_ms =
            self.open_worker_started_at.map(|t| t.elapsed().as_secs_f32() * 1000.0).unwrap_or(0.0);
        let ttv = self.open_request_at.map(|t| t.elapsed().as_secs_f32() * 1000.0).unwrap_or(0.0);
        perf_event(
            "open_document",
            self.open_worker_started_at.unwrap_or_else(std::time::Instant::now),
            &format!(
                "token={} lines={} large_file={} background=1",
                result.token,
                self.editor_buffer.line_count(),
                self.large_file_mode,
            ),
        );
        if file_open_trace_enabled() {
            eprintln!(
                "ZAROXI_OPEN_WORKER_TRACE: token={} started=1 finished=1 cancelled=0 chunks={} ms={:.2} worker_build_ms={:.2} upstream_open_prep_ms={:.2} open_commit_latency_ms={:.2} commit_to_first_visible_ms={:.2} open_install_rope_ms={:.3} open_commit_ms={:.3}",
                result.token,
                result.chunks,
                result.build_us as f32 / 1000.0,
                result.build_us as f32 / 1000.0,
                self.last_upstream_open_prep_ms,
                commit_latency_ms,
                commit_ms,
                install_rope_ms,
                commit_ms,
            );
            eprintln!(
                "ZAROXI_FILE_OPEN_TRACE: token={} stage=viewport cancelled=0 superseded_by=- first_viewport_after_worker_ms={:.2} time_to_first_viewport_ms={:.2}",
                result.token, commit_latency_ms, ttv,
            );
        }
        // Shape the freshly-installed viewport on the next frame.
        self.invalidate(InvalidationFlags::content());
    }

    /// Whether this file is heavy enough that its rope should be materialized on
    /// the background open worker (off the UI thread) rather than synchronously.
    fn should_background_open(lines: &[String]) -> bool {
        if lines.len() >= BACKGROUND_OPEN_LINE_THRESHOLD {
            return true;
        }
        let byte_count: usize = lines.iter().map(|l| l.len() + 1).sum();
        byte_count > BACKGROUND_OPEN_BYTE_THRESHOLD
    }

    /// Whether the current file is huge enough that full-document tree-sitter
    /// parsing should be skipped entirely in favour of plain-text fallback.
    pub(crate) fn is_huge_file(&self) -> bool {
        let total = self.editor_buffer.line_count();
        total > HUGE_FILE_LINE_THRESHOLD
    }

    /// Whether the active document is small enough to be highlighted
    /// SYNCHRONOUSLY at activation (open / checkout) without risking a UI-thread
    /// stall. Mirrors the background-open thresholds: large-file-mode,
    /// line-heavy (huge), and byte-heavy documents are excluded and keep the
    /// off-thread degraded path; everything else is eagerly coloured so its
    /// first visible frame is highlighted.
    fn should_eager_highlight(&self) -> bool {
        !self.large_file_mode
            && !self.is_huge_file()
            && self.editor_buffer.char_count() <= BACKGROUND_OPEN_BYTE_THRESHOLD
    }

    /// Synchronously (re)highlight the active document for its CURRENT
    /// `buffer_version`, storing the result in `latest_spans` /
    /// `latest_spans_version`.
    ///
    /// This is the single synchronous syntax source, shared by the edit path
    /// (after every keystroke) AND by open/checkout activation (via
    /// `finalize_buffer_commit`). Because it runs before the next frame and ties
    /// the spans to the active buffer version, the strict render-side span gate
    /// (`latest_spans_version == render_buffer_version`) accepts the result on
    /// the very first visible frame — so syntax is immediate on open and never
    /// disappears on tab checkout. It always reflects the in-memory edited text
    /// (not a saved baseline). Returns `true` when non-empty spans were applied.
    ///
    /// Unified syntax policy (shared with the background worker via
    /// `background_parse::compute_spans`), keyed off the SAME threshold as
    /// backend selection (`DocumentBuffer::LARGE_THRESHOLD`):
    /// - Rope-backed normal files (size < `LARGE_THRESHOLD`): full-document
    ///   re-highlight — enabled by default, no hidden "medium file" cutoff.
    /// - Large files (>= `LARGE_THRESHOLD`): the rope holds only the viewport,
    ///   so `to_string()` is small and the re-highlight is viewport-scoped.
    ///
    /// The parse budget therefore equals the large-file boundary, so syntax and
    /// backend selection can never disagree silently.
    pub(crate) fn schedule_background_parse(&mut self) -> bool {
        // For large files, `editor_buffer.rope()` contains only the viewport
        // window (~100 lines), so `to_string()` only tokenizes/colours the
        // visible lines.  This is intentional: piece-table full-file parsing
        // would be O(file_size), but viewport-scoped highlighting is O(1)
        // and covers the rendered area.
        let text = self.editor_buffer.to_string();
        let version = self.editor_buffer.buffer_version;
        let language = self.current_language;

        if std::env::var("ZAROXI_DEBUG_PARSE_PIPELINE").as_deref() == Ok("1") {
            eprintln!(
                "ZAROXI_DEBUG_PARSE_PIPELINE: sync_rehighlight v={} text_bytes={}",
                version,
                text.len(),
            );
        }

        // Synchronous re-highlight is the AUTHORITATIVE immediate result for the
        // current buffer version. It ALWAYS records the version it parsed (and
        // the owning file), even when the parse produced zero spans, so
        // `latest_spans`/`latest_spans_version` ALWAYS describe exactly the
        // version they claim (no stale carry-forward).
        let computed = background_parse::compute_spans_detailed(&self.parser_pool, language, &text);
        let owner = self.committed_active_file.clone();

        // ── Coverage precedence + downstream retention (normal files only) ──
        // Tree-sitter error recovery collapses coverage on a transient invalid
        // edit — dramatically for whitespace-sensitive grammars (YAML): a single
        // mid-typed line can drop coverage from the whole buffer to a handful of
        // leading bytes, blanking every later (unchanged, still-valid) line.
        //
        // Policy:
        //   - A CLEAN full-buffer parse (no error, non-empty) is authoritative
        //     and becomes the retained baseline.
        //   - A DEGRADED parse (Tree-sitter reported an error) never colors the
        //     actively edited region: the retained baseline is remapped across
        //     the edit (exact prefix/suffix diff) so later lines keep their
        //     correct colors, while the touched line(s) become an explicit PLAIN
        //     HOLE. Fresh degraded spans are dropped entirely — they may not
        //     color a single byte until the parse is clean again. This is
        //     preferred whenever a baseline exists (not gated on coverage), so
        //     the edited line is deterministically plain during invalid moments.
        //   - Large-file (viewport) mode keeps its own fresh-only policy; the
        //     baseline is never populated or consulted there (Part 7).
        let mut effective = computed.spans.clone();
        let mut kind = "full_buffer";
        if self.large_file_mode {
            kind = "viewport_window";
        } else if !computed.had_error && !computed.spans.is_empty() {
            // Clean full-buffer coverage → refresh the retained baseline.
            self.last_good_highlight = Some(background_parse::GoodHighlight {
                text: text.clone(),
                spans: computed.spans.clone(),
                version,
                owner: owner.clone(),
            });
        } else if computed.had_error
            && let Some(good) = self.last_good_highlight.as_ref().filter(|g| g.owner == owner)
        {
            let fresh_cov = crate::gui::window::syntax_color::coverage_end(&computed.spans);
            let remapped = crate::gui::window::syntax_color::remap_good_spans_across_edit(
                &good.text,
                &text,
                &good.spans,
            );
            let remapped_cov = crate::gui::window::syntax_color::coverage_end(&remapped);
            if std::env::var("ZAROXI_DEBUG_SYNTAX_APPLY").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DEBUG_SYNTAX_APPLY: syntax_result_accept kind=retained_suffix_plain_hole owner={:?} version={} source_len={} span_count={} fresh_cov={} retained_cov={}",
                    owner,
                    version,
                    text.len(),
                    remapped.len(),
                    fresh_cov,
                    remapped_cov,
                );
            }
            effective = remapped;
            kind = "retained_suffix_plain_hole";
        }
        let applied = !effective.is_empty();
        if std::env::var("ZAROXI_DEBUG_SYNTAX_APPLY").as_deref() == Ok("1") {
            // `source_len` is the exact byte length the parser saw; it MUST equal
            // the buffer text the renderer will apply these spans to. Emitting it
            // makes a coordinate-space drift (spans built from a different source
            // than rendered) immediately visible in the trace (Part 3).
            debug_assert_eq!(
                computed.source_len,
                text.len(),
                "span source_len must match the current buffer text length",
            );
            eprintln!(
                "ZAROXI_DEBUG_SYNTAX_APPLY: syntax_coverage kind={kind} owner={:?} version={} source_len={} had_error={} fresh_coverage_end={} span_count={} effective_coverage_end={}",
                owner,
                version,
                computed.source_len,
                computed.had_error,
                computed.coverage_end,
                effective.len(),
                crate::gui::window::syntax_color::coverage_end(&effective),
            );
        }
        self.latest_spans = Some(effective);
        self.latest_spans_version = version;
        self.latest_spans_owner = owner;
        // The line hash changes on every edit, so the editor cache already
        // rebuilds; clearing keeps the per-line syntax cache consistent and
        // guarantees no per-line colored payload survives across this edit.
        self.cached_editor_lines_hash = 0;
        self.line_syntax_cache.clear();
        if syntax_trace_enabled() {
            eprintln!(
                "ZAROXI_SYNTAX_TRACE: rehighlight_sync path={:?} lang={:?} buffer_version={} spans_version={} spans={} applied={} text_bytes={}",
                self.committed_active_file,
                language,
                version,
                self.latest_spans_version,
                self.latest_spans.as_ref().map(|s| s.len()).unwrap_or(0),
                applied,
                text.len(),
            );
        }
        applied
    }

    /// Drain background *read* outcomes (Phase 8/10/11). The `Head` outcome is
    /// telemetry only: it no longer performs a separate visible swap, because a
    /// head preview painted before the registered buffer is active leaves the
    /// chrome (tab/title/status) showing the *previous* file for a frame and
    /// forces a second top-of-viewport settle when the `Full` buffer lands. The
    /// old file (or loading shell) therefore stays visible until the single,
    /// coherent atomic first paint at the `Full` activation. The `Full` outcome
    /// activates the registered buffer and feeds it into the token-gated
    /// `request_open` path. Stale outcomes (a newer file was clicked) are dropped.
    pub(crate) fn poll_read_results(&mut self) {
        let outcomes = match self.read_worker.as_mut() {
            Some(w) => w.drain(),
            None => return,
        };
        if outcomes.is_empty() {
            return;
        }
        for outcome in outcomes {
            let tok = outcome.token();
            if tok != self.read_token {
                // Stale: a newer file was clicked. Drop without painting/activating.
                if std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_TABS: stale_async_dropped source=background_read owner=token:{tok} current=token:{}",
                        self.read_token,
                    );
                }
                if file_open_trace_enabled() {
                    let is_full = matches!(outcome, background_read::ReadOutcome::Full { .. });
                    let (cancelled, read_ms) = match &outcome {
                        background_read::ReadOutcome::Full { cancelled, read_ms, .. } => {
                            (*cancelled as u8, *read_ms)
                        }
                        background_read::ReadOutcome::Head { .. } => (0, 0.0),
                        background_read::ReadOutcome::Mapped { .. } => (0, 0.0),
                    };
                    eprintln!(
                        "ZAROXI_FILE_OPEN_TRACE: read_token={} stage=read_stale_dropped superseded_by={} is_full={} read_skipped_before_start={} wasted_read_ms={:.2}",
                        tok, self.read_token, is_full as u8, cancelled, read_ms,
                    );
                }
                if matches!(outcome, background_read::ReadOutcome::Full { .. })
                    || matches!(outcome, background_read::ReadOutcome::Mapped { .. })
                        && self.read_worker.as_ref().map(|w| w.latest_token()).unwrap_or(0) <= tok
                {
                    self.read_pending = false;
                }
                continue;
            }
            match outcome {
                background_read::ReadOutcome::Head { lines, complete, .. } => {
                    // Telemetry only — no visible swap (see fn doc). Record that a
                    // head preview was available so it is reported as folded into
                    // the single atomic first paint, not presented separately.
                    if self.read_pending && !lines.is_empty() {
                        let n = lines.len();
                        if let Some(present) = self.open_present.as_mut()
                            && present.token == tok
                            && !present.presented
                        {
                            present.used_head_preview = true;
                        }
                        if file_open_trace_enabled() || open_present_trace_enabled() {
                            let fb_ms = self
                                .read_started_at
                                .map(|t| t.elapsed().as_secs_f32() * 1000.0)
                                .unwrap_or(0.0);
                            eprintln!(
                                "ZAROXI_OPEN_PRESENT_TRACE: token={} stage=head_ready first_screenful_rows={} preview_complete={} presented_separately=0 time_to_head_ms={:.2}",
                                tok, n, complete as u8, fb_ms,
                            );
                        }
                    }
                }
                background_read::ReadOutcome::Mapped { doc, index_ms, .. } => {
                    self.read_pending = false;
                    self.last_upstream_open_prep_ms = index_ms;
                    self.large_file_mode = true;
                    let total = doc.total_lines();
                    let byte_len = doc.total_bytes();
                    let doc_path = doc.path().map(|p| p.to_path_buf());
                    let path_str = doc_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if file_open_trace_enabled() || doc_lifecycle_trace_enabled() {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: register_backend path={} kind=piece_table lines={} bytes={}",
                            path_str, total, byte_len,
                        );
                    }
                    let bid = crate::ports::BufferId(format!("buf:{}", path_str));
                    if file_open_trace_enabled() {
                        eprintln!(
                            "ZAROXI_FILE_OPEN_TRACE: read_token={} stage=mapped_doc_ready \
                             lines={} doc_bytes={} index_ms={:.1}",
                            tok, total, byte_len, index_ms,
                        );
                    }
                    if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
                        eprintln!(
                            "ZAROXI_VISIBLE_TAB_MODEL: resolved_commit path={} intent={:?} is_preview={} backend_kind=piece_table backend_registered=1",
                            path_str,
                            self.open_intent,
                            self.open_intent == Some(OpenIntent::Preview),
                        );
                    }
                    // Store in per-path map; the render pipeline looks up
                    // the active tab's buffer by path.
                    self.doc_buffers.insert(path_str.clone(), doc);
                    // Always register large files in opened_buffers so
                    // build_work_content() finds a valid active_buffer.
                    // EditorGroup is the sole authority for tab membership
                    // (preview vs pinned); opened_buffers is now a content
                    // registry only, not a tab-strip source.
                    if let Some(ref mut comp) = self.composition {
                        let display = doc_path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string());
                        comp.add_opened_buffer_direct(bid.clone(), display);
                    }
                    if let Some(ref mut comp) = self.composition {
                        if let Some(ref mut meta) = comp.metadata {
                            meta.active_buffer_details =
                                Some(crate::desktop::ActiveBufferDetails {
                                    buffer_id: bid.clone(),
                                    line_count: total,
                                    display: None,
                                });
                        }
                        // Feed through the normal request_open/commit_open
                        // path so committed_active_file is set and the
                        // render pipeline can look up doc_buffers.
                        let wc = comp.build_work_content();
                        self.request_open(wc);
                    }
                    self.open_settling = false;
                    self.commit_deferred_open = true;
                    self.needs_render = true;
                }
                background_read::ReadOutcome::Full { buffer_id, read_ms, .. } => {
                    self.last_upstream_open_prep_ms = read_ms;
                    let buffer_id = match buffer_id {
                        Some(b) => b,
                        None => {
                            self.read_pending = false;
                            if file_open_trace_enabled() {
                                eprintln!(
                                    "ZAROXI_FILE_OPEN_TRACE: read_token={} stage=read_failed",
                                    tok,
                                );
                            }
                            continue;
                        }
                    };
                    if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1") {
                        eprintln!(
                            "ZAROXI_VISIBLE_TAB_MODEL: resolved_commit path={} intent={:?} backend_kind=rope backend_registered=1",
                            buffer_id
                                .to_string()
                                .strip_prefix("buf:")
                                .unwrap_or(&buffer_id.to_string()),
                            self.open_intent,
                        );
                    }
                    // Finalize on the UI thread: activate the (already-read)
                    // buffer and build the real work content (cheap session
                    // lookups, no disk read).
                    let service = match self.workspace_service.clone() {
                        Some(s) => s,
                        None => {
                            self.read_pending = false;
                            continue;
                        }
                    };
                    let view = match self.workspace_view.clone() {
                        Some(v) => v,
                        None => {
                            self.read_pending = false;
                            continue;
                        }
                    };
                    let session = match self.session_id.clone() {
                        Some(s) => s,
                        None => {
                            self.read_pending = false;
                            continue;
                        }
                    };
                    let workspace_id = self.workspace_id;
                    let wc = {
                        let comp = match self.composition.as_mut() {
                            Some(c) => c,
                            None => {
                                self.read_pending = false;
                                continue;
                            }
                        };
                        comp.set_pending_refresh_reason(
                            zaroxi_application_workspace::workspace_view::RefreshReason::ActiveBufferChanged,
                        );
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
                        comp.build_work_content()
                    };
                    self.read_pending = false;
                    if file_open_trace_enabled() {
                        let read_to_request_ms = self
                            .read_started_at
                            .map(|t| t.elapsed().as_secs_f32() * 1000.0)
                            .unwrap_or(0.0);
                        eprintln!(
                            "ZAROXI_FILE_OPEN_TRACE: read_token={} stage=read_done read_ms={:.2} read_to_request_ms={:.2}",
                            tok, read_ms, read_to_request_ms,
                        );
                    }
                    // Feed into the existing token-gated open path (commit ->
                    // rope worker -> install), replacing the preview.
                    self.request_open(wc);
                }
            }
        }
    }

    /// Drain the background parse worker and store the latest accepted result.
    ///
    /// Only a result whose version matches the most recently *scheduled*
    /// version is accepted; this rejects stale results left over from a
    /// previous buffer or a superseded edit.  When a new result is stored we
    /// invalidate the editor caches and request a redraw so the fresh
    /// highlight spans become visible.
    pub(crate) fn poll_parse_results(&mut self) {
        // Defer applying a fresh highlight commit while the open viewport is
        // still settling or a resize is in flight, so we never combine a heavy
        // shaping pass / geometry reset with a full syntax recolor on the same
        // frame. The worker keeps the result queued (parse_result_pending stays
        // true), so it is applied on the next stable frame.
        self.commit_deferred_open = false;
        self.commit_deferred_resize = false;
        if self.parse_result_pending() {
            if self.open_settling {
                self.commit_deferred_open = true;
                return;
            }
            if self.resize_pending {
                self.commit_deferred_resize = true;
                return;
            }
        }
        let accepted = if let Some(ref mut worker) = self.parse_worker {
            let current = worker.latest_version();
            let got = match worker.poll_result() {
                Some(result) if result.version == current => {
                    Some((result.spans.clone(), result.version, result.owner.clone()))
                }
                _ => None,
            };
            if got.is_some() {
                worker.clear_result();
            }
            got
        } else {
            None
        };

        if let Some((spans, version, owner)) = accepted {
            // ── Strict async precedence + ownership ──
            // An async parse result may replace the current highlighting ONLY
            // when BOTH hold:
            //   1. It is strictly newer than the current spans version. The
            //      synchronous edit-time re-highlight advances
            //      `latest_spans_version` to the current buffer version on every
            //      keystroke, so any stale async result (older, or equal to a
            //      value a sync pass already committed) is dropped and can never
            //      downgrade or overwrite a correct newer sync state.
            //   2. It belongs to the currently active file. The result carries
            //      the file identity its snapshot was taken from; if the user has
            //      since switched files, the result is dropped rather than
            //      painted onto the wrong document.
            let owner_matches = owner.as_deref() == self.committed_active_file.as_deref();
            let newer = version > self.latest_spans_version;
            if newer && owner_matches {
                if std::env::var("ZAROXI_DEBUG_PARSE_PIPELINE").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_DEBUG_PARSE_PIPELINE: spans_stored v={} span_count={} lang={:?}",
                        version,
                        spans.len(),
                        self.current_language,
                    );
                }
                if first_open_trace_enabled() {
                    eprintln!(
                        "ZAROXI_DEBUG_FIRST_OPEN: parse_applied version={} span_count={} kind={} active_file={:?}",
                        version,
                        spans.len(),
                        if self.large_file_mode { "large" } else { "normal" },
                        self.committed_active_file,
                    );
                }
                self.latest_spans = Some(spans);
                self.latest_spans_version = version;
                self.latest_spans_owner = owner;
                // Force the editor shaping caches to rebuild with the new spans.
                self.cached_editor_lines_hash = 0;
                self.line_syntax_cache.clear();
                self.invalidate(InvalidationFlags::syntax());
            } else if std::env::var("ZAROXI_DEBUG_SYNTAX_APPLY").as_deref() == Ok("1") {
                let reason = if !owner_matches { "wrong_owner" } else { "stale_version" };
                eprintln!(
                    "ZAROXI_DEBUG_SYNTAX_APPLY: syntax_spans_rejected reason={reason} version={} latest_version={} result_owner={:?} active_file={:?}",
                    version, self.latest_spans_version, owner, self.committed_active_file,
                );
            }
        }
    }

    /// Whether the background worker has a scheduled parse whose result has not
    /// yet been applied.  Used to keep the event loop polling until the result
    /// arrives so highlights appear without requiring further user input.
    pub(crate) fn parse_result_pending(&self) -> bool {
        self.parse_worker
            .as_ref()
            .map(|w| w.latest_version() > self.latest_spans_version)
            .unwrap_or(false)
    }
}
