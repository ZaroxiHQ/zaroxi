/*!
Redraw pipeline orchestration for [`GuiApp`].

Owns the `RedrawRequested` body — widget-tree (re)build, editor-data
preparation, cockpit/status/diff shaping, and the actual frame present —
plus the frame/build metric helpers that only the redraw path consumes
(`WidgetTreeFingerprint`, FPS estimate, status fingerprint, diff-hunk
viewport transform, frame-presented tracker).
*/

use super::*;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Convert absolute-line git diff hunks into the **viewport-relative VISUAL row**
/// hunks the cockpit `LivingDiffLayer` paints, using the SAME row-layout model as
/// the editor text/gutter (so markers never drift relative to the line numbers or
/// wrapped rows).
///
/// `hunks` carry **absolute** 0-based *logical* line indices. Two cases:
/// - **No wrap** (`chars_per_row == 0` or no visual map): a logical line maps 1:1
///   to a visual row, so the viewport row is `line - scroll_top` (the same
///   line-aligned scroll the text/gutter use).
/// - **Wrap active**: each changed logical line is mapped through
///   `visual_to_logical` — the SAME map the gutter and cursor consume — to its
///   visual row(s); the viewport row is `abs_visual_row - wrap_visual_offset`. An
///   added line emits one hunk per visual row it occupies (so the bar spans the
///   line's full wrapped extent); a removal emits a single tick at its first
///   visual row.
///
/// The returned `line` is a 0-based viewport-relative VISUAL row; the diff layer
/// only adds the shared content-top origin and `line_height`. Large-file mode and
/// an empty set draw nothing. Recomputed every frame, so scroll/resize/wrap
/// always reposition or clear the markers.
///
/// A free function (not a `&self` method) so the call sites can pass disjoint
/// field borrows without taking a whole-`self` borrow that would clash with the
/// window.
/// Map the git crate's line change classification to the widget gutter's
/// three-state cue so `added` / `modified` / `removed` render with distinct
/// colors (green / indigo-blue / red) instead of collapsing add+modify.
fn git_change_kind_to_diff_kind(
    kind: zaroxi_core_platform_git::diff::ChangeKind,
) -> zaroxi_interface_widgets::components::DiffKind {
    use zaroxi_core_platform_git::diff::ChangeKind;
    use zaroxi_interface_widgets::components::DiffKind;
    match kind {
        ChangeKind::Added => DiffKind::Added,
        ChangeKind::Modified => DiffKind::Modified,
        ChangeKind::Removed => DiffKind::Removed,
    }
}

#[allow(clippy::too_many_arguments)]
fn diff_hunks_to_viewport(
    hunks: &[zaroxi_interface_widgets::components::DiffHunk],
    large_file_mode: bool,
    scroll_top: usize,
    editor_rect_height: f32,
    line_height: f32,
    visual_to_logical: &[usize],
    chars_per_row: usize,
    wrap_visual_offset: usize,
    active_file: Option<&str>,
    buffer_version: u64,
    cockpit_diff_version: u64,
) -> Vec<zaroxi_interface_widgets::components::DiffHunk> {
    if large_file_mode || hunks.is_empty() {
        return Vec::new();
    }
    // One row of slack so a marker on the last partially-visible row is kept.
    let visible_rows =
        if line_height > 0.0 { (editor_rect_height / line_height).ceil() as usize + 1 } else { 0 };
    let wrapping = chars_per_row > 0 && !visual_to_logical.is_empty();
    let trace = decoration_trace_enabled();
    let mut out = Vec::new();

    // Map one (logical line, absolute visual row) to a viewport-relative visual
    // row, dropping it when off-screen. `view_offset` is the scroll origin in the
    // same units as `abs_visual` (logical lines when unwrapped, visual rows when
    // wrapped) — matching how the text/gutter apply `content_offset_y`.
    let mut emit = |logical: usize,
                    kind: zaroxi_interface_widgets::components::DiffKind,
                    abs_visual: usize,
                    view_offset: usize| {
        let rel = abs_visual as isize - view_offset as isize;
        let kept = rel >= 0 && (rel as usize) < visible_rows;
        if trace {
            eprintln!(
                "ZAROXI_DEBUG_DECORATION: layer=diff source={:?} logical_line={} abs_visual_row={} viewport_row={} wrap={} scroll_top={} wrap_offset={} visible_rows={} active_file={:?} buffer_version={} cockpit_diff_version={} kept={}",
                kind,
                logical,
                abs_visual,
                rel,
                wrapping as u8,
                scroll_top,
                wrap_visual_offset,
                visible_rows,
                active_file,
                buffer_version,
                cockpit_diff_version,
                kept as u8,
            );
        }
        if kept {
            out.push(zaroxi_interface_widgets::components::DiffHunk { line: rel as usize, kind });
        }
    };

    for h in hunks {
        if wrapping {
            if h.kind.is_bar() {
                // Added/modified bars span every visual row the logical line occupies.
                for (abs_visual, &ll) in visual_to_logical.iter().enumerate() {
                    if ll == h.line {
                        emit(h.line, h.kind, abs_visual, wrap_visual_offset);
                    }
                }
            } else if let Some(abs_visual) = visual_to_logical.iter().position(|&ll| ll == h.line) {
                emit(h.line, h.kind, abs_visual, wrap_visual_offset);
            }
        } else {
            // Unwrapped: logical line == visual row; scroll is line-aligned.
            emit(h.line, h.kind, h.line, scroll_top);
        }
    }
    out
}

/// Recompute the cockpit git-diff hunks for the active buffer when the buffer has
/// advanced past the version the current hunks were computed for.
///
/// Called BEFORE the cockpit fingerprint/skip check so that an edit's markers
/// rebuild in the SAME frame as the edit (the fingerprint keys on
/// `cockpit_diff_version`; updating it first makes the skip check see the change
/// and rebuild immediately instead of a beat late). Idempotent: a no-op once the
/// version already matches, so calling it again later is free.
///
/// A disjoint-field free function so it can run while `core` is mutably borrowed.
/// Baseline lookups are cached by the provider, so the per-edit cost is only the
/// in-memory line diff — and large-file mode skips it entirely.
fn refresh_cockpit_diff_hunks(
    editor_buffer: &crate::gui::window::editor_buf::EditorBufferState,
    large_file_mode: bool,
    committed_active_file: Option<&str>,
    git_diff_provider: &mut zaroxi_core_platform_git::GitDiffProvider,
    cockpit_diff_hunks: &mut Vec<zaroxi_interface_widgets::components::DiffHunk>,
    cockpit_diff_version: &mut u64,
) {
    if editor_buffer.buffer_version == *cockpit_diff_version {
        return;
    }
    let diff_path = committed_active_file.map(|s| s.strip_prefix("buf:").unwrap_or(s).to_string());
    let hunks = if large_file_mode || editor_buffer.char_count() == 0 {
        // Large files skip live diff; an empty (not-yet-materialized) buffer would
        // diff as "whole file removed", so draw nothing until content arrives.
        Vec::new()
    } else if let Some(path) = diff_path {
        let current = editor_buffer.to_string();
        match git_diff_provider.diff_file(std::path::Path::new(&path), &current) {
            Some(fd) => fd
                .changed_lines
                .iter()
                .map(|c| zaroxi_interface_widgets::components::DiffHunk {
                    line: c.line,
                    kind: git_change_kind_to_diff_kind(c.kind),
                })
                .collect(),
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    if decoration_trace_enabled() {
        eprintln!(
            "ZAROXI_DEBUG_DECORATION: diff_recompute hunks={} buffer_version={} cockpit_diff_version={}->{} large_file={} active_file={:?} char_count={}",
            hunks.len(),
            editor_buffer.buffer_version,
            *cockpit_diff_version,
            editor_buffer.buffer_version,
            large_file_mode,
            committed_active_file,
            editor_buffer.char_count(),
        );
    }
    *cockpit_diff_hunks = hunks;
    *cockpit_diff_version = editor_buffer.buffer_version;
}

/// Cheap fingerprint of the `ShellWorkContent` fields that drive widget-tree
/// rebuilds. Replaces a per-frame full `ShellWorkContent` clone — which carried
/// the entire document body AND the whole explorer file tree — that existed
/// only to detect changes between frames. Cloning a few small fields (tab
/// names, active-file path, lengths) is O(1)-ish instead of O(document).
#[derive(Clone, PartialEq, Default)]
pub struct WidgetTreeFingerprint {
    explorer_empty_button: Option<String>,
    explorer_items_len: Option<usize>,
    explorer_scroll_top: usize,
    editor_lines_len: Option<usize>,
    active_file: Option<String>,
}

impl WidgetTreeFingerprint {
    fn of(wc: &ShellWorkContent) -> Self {
        Self {
            explorer_empty_button: wc.explorer_empty_button.clone(),
            explorer_items_len: wc.explorer_panel_items.as_ref().map(|v| v.len()),
            explorer_scroll_top: wc.explorer_scroll_top,
            editor_lines_len: wc.editor_body.as_ref().map(|b| b.lines.len()),
            active_file: wc.active_file.clone(),
        }
    }
}

/// Lightweight always-on inter-frame FPS estimate (EMA) for the cockpit health
/// band. Call exactly once per rendered frame; returns `None` on the warm-up
/// frame. Independent of the `ZAROXI_FPS_TRACE` diagnostic tracker.
fn current_fps_estimate() -> Option<u32> {
    use std::sync::Mutex;
    static TRACKER: Mutex<Option<(Instant, f32)>> = Mutex::new(None);
    let now = Instant::now();
    let mut guard = TRACKER.lock().ok()?;
    let fps = match guard.take() {
        Some((last, ema)) => {
            let dt = (now - last).as_secs_f32();
            let next = if dt > 0.0 {
                let inst = 1.0 / dt;
                if ema <= 0.0 { inst } else { ema * 0.9 + inst * 0.1 }
            } else {
                ema
            };
            *guard = Some((now, next));
            next
        }
        None => {
            *guard = Some((now, 0.0));
            0.0
        }
    };
    if fps >= 1.0 { Some(fps.round() as u32) } else { None }
}

/// Cheap 64-bit fingerprint of an `InstrumentStatus` + window geometry.
/// Used to skip cockpit rebuilds when nothing material changed.
fn instrument_status_fingerprint(
    s: &zaroxi_interface_widgets::InstrumentStatus,
    size: (u32, u32),
    diff_version: u64,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.context.leaf.hash(&mut h);
    s.context.position.hash(&mut h);
    s.health.fps.hash(&mut h);
    s.health.mem_mb.hash(&mut h);
    s.health.lsp.hash(&mut h);
    s.ai.mode.hash(&mut h);
    s.ai.tokens_used.hash(&mut h);
    s.rtl.hash(&mut h);
    size.hash(&mut h);
    diff_version.hash(&mut h);
    h.finish()
}

fn record_frame_presented() {
    if std::env::var("ZAROXI_FPS_TRACE").as_deref() != Ok("1") {
        return;
    }
    let now = std::time::Instant::now();
    use std::sync::Mutex;
    // One-off local perf tracker aggregate (last_instant, frames, bytes, ema, start).
    #[allow(clippy::type_complexity)]
    static TRACKER: Mutex<Option<(Option<std::time::Instant>, u64, u64, f64, std::time::Instant)>> =
        Mutex::new(None);
    let mut guard = TRACKER.lock().unwrap();
    if guard.is_none() {
        *guard = Some((None, 0, 0, 0.0, now));
    }
    let (last_frame, count, win_frames, win_sum_ms, win_start) = guard.as_mut().unwrap();
    *count += 1;
    let dt_ms: f64 = last_frame.map_or(0.0, |lf| (now - lf).as_secs_f64() * 1000.0);
    *last_frame = Some(now);

    *win_frames += 1;
    *win_sum_ms += dt_ms;
    let win_elapsed = (now - *win_start).as_secs_f64();
    if win_elapsed >= 1.0 {
        let avg_fps = *win_frames as f64 / win_elapsed;
        let avg_ms = *win_sum_ms / (*win_frames).max(1) as f64;
        eprintln!(
            "ZAROXI_FPS_TRACE: rolling frames={} avg_fps={:.1} avg_frame_ms={:.1}",
            win_frames, avg_fps, avg_ms
        );
        *win_start = now;
        *win_frames = 0;
        *win_sum_ms = 0.0;
    }
    eprintln!(
        "ZAROXI_FPS_TRACE: frame={} dt_ms={:.1} instant_fps={:.0}",
        count,
        dt_ms,
        if dt_ms > 0.0 { 1000.0 / dt_ms } else { 0.0 }
    );
}

/// Rebuild the structure-first minimap projection for the active document when
/// the cache key has changed, otherwise reuse the cached projection.
fn ensure_minimap_projection(
    cached: &mut zaroxi_interface_widgets::MinimapProjection,
    key: &mut (Option<String>, u64, usize),
    active_path: Option<&str>,
    editor_buffer: &crate::gui::window::editor_buf::EditorBufferState,
    doc_buffers: &std::collections::HashMap<String, zaroxi_core_editor_largefile::DocumentBuffer>,
    max_rows: usize,
) {
    let version = editor_buffer.buffer_version;
    let new_key = (active_path.map(|s| s.to_string()), version, max_rows);
    if *key == new_key {
        return;
    }
    *key = new_key.clone();
    *cached =
        if let (Some(_), Some(db)) = (active_path, active_path.and_then(|p| doc_buffers.get(p))) {
            let total = db.total_lines();
            zaroxi_interface_widgets::MinimapProjection::from_sampled(
                total,
                max_rows,
                crate::gui::window::editor_buf::EditorBufferState::TAB_WIDTH,
                |line| db.lines_in_range(line, line).into_iter().next().map(|(_, s)| s),
            )
        } else {
            let text = editor_buffer.to_string();
            let total = editor_buffer.total_lines();
            zaroxi_interface_widgets::MinimapProjection::from_lines(
                text.lines(),
                total,
                max_rows,
                crate::gui::window::editor_buf::EditorBufferState::TAB_WIDTH,
            )
        };
    if super::doc_lifecycle_trace_enabled()
        || std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1")
    {
        eprintln!(
            "ZAROXI_MINIMAP: minimap_projection_recomputed rows={} total_lines={} sampled={} path={}",
            cached.source_rows(),
            cached.total_lines,
            cached.sampled,
            active_path.unwrap_or("<none>"),
        );
    }
}

impl GuiApp {
    /// Render one frame in response to `WindowEvent::RedrawRequested`.
    pub(super) fn on_redraw_requested(&mut self) {
        let frame_id = GUI_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed);
        let perf_on = perf_trace_enabled();
        let frame_start = std::time::Instant::now();
        // A redraw arrived: clear the outstanding-redraw bookkeeping so a
        // later invalidation can schedule a fresh one.
        self.frame_scheduler.on_redraw_received();

        // Phase 8: commit a completed off-thread read first — it issues a
        // `request_open` for the freshly-read file, which `commit_open`
        // below then materializes this same frame.
        self.poll_read_results();

        // Stage B–E: materialize the newest pending open (if any) before
        // anything else this frame. Only the latest token commits, so a
        // rapid explorer switch never loads a superseded file.
        let _t_commit = if perf_on || pipeline_trace_enabled() {
            Some(std::time::Instant::now())
        } else {
            None
        };
        self.commit_open();
        let commit_open_ms = _t_commit.map(|t| t.elapsed().as_secs_f32() * 1000.0).unwrap_or(0.0);
        // Commit a completed background open (winning token only).
        self.poll_open_results();

        // Apply any completed background parse result before shaping the
        // editor content for this frame so fresh highlight spans are
        // used immediately (may invalidate the UI).
        let _t_parse = if perf_on || pipeline_trace_enabled() {
            Some(std::time::Instant::now())
        } else {
            None
        };
        self.poll_parse_results();
        let poll_parse_results_ms =
            _t_parse.map(|t| t.elapsed().as_secs_f32() * 1000.0).unwrap_or(0.0);

        // Integrated terminal: resize the PTY to the current layout and drain
        // any pending output before the dirty-check, so live shell output keeps
        // repainting even when the editor is otherwise idle.
        self.maintain_terminal();

        // Rebuild the Problems list from real diagnostics for this frame.
        self.refresh_problems();

        if frame_trace_enabled() {
            eprintln!(
                "ZAROXI_FRAME_TRACE: frame={} dirty={} reasons={}",
                frame_id,
                self.needs_render,
                self.frame_scheduler.pending_summary()
            );
        }
        if render_trace_enabled() {
            eprintln!(
                "ZAROXI_RENDER_TRACE: RedrawRequested frame={} dirty={}",
                frame_id, self.needs_render
            );
        }
        if !self.needs_render {
            if render_trace_enabled() {
                eprintln!(
                    "ZAROXI_RENDER_TRACE: RedrawRequested frame={} SKIPPED (not dirty)",
                    frame_id
                );
            }
            return;
        }

        let cursor_line = self.editor_cursor_line();
        let cursor_col = self.editor_cursor_col();
        // Visual (tab-expanded) caret column — the column space the rendered text
        // and the wrap mapping use. The caret is positioned and wrap-projected
        // from this single live value, not the raw char column.
        let cursor_vis_col = self.editor_buffer.caret_vis_col();
        // The caret's logical line text (raw), captured before the window borrow
        // so the caret's wrapped sub-row/column can be projected from the SAME
        // word-boundary plan the presenter wrapped with.
        let caret_line_text = self.editor_buffer.rope().line(cursor_line).unwrap_or_default();
        let selection_range = self.editor_selection_range();
        // Pre-compute values needed inside the render closure
        // (self is mutably borrowed via maybe_window below).
        let debug_large = std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1");
        let large_file_mode = self.large_file_mode;
        let _is_huge = self.is_huge_file();
        let _rope_line_count = self.editor_buffer.line_count();

        // Status bar inputs gathered before the window borrow below
        // (these use whole-`&self` accessors that cannot run while the
        // mutable `maybe_window` borrow `z` is held).
        let status_modified = self.document_modified();
        let status_parsing = self.parse_result_pending();
        let status_selection = self.status_selection();
        let status_diagnostics = self.status_diagnostics();
        let status_workspace_name = self
            .composition
            .as_ref()
            .and_then(|c| c.workspace_root_path.as_ref())
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        // Raw leading slice (line endings preserved) for indent + EOL detection.
        let status_text_sample = self.editor_buffer.raw_head(4096);
        // Active-document label from the best available real signal so the
        // status bar reflects the document the editor is actually showing,
        // even when the workspace's `active_file` id is not yet populated.
        let status_file_label = self.work_content.as_ref().and_then(|w| {
            w.active_file
                .clone()
                .or_else(|| w.editor_breadcrumb.clone())
                .or_else(|| w.editor_body.as_ref().map(|b| b.title.clone()))
                .filter(|s| !s.trim().is_empty())
        });

        // Exact monospace advance, captured before the window borrow so
        // the explorer presenter can size ellipsis / highlight columns.
        let mono_advance = self.monospace_advance_x().unwrap_or(lc::CHAR_WIDTH_STUB);

        // Dirty-document paths, captured before the window borrow so the tab
        // strip can be annotated with an unsaved-state cue over a disjoint
        // `tab_state` borrow (a whole-`self` call is impossible here).
        let dirty_doc_paths = self.dirty_doc_paths();

        // For large files, ensure the rope extends far enough to cover the
        // current scroll position before the render borrow locks self.
        // The ensure_caret_visible path also extends on caret movement;
        // this handles mouse-wheel / scrollbar-driven scroll changes.
        if self.large_file_mode {
            let scroll_end = self
                .composition
                .as_ref()
                .and_then(|c| c.metadata.as_ref())
                .map(|m| m.editor_scroll_top_line)
                .unwrap_or(0)
                .saturating_add(200);
            // A rope extension bumps `buffer_version` and leaves the previous
            // viewport-scoped spans describing the OLD (shorter) window. Re-run
            // the synchronous viewport re-highlight so `latest_spans` /
            // `latest_spans_version` match the newly extended window; without
            // this the strict render gate would blank syntax for the revealed
            // lines (a version mismatch) after every scroll.
            if self.repopulate_large_file_rope(scroll_end) {
                self.schedule_background_parse();
            }
        }

        // Deferred owner-mismatch re-hydration for normal files: the render
        // guard (below, inside the window borrow) can only blank the rope and
        // flag; the corrective re-open runs here where `&mut self` is free.
        // Bounded to one attempt per mismatch episode via
        // `owner_reload_attempted_for`, so it can never storm.
        if self.pending_owner_rehydrate {
            self.pending_owner_rehydrate = false;
            let loading =
                self.visible_loading_state || self.background_open_pending || self.read_pending;
            if !loading && let Some(comp) = self.composition.as_mut() {
                let wc = comp.build_work_content();
                self.request_open(wc);
            }
        }

        // Owner-correct scroll input: refresh the active document's total line
        // count from its backend before the window borrow (and thus before
        // apply_pending_scrolls), so a preview→pin (or any same-path)
        // transition can never starve the scroll clamp with a stale/zeroed
        // projection. Runs here (outside the window borrow) because it takes
        // `&mut self`.
        self.sync_authoritative_scroll_line_count();

        // Rail highlight is a DERIVED reflection of the canonical active tab,
        // never an independent authority. Re-derive it every frame from
        // WorkbenchTabState so the activity-rail selection can never desync
        // from the active destination / rendered content.
        self.sync_rail_reflection();

        if let Some(z) = self.maybe_window.as_mut() {
            let (sw, sh) = z.size();
            if sw == 0 || sh == 0 {
                if render_trace_enabled() {
                    eprintln!(
                        "ZAROXI_RENDER_TRACE: RedrawRequested frame={} SKIPPED (zero size)",
                        frame_id
                    );
                }
                return;
            }

            if self.startup_geometry_initial.is_none() {
                self.startup_geometry_initial = Some((sw, sh));
                if self.startup_geometry_changed_reason.is_none() {
                    self.startup_geometry_changed_reason = Some("no_change".to_string());
                }
            }

            // Notify compositor before rendering this frame.
            // Required on Wayland to register for the next frame callback.
            z.window().pre_present_notify();

            let system_is_dark =
                z.window().theme().map(|t| matches!(t, winit::window::Theme::Dark)).unwrap_or(true);
            let resolved = self.theme_mode.resolve(system_is_dark);
            let variant = resolved;

            let layout_t = std::time::Instant::now();
            let _ = self.layout_controller.get_or_compute(sw, sh, resolved);
            let layout_ms = layout_t.elapsed().as_secs_f32() * 1000.0;
            self.editor_viewport = Some(*self.layout_controller.viewport());

            let mut sem = variant.colors(false);

            let debug_theme_active = std::env::var("ZAROXI_DEBUG_THEME").as_deref() == Ok("1");
            if debug_theme_active {
                sem = zaroxi_interface_theme::theme::SemanticColors::debug();
                debug::gui_debug("ZAROXI_DEBUG_THEME: debug theme override ACTIVE");
            }

            if !self.first_render_shown && debug_theme_active {
                debug::gui_debug_fmt!(
                    "ZAROXI_THEME_TRACE: mode={:?} system_is_dark={} resolved={:?}",
                    self.theme_mode,
                    system_is_dark,
                    variant
                );
                debug::gui_debug_fmt!(
                    "ZAROXI_THEME_TRACE: sem.shell_background={:?} sem.app_background={:?} sem.editor_background={:?}",
                    sem.shell_background,
                    sem.app_background,
                    sem.editor_background
                );
            }

            let tokens =
                super::super::style_tokens_adapter::resolve_style_tokens(&sem, &Default::default());

            if !self.first_render_shown && debug_theme_active {
                debug::gui_debug_fmt!(
                    "ZAROXI_STYLE_TOKENS: app_bg={:?} titlebar_bg={:?} editor_bg={:?} sidebar_bg={:?}",
                    tokens.app_background.to_array(),
                    tokens.titlebar_background.to_array(),
                    tokens.editor_content_background.to_array(),
                    tokens.sidebar_background.to_array(),
                );
            }

            // ── Per-region theme dump (ZAROXI_THEME_REGION_DUMP=1) ──
            // Diagnostic: for each major surface prints the intended semantic
            // token, its intended sRGB hex, and the final RGBA handed to the
            // renderer — plus the value the OLD sRGB swapchain would have
            // re-encoded to. With the non-sRGB surface, `displayed_rgb` equals
            // the sent value (verbatim), proving no whitening is introduced.
            // Guarded: silent unless the env var is set; fires once per launch.
            if !self.first_render_shown
                && std::env::var("ZAROXI_THEME_REGION_DUMP").as_deref() == Ok("1")
            {
                // What a `*UnormSrgb` swapchain would have produced (the bug).
                fn srgb8(c: f32) -> u8 {
                    let v =
                        if c <= 0.0031308 { 12.92 * c } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
                    (v.clamp(0.0, 1.0) * 255.0).round() as u8
                }
                let regions: [(&str, zaroxi_interface_theme::Color, [f32; 4]); 14] = [
                    ("app_background", sem.app_background, tokens.app_background.to_array()),
                    (
                        "editor_background",
                        sem.editor_background,
                        tokens.editor_content_background.to_array(),
                    ),
                    (
                        "sidebar_background",
                        sem.sidebar_background,
                        tokens.sidebar_background.to_array(),
                    ),
                    ("panel_background", sem.panel_background, tokens.panel_background.to_array()),
                    (
                        "tab_strip_background",
                        sem.tab_strip_background,
                        tokens.tab_strip_background.to_array(),
                    ),
                    (
                        "tab_active_background",
                        sem.tab_active_background,
                        tokens.tab_active_background.to_array(),
                    ),
                    (
                        "tab_inactive_background",
                        sem.tab_background,
                        tokens.tab_inactive_background.to_array(),
                    ),
                    (
                        "assistant_panel_background",
                        sem.assistant_panel_background,
                        tokens.assistant_panel_background.to_array(),
                    ),
                    (
                        "status_bar_background",
                        sem.status_bar_background,
                        tokens.status_bar_background.to_array(),
                    ),
                    (
                        "title_bar_background",
                        sem.title_bar_background,
                        tokens.titlebar_background.to_array(),
                    ),
                    (
                        "editor_gutter_background",
                        sem.editor_gutter_background,
                        tokens.editor_gutter_bg.to_array(),
                    ),
                    (
                        "editor_line_highlight",
                        sem.editor_line_highlight,
                        tokens.editor_line_highlight.to_array(),
                    ),
                    ("editor_selection", sem.editor_selection, tokens.editor_selection.to_array()),
                    (
                        "border",
                        sem.border,
                        [sem.border.r, sem.border.g, sem.border.b, sem.border.a],
                    ),
                ];
                debug::gui_debug_fmt!(
                    "ZAROXI_THEME_REGION_DUMP: theme={:?} resolved={:?} (surface=non-sRGB Unorm → sent == displayed)",
                    self.theme_mode,
                    variant
                );
                for (name, intended, final_rgba) in regions.iter() {
                    debug::gui_debug_fmt!(
                        "ZAROXI_THEME_REGION_DUMP: region={:<26} token_hex={} sent_rgba=[{:.3},{:.3},{:.3},{:.3}] displayed=#{:02x}{:02x}{:02x} (old_sRGB_would_be=#{:02x}{:02x}{:02x})",
                        name,
                        intended.to_hex(),
                        final_rgba[0],
                        final_rgba[1],
                        final_rgba[2],
                        final_rgba[3],
                        (final_rgba[0] * 255.0).round() as u8,
                        (final_rgba[1] * 255.0).round() as u8,
                        (final_rgba[2] * 255.0).round() as u8,
                        srgb8(final_rgba[0]),
                        srgb8(final_rgba[1]),
                        srgb8(final_rgba[2]),
                    );
                }
            }

            if let Some(ref mut comp) = self.composition {
                comp.apply_pending_scrolls();
            }

            // Sync normalized scroll offset from canonical top_line to interaction model.
            // Must run unconditionally — small files (total <= visible) need offset 0.0
            // to avoid a stale value from a previous file.
            if let Some(ref comp) = self.composition
                && let Some(ref meta) = comp.metadata
            {
                let total_lines = self.editor_buffer.line_count();
                let visible = meta.editor_viewport_line_count.unwrap_or(10).max(1);
                let max_scroll = total_lines.saturating_sub(visible).max(1) as f32;
                let norm_offset =
                    (meta.editor_scroll_top_line as f32 / max_scroll.max(1.0)).clamp(0.0, 1.0);
                let editor_id = WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                self.interaction.set_scroll_offset(&editor_id, norm_offset);
            }

            let widget_t = std::time::Instant::now();
            let engine_layout = self.layout_controller.engine_shell_layout();

            // ── Explorer vertical scroll: clamp & publish offset ──
            // Publish BEFORE the fingerprint so a scroll-only change still
            // forces a widget-tree rebuild, keeping hit targets aligned
            // with the (always-rebuilt) render blocks.
            {
                let total_items = self
                    .work_content
                    .as_ref()
                    .and_then(|wc| wc.explorer_panel_items.as_ref())
                    .map(|items| items.len())
                    .unwrap_or(0);
                let has_title = self
                    .work_content
                    .as_ref()
                    .map(|wc| wc.explorer_panel_title.is_some())
                    .unwrap_or(false);
                let visible_rows =
                    lc::explorer_visible_rows(engine_layout.left_panel.height, has_title);
                let max_scroll = total_items.saturating_sub(visible_rows);
                if self.explorer_scroll_top > max_scroll {
                    self.explorer_scroll_top = max_scroll;
                }
                if let Some(wc) = self.work_content.as_mut() {
                    wc.explorer_scroll_top = self.explorer_scroll_top;
                    wc.explorer_search_active = self.explorer_search_active;
                }
            }

            let new_fingerprint = self.work_content.as_ref().map(WidgetTreeFingerprint::of);
            let content_changed = match (&self.last_widget_tree_fingerprint, &new_fingerprint) {
                (Some(old), Some(new)) => old != new,
                _ => true,
            };
            let rebuild_tree = self.last_widget_tree_size != (sw, sh) || content_changed;

            if rebuild_tree && first_open_trace_enabled() {
                // Isolate the rebuild reason so we can prove a file open
                // only mutates editor/tab state and never the explorer
                // subtree (which would be a flicker regression).
                let (explorer_changed, editor_changed, file_changed) =
                    match (&self.last_widget_tree_fingerprint, &new_fingerprint) {
                        (Some(old), Some(new)) => (
                            old.explorer_empty_button != new.explorer_empty_button
                                || old.explorer_items_len != new.explorer_items_len
                                || old.explorer_scroll_top != new.explorer_scroll_top,
                            old.editor_lines_len != new.editor_lines_len,
                            old.active_file != new.active_file,
                        ),
                        _ => (true, true, true),
                    };
                eprintln!(
                    "ZAROXI_DEBUG_FIRST_OPEN: rebuild_tree reason size_changed={} explorer_changed={} editor_changed={} active_file_changed={}",
                    self.last_widget_tree_size != (sw, sh),
                    explorer_changed,
                    editor_changed,
                    file_changed,
                );
            }

            self.last_widget_tree_size = (sw, sh);
            if new_fingerprint.is_some() {
                self.last_widget_tree_fingerprint = new_fingerprint;
            }

            // When Welcome is active, suppress the shell's empty-state
            // widget — the cockpit provides the Welcome screen instead.
            let shell_wc: Option<ShellWorkContent> = self.work_content.clone().map(|mut wc| {
                if matches!(
                    self.tab_state.active(),
                    super::super::destination::WorkbenchTabId::Welcome
                ) {
                    wc.suppress_empty_state = true;
                }
                // AI panel widget state: setup CTA + composer placeholder are
                // derived from the same normalized sources as the painted
                // blocks so hit targets and visuals stay in sync.
                {
                    use super::super::presenters::ai_presenter as aip;
                    let status = self.ai_provider_status.clone().unwrap_or_else(|| {
                        aip::derive_provider_status(
                            &self.settings.ai,
                            self.workspace_service.is_some(),
                        )
                    });
                    let loading = aip::session_is_loading(&self.ai_session);
                    wc.ai_show_setup_cta = !matches!(
                        status,
                        super::super::ai_pane::ProviderUiStatus::Connected { .. }
                            | super::super::ai_pane::ProviderUiStatus::Connecting
                    );
                    wc.ai_composer_placeholder =
                        Some(aip::composer_placeholder_for(&status, loading));
                }
                wc
            });

            let mut widget_tree = if rebuild_tree {
                zaroxi_core_engine_ui::build_shell_widget_tree(
                    engine_layout,
                    &tokens,
                    shell_wc.as_ref(),
                )
            } else {
                self.widget_tree.take().unwrap_or_else(|| {
                    zaroxi_core_engine_ui::build_shell_widget_tree(
                        engine_layout,
                        &tokens,
                        shell_wc.as_ref(),
                    )
                })
            };

            self.interaction.apply_to_tree(&mut widget_tree);

            // Fix editor scrollbar thumb height to match actual content ratio.
            let editor_id = WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
            let total_lines = self.editor_buffer.line_count().max(1);
            let visible = self
                .editor_viewport
                .as_ref()
                .map(|vp| lc::visible_lines_from_region(vp.content_rect.3) as usize)
                .unwrap_or(10)
                .max(1);
            let thumb_ratio = (visible as f32 / total_lines as f32).clamp(0.05, 1.0);
            for w in &mut widget_tree.widgets {
                if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
                    id,
                    track_rect,
                    thumb_rect,
                    ..
                } = w
                    && id == &editor_id
                {
                    let min_h = 20.0f32;
                    let new_h = (track_rect.height * thumb_ratio).max(min_h).min(track_rect.height);
                    thumb_rect.height = new_h;
                }
            }

            self.interaction.apply_scroll_offsets(&mut widget_tree);

            if scroll_trace_enabled() {
                let engine_layout = self.layout_controller.engine_shell_layout();
                let content_right = engine_layout.content_area.x + engine_layout.content_area.width;
                let ai_left = engine_layout.right_panel.x;
                let mut found = false;
                for w in &widget_tree.widgets {
                    if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
                        id,
                        track_rect,
                        thumb_rect,
                        ..
                    } = w
                        && id == &editor_id
                    {
                        eprintln!(
                            "ZAROXI_SCROLL_TRACE: widget_tree scrollbar rect=(ix={:.1},iy={:.1},iw={:.1},ih={:.1}) thumb_h={:.1} hit_right={:.1} content_right={:.1} ai_left={:.1}",
                            track_rect.x,
                            track_rect.y,
                            track_rect.width,
                            track_rect.height,
                            thumb_rect.height,
                            track_rect.x + track_rect.width,
                            content_right,
                            ai_left
                        );
                        found = true;
                    }
                }
                if !found {
                    eprintln!(
                        "ZAROXI_SCROLL_TRACE: widget_tree scrollbar MISSING total_widgets={} content_right={:.1} ai_left={:.1}",
                        widget_tree.widgets.len(),
                        content_right,
                        ai_left
                    );
                }
            }
            self.last_explorer_ids = self
                .work_content
                .as_ref()
                .and_then(|wc| wc.explorer_panel_items.as_deref())
                .map(|items| items.iter().map(|it| it.id.clone()).collect())
                .unwrap_or_default();
            debug::click_trace_fmt!(
                "ZAROXI_REDRAW: widget_tree built widgets={} cta_rect_present={}",
                widget_tree.widgets.len(),
                self.explorer_button_rect.is_some()
            );

            // Store the fully-interacted tree (move, not clone) so enrich
            // passes below can read interaction state from `self.widget_tree`.
            self.widget_tree = Some(widget_tree);
            let widget_ms = widget_t.elapsed().as_secs_f32() * 1000.0;

            // ── Startup trace (first 10 frames) ──────────────────
            let startup_trace =
                std::env::var("ZAROXI_STARTUP_TRACE").as_deref() == Ok("1") && frame_id < 10;
            let _t0 = if startup_trace { Some(std::time::Instant::now()) } else { None };
            let shell_regions = self.layout_controller.shell_regions();
            if startup_trace {
                let dt = _t0.unwrap().elapsed().as_secs_f32() * 1000.0;
                eprintln!(
                    "ZAROXI_STARTUP_TRACE: frame={} phase=initial_layout_compute ms={:.2}",
                    frame_id, dt
                );
            }
            debug::click_trace_fmt!(
                "ZAROXI_DIAG: window={}x{} layout_last={}x{} nregions={}",
                sw,
                sh,
                self.layout_controller.size().width,
                self.layout_controller.size().height,
                shell_regions.len(),
            );
            for r in shell_regions {
                if r.rect.width > 0 || r.rect.height > 0 {
                    debug::click_trace_fmt!(
                        "ZAROXI_DIAG:   region id={} x={} y={} w={} h={}",
                        r.id,
                        r.rect.x,
                        r.rect.y,
                        r.rect.width,
                        r.rect.height,
                    );
                }
            }
            let render_layout =
                super::super::renderbridge::build_render_layout(shell_regions, &tokens);

            self.shell.regions = shell_regions.to_vec();
            self.shell.size = *self.layout_controller.size();

            // Compute visible line range for viewport-only rendering.
            let editor_region = crate::gui::region_dispatch::find_region_by_role(
                shell_regions,
                zaroxi_core_engine_style::PanelRole::ContentArea,
            );
            let editor_visible_lines = editor_region
                .map(|r| lc::visible_lines_from_region(r.rect.height as f32))
                .unwrap_or(1);

            // Editor + status rects (owned Copy tuples) for the cockpit
            // overview/status anchoring further down. Captured here while
            // `shell_regions` is borrowed so the cockpit block needs no
            // further borrow of the layout controller.
            let cockpit_editor_rect: (f32, f32, f32, f32) = editor_region
                .map(|r| {
                    (r.rect.x as f32, r.rect.y as f32, r.rect.width as f32, r.rect.height as f32)
                })
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            let cockpit_status_rect: (f32, f32, f32, f32) =
                crate::gui::region_dispatch::find_region_by_role(
                    shell_regions,
                    zaroxi_core_engine_style::PanelRole::StatusBar,
                )
                .map(|r| {
                    (r.rect.x as f32, r.rect.y as f32, r.rect.width as f32, r.rect.height as f32)
                })
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            let cockpit_rail_rect: (f32, f32, f32, f32) =
                crate::gui::region_dispatch::find_region_by_role(
                    shell_regions,
                    zaroxi_core_engine_style::PanelRole::NavigationRail,
                )
                .map(|r| {
                    (r.rect.x as f32, r.rect.y as f32, r.rect.width as f32, r.rect.height as f32)
                })
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            let cockpit_tab_strip_rect: (f32, f32, f32, f32) =
                crate::gui::region_dispatch::find_region_by_role(
                    shell_regions,
                    zaroxi_core_engine_style::PanelRole::ContentTabStrip,
                )
                .map(|r| {
                    (r.rect.x as f32, r.rect.y as f32, r.rect.width as f32, r.rect.height as f32)
                })
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            let visible_line_range: Option<(usize, usize)> =
                self.composition.as_ref().and_then(|comp| comp.metadata.as_ref()).map(|meta| {
                    let scroll_top = meta.editor_scroll_top_line;
                    let scroll_end = scroll_top + editor_visible_lines;
                    (scroll_top, scroll_end.max(scroll_top + 1))
                });

            if std::env::var("ZAROXI_DEBUG_EDITOR_SPANS").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DEBUG_EDITOR_SPANS: prepare large_file_mode={} visible_range={:?} latest_spans={} spans_version={}",
                    large_file_mode,
                    visible_line_range,
                    self.latest_spans.as_ref().map(|s| s.len()).unwrap_or(0),
                    self.latest_spans_version,
                );
            }

            let syntax_t = std::time::Instant::now();
            let wrap_chars_per_row = {
                let available_w = if self.tab_state.is_editor_active() {
                    // Reserve exactly the right-edge cluster (minimap + scrollbar
                    // lane) so code wraps flush against the minimap's left edge
                    // with no dead gap between the last character and the minimap.
                    let cluster = super::super::cockpit::right_cluster_width();
                    editor_region
                        .map(|r| r.rect.width as f32 - lc::CONTENT_PAD_X - cluster)
                        .unwrap_or(600.0)
                        .max(40.0)
                } else {
                    editor_region
                        .map(|r| r.rect.width as f32 - lc::CONTENT_PAD_X)
                        .unwrap_or(600.0)
                        .max(40.0)
                };
                let scale = z.window().scale_factor() as f32;
                ((available_w * scale) / mono_advance).floor() as usize
            };
            let active_path_str = self
                .committed_active_file
                .as_deref()
                .map(|s| s.strip_prefix("buf:").unwrap_or(s).to_string());

            // ── Content-ownership enforcement (render source authority) ──
            // The rope is the visible render source. It may be presented ONLY
            // when it owns the active document. If the rope's owner path does
            // not equal the active canonical path — and we are NOT in a
            // legitimate loading state — the rope holds foreign or stale
            // content. Never show it: rebuild large-file viewports from the
            // canonical PieceTable, and re-hydrate normal files via a single
            // bounded re-open. This makes wrong-file content impossible.
            //
            // Only enforce while the EDITOR destination is active: when a
            // non-file tab (Settings, etc.) is showing, the file editor is not
            // projected, so the parked file rope must be left untouched.
            if let Some(active) = active_path_str.as_deref()
                && self.tab_state.is_editor_active()
            {
                let owner = self.active_rope_owner_path.as_deref();
                let owner_ok = owner == Some(active);
                if owner_ok {
                    // Owner agrees again: clear any spent reload marker.
                    if self.owner_reload_attempted_for.as_deref() == Some(active) {
                        self.owner_reload_attempted_for = None;
                    }
                } else {
                    // The rope does not own the active document. Two cases:
                    //   FOREIGN  — owner is Some(other): the rope literally
                    //              holds another file's text. This MUST NOT be
                    //              painted for even one frame, INCLUDING during
                    //              a loading state (the loading window is
                    //              exactly when cross-file bleed was visible).
                    //   EMPTY    — owner is None: the rope was already blanked;
                    //              a loading placeholder is safe, no bleed risk.
                    let foreign = owner.is_some();
                    let loading = self.visible_loading_state
                        || self.background_open_pending
                        || self.read_pending;
                    if (foreign || !loading)
                        && (std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1")
                            || std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1")
                            || std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1"))
                    {
                        eprintln!(
                            "ZAROXI_DOC_LIFECYCLE: content_owner_mismatch active={} rope_owner={} foreign={} loading={} large_file_mode={}",
                            active,
                            owner.unwrap_or("<none>"),
                            foreign,
                            loading,
                            large_file_mode,
                        );
                    }
                    // NOTE: only disjoint-field access is permitted here — the
                    // window (`z`) holds a `&mut self.maybe_window` borrow, so
                    // no whole-`self` method may be called.
                    if large_file_mode && self.doc_buffers.contains_key(active) {
                        // Rebuild the viewport window from the canonical
                        // PieceTable (owner-correct source), inline.
                        let lines: Vec<String> = if let Some(db) = self.doc_buffers.get(active) {
                            let end = 200usize.min(db.total_lines());
                            db.lines_in_range(0, end.saturating_sub(1))
                                .into_iter()
                                .map(|(_, s)| s)
                                .collect()
                        } else {
                            Vec::new()
                        };
                        if lines.is_empty() {
                            self.editor_buffer.replace_content("");
                            self.active_rope_owner_path = None;
                        } else {
                            self.editor_buffer.populate_from_lines(&lines, 0, 0);
                            self.active_rope_owner_path = Some(active.to_string());
                        }
                    } else if foreign {
                        // FOREIGN normal content — blank immediately so file A
                        // can NEVER be painted under file B, regardless of the
                        // loading flag. Defer the bounded re-open only when no
                        // load is already in flight (else the in-flight commit
                        // hydrates the correct file).
                        self.editor_buffer.replace_content("");
                        self.active_rope_owner_path = None;
                        if !loading && self.owner_reload_attempted_for.as_deref() != Some(active) {
                            self.owner_reload_attempted_for = Some(active.to_string());
                            self.pending_owner_rehydrate = true;
                        }
                    } else if !loading {
                        // EMPTY rope, not loading: bounded re-open from the
                        // canonical source.
                        self.editor_buffer.replace_content("");
                        self.active_rope_owner_path = None;
                        if self.owner_reload_attempted_for.as_deref() != Some(active) {
                            self.owner_reload_attempted_for = Some(active.to_string());
                            self.pending_owner_rehydrate = true;
                        }
                    }
                    // else: EMPTY + loading → genuine load in progress; a blank
                    // placeholder is shown, never another file's content.
                }
            }

            if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                let keys: Vec<&String> = self.doc_buffers.keys().collect();
                eprintln!(
                    "ZAROXI_DEBUG_LARGE_FILE: render_lookup committed_active_file={:?} lookup_key={:?} doc_buf_hit={} doc_buffers_keys={:?}",
                    self.committed_active_file,
                    active_path_str,
                    active_path_str
                        .as_ref()
                        .is_some_and(|p| self.doc_buffers.contains_key(p.as_str())),
                    keys,
                );
            }
            let doc_buf_keys: Vec<String> = self.doc_buffers.keys().cloned().collect();
            let doc_buf = active_path_str.as_ref().and_then(|p| self.doc_buffers.get_mut(p));
            if std::env::var("ZAROXI_DEBUG_VISIBLE_TABS").as_deref() == Ok("1")
                || std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1")
            {
                // Content-ownership binding: the render source is keyed
                // EXCLUSIVELY by the active canonical path, so it is
                // structurally impossible to bind another file's content.
                let source = if doc_buf.is_some() {
                    "doc_buffers"
                } else if large_file_mode {
                    "large_file_viewport"
                } else {
                    "rope"
                };
                eprintln!(
                    "ZAROXI_VISIBLE_TAB_MODEL: render_binding path={} source={}",
                    active_path_str.as_deref().unwrap_or("<none>"),
                    source,
                );
                // Full content-ownership assertion: the active tab, the active
                // file, the rope owner, and the render source must all agree.
                let allowed = active_path_str.as_deref() == self.active_rope_owner_path.as_deref()
                    || large_file_mode;
                eprintln!(
                    "ZAROXI_TABS: content_owner active_tab={:?} active_file={} rope_owner={} render_source={} allowed={}",
                    self.tab_state.active(),
                    active_path_str.as_deref().unwrap_or("<none>"),
                    self.active_rope_owner_path.as_deref().unwrap_or("<none>"),
                    source,
                    allowed,
                );
            }
            if std::env::var("ZAROXI_DEBUG_RENDER_SOURCE").as_deref() == Ok("1")
                || std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1")
            {
                let source_label = if doc_buf.is_some() {
                    "doc_buffers"
                } else if large_file_mode {
                    "large_file_no_doc"
                } else {
                    "rope"
                };
                let pt_total = doc_buf.as_ref().map(|db| db.total_lines());
                let rope_lines = self.editor_buffer.line_count();
                eprintln!(
                    "ZAROXI_DOC_LIFECYCLE: render_source source={} active_file={:?} rope_lines={} pt_total={:?} large_file_mode={} rope_is_derived_viewport={} doc_buffers_keys={:?}",
                    source_label,
                    self.committed_active_file,
                    rope_lines,
                    pt_total,
                    large_file_mode,
                    large_file_mode && pt_total.is_some_and(|t| rope_lines < t),
                    doc_buf_keys,
                );
            }
            // Invalidate cached editor data if the active file
            // identity changed since the last frame. This is a
            // belt-and-suspenders check: the content hash should
            // already differ, but identity-check guarantees no
            // cross-file pollution regardless of hash collision.
            if self.cached_editor_active_file.as_deref() != self.committed_active_file.as_deref() {
                let prev = self.cached_editor_active_file.as_deref().unwrap_or("<none>");
                let cur = self.committed_active_file.as_deref().unwrap_or("<none>");
                if std::env::var("ZAROXI_DEBUG_STALE_RENDER").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_DEBUG_STALE_RENDER: identity_mismatch prev={} cur={} clearing_cache",
                        prev, cur,
                    );
                }
                self.cached_editor_data = None;
                self.cached_editor_lines_hash = 0;
                self.cached_editor_spans_version = 0;
            }
            let is_large_doc = doc_buf
                .as_ref()
                .map(|db| matches!(db, zaroxi_core_editor_largefile::DocumentBuffer::Large(_)))
                .unwrap_or(false);
            // ── Strict syntax-snapshot gate ──
            // Highlight spans are parsed from `editor_buffer.to_string()`
            // at a specific `buffer_version` (full-document byte offsets).
            // They may be applied ONLY when they describe the EXACT rope
            // snapshot this frame renders — i.e. the stored spans' version
            // equals the current buffer version. Every other case means the
            // spans were computed from different text than what is on screen
            // now and would paint wrong colors (the transient open flash):
            //   - first parse still in flight after open (spans=None),
            //   - loading→ready materialization bumped the version,
            //   - large-file doc_buffers (re)hydration bumped the version,
            //   - a superseded async parse carrying an older version.
            // The synchronous re-highlight on edit (`schedule_background_parse`)
            // keeps `latest_spans_version == buffer_version` while typing, so
            // settled editing keeps its colors; only genuinely unverified
            // snapshots fall through to plain text. Plain-during-settle is
            // always preferred over wrong colors; correct colors appear on the
            // next frame once a parse for the current version lands.
            let render_buffer_version = self.editor_buffer.buffer_version;
            // Strict span ownership: spans may color the frame ONLY when they
            // describe this exact buffer version AND belong to the currently
            // active file. The version check rejects spans computed from a
            // different snapshot (open flash, materialization bump, superseded
            // async parse); the owner check closes the residual window where a
            // monotonic version could coincide across a file switch. Any failure
            // falls through to plain text — never wrong colors.
            let owner_matches =
                self.latest_spans_owner.as_deref() == self.committed_active_file.as_deref();
            let syntax_snapshot_verified = self.latest_spans.is_some()
                && self.latest_spans_version == render_buffer_version
                && owner_matches;
            let spans_for_render: &[HighlightSpan] = if syntax_snapshot_verified {
                self.latest_spans.as_deref().unwrap_or(&[])
            } else {
                &[]
            };
            let spans_version_for_render =
                if syntax_snapshot_verified { self.latest_spans_version } else { 0 };
            if first_open_trace_enabled() {
                let reason = if self.latest_spans.is_none() {
                    "no_spans_yet"
                } else if self.latest_spans_version != render_buffer_version {
                    "version_mismatch"
                } else if !owner_matches {
                    "owner_mismatch"
                } else {
                    "ok"
                };
                eprintln!(
                    "ZAROXI_DEBUG_FIRST_OPEN: span_gate verified={} reason={} render_plain_text={} latest_spans_len={} latest_spans_version={} render_buffer_version={} kind={} active_file={:?}",
                    syntax_snapshot_verified,
                    reason,
                    !syntax_snapshot_verified,
                    self.latest_spans.as_ref().map(|s| s.len()).unwrap_or(0),
                    self.latest_spans_version,
                    render_buffer_version,
                    if large_file_mode { "large" } else { "normal" },
                    self.committed_active_file,
                );
            }
            let editor_data = render_state::prepare_editor_data(
                &self.work_content,
                &mut self.cached_editor_data,
                &mut self.cached_editor_lines_hash,
                &mut self.cached_editor_spans_version,
                spans_for_render,
                spans_version_for_render,
                &sem,
                &mut self.line_syntax_cache,
                &mut self.cached_line_hashes,
                large_file_mode || is_large_doc,
                visible_line_range,
                Some(self.editor_buffer.rope()),
                doc_buf.as_deref(),
                self.editor_buffer.buffer_version,
                wrap_chars_per_row,
                // For large files the rope holds only the loaded prefix,
                // not the full document.  Total lines come from the
                // PieceTable backend so the scrollbar, caret-follow, and
                // render culling use the correct document size.
                if large_file_mode || is_large_doc {
                    Some(
                        doc_buf
                            .as_ref()
                            .map(|db| db.total_lines())
                            .unwrap_or_else(|| self.editor_buffer.line_count()),
                    )
                } else {
                    None
                },
                // Canonical content owner + true-owner-switch epoch: fold into
                // the cache key so a payload from another file (or a prior
                // owner of the same path) can never be reused.
                active_path_str.as_deref(),
                self.content_generation,
            );
            // Estimate retained editor bytes for memory trace.
            self.editor_retained_bytes = self
                .line_syntax_cache
                .values()
                .map(|v| v.iter().map(|(s, _)| s.len()).sum::<usize>())
                .sum::<usize>()
                + self.cached_line_hashes.len() * 8
                + self.latest_spans.as_ref().map(|s| s.len() * 32).unwrap_or(0);
            self.editor_visual_to_logical = editor_data.visual_to_logical.clone();
            self.editor_chars_per_row = editor_data.chars_per_row;
            self.editor_wrap_visual_offset = editor_data.wrap_visual_offset;
            // Track which file the cached editor data is for.
            self.cached_editor_active_file = self.committed_active_file.clone();

            if debug_large {
                let content_lines = editor_data.editor_body_text.lines().count();
                eprintln!(
                    "ZAROXI_DEBUG_LARGE_FILE: editor_data total={} content_lines={} content_bytes={} vis_range={:?}",
                    editor_data.total_lines,
                    content_lines,
                    editor_data.editor_body_text.len(),
                    editor_data.visible_line_range,
                );
            }
            if first_open_trace_enabled() {
                eprintln!(
                    "ZAROXI_DEBUG_FIRST_OPEN: render_frame frame={} kind={} active_file={:?} editor_data_total={} editor_data_content_lines={} editor_data_bytes={} has_spans={} spans_version={}",
                    frame_id,
                    if large_file_mode { "large" } else { "normal" },
                    self.committed_active_file,
                    editor_data.total_lines,
                    editor_data.editor_body_text.lines().count(),
                    editor_data.editor_body_text.len(),
                    editor_data.editor_spans.is_some(),
                    self.latest_spans_version,
                );
            }
            let mut explorer_data =
                super::super::presenters::shape_explorer_content(&self.work_content);
            // Exact monospace advance for ellipsis truncation + match-run
            // highlight positioning; blink-phased caret; keyboard nav row.
            explorer_data.char_advance = mono_advance;
            explorer_data.selected_row = self.explorer_search_sel;
            explorer_data.search_caret_visible = self.explorer_search_active
                && (self.explorer_caret_blink_epoch.elapsed().as_millis()
                    / CARET_BLINK_INTERVAL_MS)
                    .is_multiple_of(2);
            let ai_data = super::super::presenters::shape_ai_panel(
                super::super::presenters::AiPanelSources {
                    work_content: &self.work_content,
                    ai_settings: &self.settings.ai,
                    backend_available: self.workspace_service.is_some(),
                    session: &self.ai_session,
                    provider_override: self.ai_provider_status.clone(),
                    active_file: self.committed_active_file.as_deref(),
                    messages: Vec::new(),
                },
            );

            let status_inputs = super::super::status_bar::StatusInputs {
                file_label: status_file_label.as_deref(),
                workspace_name: status_workspace_name.as_deref(),
                cursor_line,
                cursor_col,
                text_sample: Some(status_text_sample.as_str()),
                modified: status_modified,
                parsing: status_parsing,
                readonly: large_file_mode,
                selection: status_selection,
                diagnostics: status_diagnostics,
            };
            // ── Startup trace: status model ──────────────────────
            let _ts = if startup_trace { Some(std::time::Instant::now()) } else { None };
            let status_data = super::super::presenters::shape_status_content(&status_inputs);
            self.status_model_generation += 1;
            if let Some(t) = _ts {
                eprintln!(
                    "ZAROXI_STARTUP_TRACE: frame={} phase=status_model_init ms={:.2}",
                    frame_id,
                    t.elapsed().as_secs_f32() * 1000.0
                );
            }
            if std::env::var("ZAROXI_STATUS_TRACE").as_deref() == Ok("1") {
                let sm_gen = self.status_model_generation;
                let from_open = self
                    .last_open_started_at
                    .map(|t| (std::time::Instant::now() - t).as_secs_f32() * 1000.0);
                let from_focus = self
                    .last_focus_change_at
                    .map(|t| (std::time::Instant::now() - t).as_secs_f32() * 1000.0);
                eprintln!(
                    "ZAROXI_STATUS_TRACE: status_model_generation={} status_model_latency_ms_from_open={:.1} status_model_latency_ms_from_focus_change={:.1}",
                    sm_gen,
                    from_open.unwrap_or(-1.0),
                    from_focus.unwrap_or(-1.0),
                );
                // Clear the timestamps so they're not reported again
                // until the next open / focus change.
                if from_open.is_some() {
                    self.last_open_started_at = None;
                }
                if from_focus.is_some() {
                    self.last_focus_change_at = None;
                }
            }
            // Canonical instrument-panel context + metadata bands (shared
            // presenter). The cockpit maps these into visual roles; the
            // legacy fallback bar derives the same facts via `status_zones`.
            // Derived here before `status_data` is moved into the block ctx.
            let (cockpit_context, cockpit_meta) =
                super::super::status_bar::instrument_context(&status_data);
            let syntax_ms = syntax_t.elapsed().as_secs_f32() * 1000.0;

            if std::env::var("ZAROXI_STATUS_DEBUG").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_STATUS_DEBUG: has_file={} ws={:?} state={:?} modified={} ln={} col={} sel={:?} indent={:?} eol={} lang={:?} diag={:?}",
                    status_data.has_file,
                    status_data.workspace,
                    status_data.document_state,
                    status_data.modified,
                    status_data.line + 1,
                    status_data.column + 1,
                    status_data.selection,
                    status_data.indent,
                    status_data.line_ending.label(),
                    status_data.language,
                    status_data.diagnostics,
                );
            }

            let block_t = std::time::Instant::now();
            let destination = self.tab_state.active().destination();
            if std::env::var("ZAROXI_DEBUG_TABS").as_deref() == Ok("1")
                && !destination.is_explorer()
            {
                eprintln!(
                    "ZAROXI_TABS: nonfile_tab_render destination={:?} tabid={:?} visible=true",
                    destination,
                    self.tab_state.active(),
                );
            }
            let (ext_sel, set_sel) =
                super::super::destination::sidebar_selection_for(self.tab_state.active());
            let sidebar_list =
                super::super::destination::sidebar_rows(destination, ext_sel, set_sel);
            let ctx = super::super::frame::ShellBlockContext {
                editor_data,
                explorer_data,
                status_bar_data: status_data,
                ai_data,
                terminal_tabs: self.work_content.as_ref().and_then(|wc| wc.terminal_tabs.clone()),
                destination,
                sidebar_list,
                cockpit_text_active: self.cockpit_text_active,
                welcome_active: matches!(
                    self.tab_state.active(),
                    super::super::destination::WorkbenchTabId::Welcome
                ),
            };

            // ── Startup trace: shell block composition ───────────
            let _tc = if startup_trace { Some(std::time::Instant::now()) } else { None };
            let (mut render_blocks, explorer_cta_rect, explorer_search_rect, sidebar_rows) =
                super::super::frame::compose_blocks(shell_regions, &tokens, &ctx);
            if let Some(t) = _tc {
                eprintln!(
                    "ZAROXI_STARTUP_TRACE: frame={} phase=first_frame_shell_build ms={:.2}",
                    frame_id,
                    t.elapsed().as_secs_f32() * 1000.0
                );
            }

            // Bottom panel: project the selected bottom tab onto its block via
            // the single unified rendering path — the live terminal grid for
            // Terminal, the real diagnostics list for Problems, or the in-app
            // log stream for Output. Borrows only disjoint app fields so it is
            // safe under the window borrow.
            {
                let palette = super::terminal::palette_from_tokens(&tokens);
                let bottom_tab = self.bottom_tab;
                let scroll = self.bottom_scroll;
                if let Some(block) =
                    render_blocks.iter_mut().find(|b| b.id == "center_bottom_panel")
                {
                    super::bottom_panel::render_tab(
                        bottom_tab,
                        &self.terminal,
                        &self.problems,
                        &self.output_log,
                        scroll,
                        block,
                        &palette,
                        &tokens,
                    );
                }
            }

            self.explorer_button_rect = explorer_cta_rect;
            self.explorer_search_rect = explorer_search_rect;
            self.sidebar_row_hit_rects = sidebar_rows;

            if std::env::var("ZAROXI_DEBUG_EDITOR_SPANS").as_deref() == Ok("1") {
                for block in &render_blocks {
                    let is_content = block.id.contains("ContentArea")
                        || block.id.contains("content_area")
                        || block.id == "editor_content";
                    if is_content {
                        eprintln!(
                            "ZAROXI_DEBUG_EDITOR_SPANS: render_block id='{}' content_bytes={} content_spans={:?} (styled_path={})",
                            block.id,
                            block.content.len(),
                            block.content_spans.as_ref().map(|s| s.len()),
                            block.content_spans.as_ref().map(|s| !s.is_empty()).unwrap_or(false),
                        );
                    }
                }
            }
            debug::click_trace_fmt!(
                "ZAROXI_REDRAW: cta_rect={:?}",
                explorer_cta_rect
                    .map(|(x, y, w, h)| format!("({:.0},{:.0},{:.0}x{:.0})", x, y, w, h))
            );

            let active_path_str2 = self
                .committed_active_file
                .as_deref()
                .map(|s| s.strip_prefix("buf:").unwrap_or(s).to_string());
            let editor_total_lines = active_path_str2
                .as_ref()
                .and_then(|p| self.doc_buffers.get(p))
                .map(|db| db.total_lines())
                .unwrap_or_else(|| self.editor_buffer.line_count());

            if let Some(ref mut comp) = self.composition {
                comp.set_editor_viewport_lines(editor_visible_lines);
            }

            let sidebar_region = crate::gui::region_dispatch::find_region_by_role(
                shell_regions,
                zaroxi_core_engine_style::PanelRole::SidePanel,
            );
            let sidebar_has_title = self
                .work_content
                .as_ref()
                .map(|wc| wc.explorer_panel_title.is_some())
                .unwrap_or(false);
            let sidebar_visible = sidebar_region
                .map(|r| lc::explorer_visible_rows(r.rect.height as f32, sidebar_has_title))
                .unwrap_or(1)
                .max(1);
            self.explorer_visible_rows = sidebar_visible;
            let sidebar_items = self
                .work_content
                .as_ref()
                .and_then(|wc| wc.explorer_panel_items.as_ref())
                .map(|items| items.len())
                .unwrap_or(0);
            let sidebar_scroll_offset = {
                let max_scroll = sidebar_items.saturating_sub(sidebar_visible).max(1) as f32;
                (self.explorer_scroll_top as f32 / max_scroll).clamp(0.0, 1.0)
            };

            let bottom_region = crate::gui::region_dispatch::find_region_by_role(
                shell_regions,
                zaroxi_core_engine_style::PanelRole::BottomPanel,
            );
            let bottom_visible = bottom_region
                .map(|r| lc::visible_lines_from_region(r.rect.height as f32))
                .unwrap_or(1);

            let editor_scroll_offset = self
                .interaction
                .get_scroll_offset(&WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR });

            let scroll_blocks = super::super::frame::compute_scrollbar_blocks(
                shell_regions,
                &tokens,
                editor_total_lines,
                editor_visible_lines,
                sidebar_items,
                sidebar_visible,
                0,
                bottom_visible,
                editor_scroll_offset,
                sidebar_scroll_offset,
            );
            render_blocks.extend(scroll_blocks);
            let block_build_ms = block_t.elapsed().as_secs_f32() * 1000.0;
            let enrich_t = std::time::Instant::now();

            // ── Scrollbar hover/active bridging (editor, sidebar, bottom) ──
            // On hover/active the thumb picks up a restrained Zaroxi accent tint —
            // clearly stronger than the passive faint thumb, consistent across all
            // three panels (previously only the editor scrollbar reacted).
            if let Some(ref tree) = self.widget_tree {
                for w in &tree.widgets {
                    if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
                        id: zaroxi_core_engine_ui::WidgetId::Scrollbar { index },
                        state,
                        ..
                    } = w
                    {
                        let thumb_id = match *index {
                            lc::SCROLLBAR_ID_EDITOR => "scrollbar_thumb_editor",
                            lc::SCROLLBAR_ID_SIDEBAR => "scrollbar_thumb_sidebar",
                            lc::SCROLLBAR_ID_BOTTOM => "scrollbar_thumb_bottom",
                            _ => continue,
                        };
                        let hover_color = match *state {
                            zaroxi_core_engine_ui::InteractionState::Hover
                            | zaroxi_core_engine_ui::InteractionState::Active => {
                                let mut c = tokens.accent.to_array();
                                c[3] = 0.55;
                                Some(c)
                            }
                            _ => None,
                        };
                        if let Some(color) = hover_color {
                            for block in &mut render_blocks {
                                if block.id == thumb_id {
                                    block.header_color = Some(color);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // ── Explorer row hover/focus bridging ──
            if let Some(ref tree) = self.widget_tree {
                for w in &tree.widgets {
                    if let zaroxi_core_engine_ui::ShellWidget::ListItem {
                        id: zaroxi_core_engine_ui::WidgetId::ListItem { index },
                        state,
                        ..
                    } = w
                        && *index >= 10
                    {
                        let row_idx = *index - 10;
                        let state = *state;
                        let hover_focus_color = match state {
                            zaroxi_core_engine_ui::InteractionState::Hover => {
                                Some(tokens.hover_bg.to_array())
                            }
                            zaroxi_core_engine_ui::InteractionState::Focused
                            | zaroxi_core_engine_ui::InteractionState::Selected => {
                                Some(tokens.rail_item_active.to_array())
                            }
                            _ => None,
                        };
                        if let Some(color) = hover_focus_color {
                            let block_id = format!("explorer_row_{}", row_idx);
                            for block in &mut render_blocks {
                                if block.id == block_id {
                                    block.header_color = Some(color);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if std::env::var("ZAROXI_DEBUG_SEAMS").as_deref() == Ok("1") {
                for blk in &render_blocks {
                    let narrow_or_tall = blk.rect.w <= 10.0 || blk.rect.h > blk.rect.w * 2.0;
                    if narrow_or_tall {
                        eprintln!(
                            "ZAROXI_SEAM: win={}x{} id='{}' x={:.1} y={:.1} w={:.1} h={:.1}",
                            sw, sh, blk.id, blk.rect.x, blk.rect.y, blk.rect.w, blk.rect.h,
                        );
                    }
                }
            }

            let is_content_block = |id: &str| {
                id.contains("ContentArea") || id.contains("content_area") || id == "editor_content"
            };
            if let Some(vp) = &self.editor_viewport {
                for block in &mut render_blocks {
                    if is_content_block(&block.id) {
                        // Caret visual row/col derive from the SINGLE live source
                        // (`editor_buffer`) via the shared, unit-tested projection
                        // — never from the stale `editor_body.cursor_*` view-model.
                        // The caret's wrapped sub-row and in-row column come from
                        // the caret line's own word-boundary plan, so both track
                        // the rendered rows even with unequal-width word wrap.
                        let (caret_sub_row, caret_col_in_row) =
                            super::super::presenters::editor_presenter::wrapped_caret_subrow_col(
                                &caret_line_text,
                                self.editor_chars_per_row,
                                cursor_vis_col,
                            );
                        let vis_cursor_line = super::caret_visual_row(
                            cursor_line,
                            caret_sub_row,
                            &self.editor_visual_to_logical,
                            self.editor_chars_per_row,
                        );
                        let vis_cursor_col = if self.editor_chars_per_row > 0 {
                            caret_col_in_row
                        } else {
                            cursor_vis_col
                        };
                        if caret_trace_enabled() {
                            eprintln!(
                                "ZAROXI_CARET_VIEWPORT: render_project logical_line={} vis_col={} map_len={} map_first={:?} map_last={:?} chars_per_row={} -> vis_row={} col={}",
                                cursor_line,
                                cursor_vis_col,
                                self.editor_visual_to_logical.len(),
                                self.editor_visual_to_logical.first(),
                                self.editor_visual_to_logical.last(),
                                self.editor_chars_per_row,
                                vis_cursor_line,
                                vis_cursor_col,
                            );
                        }
                        block.cursor_line = Some(vis_cursor_line);
                        block.cursor_col = Some(vis_cursor_col);
                        block.selection_range = selection_range;
                        // Glyph paint clip / line-highlight / selection right edge.
                        // The editor reserves EXACTLY the right-edge cluster
                        // (minimap + scrollbar lane) — the same reserve the wrap
                        // width uses above — so the paint clip ends precisely at
                        // the minimap's left edge. This is the single stable
                        // right-edge model:
                        //   clip_right == wrap_right == minimap_left
                        // Consequences:
                        //   * wrap width == paint clip width  → no glyph is cut
                        //     (wrapped lines break exactly where the clip ends).
                        //   * highlight/selection fill to the minimap's edge  →
                        //     no dead band, no early stop.
                        //   * the opaque minimap begins where text stops  → no
                        //     glyph/minimap collision, no fake gap.
                        // The previous stale `- 100.0` here was 42px narrower
                        // than the (58px) cluster reserve, which both cut the
                        // last characters and opened a dead strip before the
                        // minimap.
                        let clip_w = if self.tab_state.is_editor_active() {
                            (vp.clip_rect.2 - super::super::cockpit::right_cluster_width()).max(0.0)
                        } else {
                            vp.clip_rect.2
                        };
                        block.clip_rect = Some(zaroxi_core_engine_render::Rect {
                            x: vp.clip_rect.0,
                            y: vp.clip_rect.1,
                            w: clip_w,
                            h: vp.clip_rect.3,
                        });
                        if let Some(ref comp) = self.composition
                            && let Some(meta) = &comp.metadata
                        {
                            block.content_offset_x =
                                meta.editor_horizontal_offset_px.unwrap_or(0.0);
                            let off_y = if self.editor_chars_per_row > 0 {
                                self.editor_wrap_visual_offset as f32 * lc::LINE_HEIGHT
                            } else {
                                meta.editor_scroll_top_line as f32 * lc::LINE_HEIGHT
                            };
                            block.content_offset_y = off_y;
                            if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref() == Ok("1") {
                                eprintln!(
                                    "ZAROXI_SCROLL: block content_offset x={:.1} y={:.1} top_line={}",
                                    block.content_offset_x, off_y, meta.editor_scroll_top_line
                                );
                            }
                        }
                    }
                }

                // Apply vertical scroll offset to the gutter lane block
                if let Some(ref comp) = self.composition
                    && let Some(meta) = &comp.metadata
                {
                    let off_y = if self.editor_chars_per_row > 0 {
                        self.editor_wrap_visual_offset as f32 * lc::LINE_HEIGHT
                    } else {
                        meta.editor_scroll_top_line as f32 * lc::LINE_HEIGHT
                    };
                    for block in &mut render_blocks {
                        if block.id == "gutter_lane" {
                            block.clip_rect = Some(zaroxi_core_engine_render::Rect {
                                x: block.rect.x,
                                y: block.rect.y,
                                w: block.rect.w,
                                h: block.rect.h,
                            });
                            block.content_offset_y = off_y;
                            block.content_offset_x =
                                meta.editor_horizontal_offset_px.unwrap_or(0.0);
                            break;
                        }
                    }
                }
            } else {
                for block in &mut render_blocks {
                    if is_content_block(&block.id) {
                        block.cursor_line = Some(cursor_line);
                        block.cursor_col = Some(cursor_vis_col);
                        block.selection_range = selection_range;
                    }
                }
            }

            // ── Renderer lifecycle ──
            let enrich_ms = enrich_t.elapsed().as_secs_f32() * 1000.0;
            self.last_render_size = (sw, sh);

            let clear_color = [
                tokens.app_background.r as f64,
                tokens.app_background.g as f64,
                tokens.app_background.b as f64,
                1.0,
            ];

            // ── Per-frame content trace (ZAROXI_RENDER_TRACE=1) ──
            if render_trace_enabled() {
                let editor_body_hash = self
                    .work_content
                    .as_ref()
                    .and_then(|wc| wc.editor_body.as_ref())
                    .map(|cv| {
                        let mut h: u64 = 0;
                        for line in cv.lines.iter() {
                            h = h.wrapping_mul(31).wrapping_add(line.len() as u64);
                        }
                        h
                    })
                    .unwrap_or(0);
                let explorer_count = self
                    .work_content
                    .as_ref()
                    .map(|wc| wc.explorer_items.as_ref().map(|v| v.len()).unwrap_or(0))
                    .unwrap_or(0);
                let mut rblock_hash: u64 = 0;
                for blk in &render_blocks {
                    rblock_hash = rblock_hash.wrapping_mul(31).wrapping_add(blk.id.len() as u64);
                    rblock_hash =
                        rblock_hash.wrapping_mul(31).wrapping_add(blk.content.len() as u64);
                    rblock_hash =
                        rblock_hash.wrapping_mul(31).wrapping_add((blk.rect.x * 100.0) as u64);
                    rblock_hash =
                        rblock_hash.wrapping_mul(31).wrapping_add((blk.rect.y * 100.0) as u64);
                }
                eprintln!(
                    "ZAROXI_RENDER_TRACE: app_frame frame={} work_hash={:016x} explorer_count={} rblocks={} rblock_hash={:016x}",
                    frame_id,
                    editor_body_hash,
                    explorer_count,
                    render_blocks.len(),
                    rblock_hash
                );
            }

            // Snapshot the pending invalidation reasons before the
            // renderer borrow so the retained-node tracer can label the
            // dirty reasons for this frame.
            let ui_flags = self.frame_scheduler.pending();
            // Whether this frame is part of an open settle, and whether
            // it is the one-time first-screenful frame (full visible-row
            // budget) vs. a progressive below-the-fold fill frame.
            let open_settling = self.open_settling;
            let open_first_screenful = self.open_first_screenful_pending;
            // Phase 11: the single atomic first-paint frame uses a large
            // one-shot budget so the WHOLE visible screenful shapes in one
            // pass — the new file is presented coherently and atomically,
            // never as a partially-shaped top viewport. (Editor content is
            // viewport-windowed, so this is screenful-bounded.) Subsequent
            // open frames drop to the low progressive below-the-fold budget.
            let open_atomic_first_paint = self.open_atomic_first_paint;
            let open_budget_ms = if open_atomic_first_paint {
                OPEN_ATOMIC_FIRST_PAINT_BUDGET_MS
            } else if open_first_screenful {
                open_first_screenful_budget_ms()
            } else {
                open_progressive_budget_ms()
            };

            // Create persistent RenderCore on first frame.
            let core_exists = self.render_core.is_some();
            let mut render_core_create_ms: f32 = 0.0;
            if !core_exists {
                let _t_core = std::time::Instant::now();
                let window_arc = z.window_arc();
                let surface_size = winit::dpi::PhysicalSize::new(sw, sh);
                match pollster::block_on(
                    zaroxi_core_engine_render::renderer::core::RenderCore::new(
                        window_arc,
                        clear_color,
                        surface_size,
                    ),
                ) {
                    Ok(core) => {
                        render_core_create_ms = _t_core.elapsed().as_secs_f32() * 1000.0;
                        self.render_core = Some(core);
                        if zaroxi_core_telemetry::startup_trace_enabled() {
                            eprintln!(
                                "MEM_STARTUP: after_renderer_init rss={:.1}MB",
                                zaroxi_core_telemetry::rss_mb()
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("GuiApp: failed to create RenderCore: {:?}", e);
                        return;
                    }
                }
            }

            if let Some(ref mut core) = self.render_core {
                let surface_size = winit::dpi::PhysicalSize::new(sw, sh);
                // Phase 6: two-tier open budget. The first post-commit
                // open frame gets the (bounded) first-screenful budget so
                // the visible rows shape at once; later open frames use
                // the low progressive budget for below-the-fold/overscan
                // rows. Neither tier is a 250 ms burst. Non-open frames
                // keep the steady-state budget.
                core.set_shape_budget_ms(if open_settling { Some(open_budget_ms) } else { None });
                // ── Cockpit build BEFORE render pass ────────────
                // Built here so the cockpit scene is available
                // for this frame's render pass, not next frame's.
                {
                    let do_cockpit = super::super::cockpit::cockpit_surfaces_active();
                    if do_cockpit {
                        let rail_style_colors = (
                            tokens.rail_background.to_array(),
                            tokens.rail_item_active.to_array(),
                            tokens.rail_item_active_accent.to_array(),
                            tokens.text_primary.to_array(),
                            tokens.text_muted.to_array(),
                            tokens.divider_subtle.to_array(),
                        );
                        let cockpit_tokens =
                            super::super::cockpit::cockpit_tokens(self.theme_mode, system_is_dark);
                        let ai_band = {
                            use zaroxi_application_ai::view_model::AiPhase;
                            let used = self.ai_session.tokens_streamed as u32;
                            let mode = match self.ai_session.phase {
                                AiPhase::Idle => zaroxi_interface_widgets::AiMode::Dormant,
                                AiPhase::PromptBuilding
                                | AiPhase::Requesting
                                | AiPhase::Streaming => zaroxi_interface_widgets::AiMode::Live,
                                AiPhase::Complete => {
                                    if used > 0 {
                                        zaroxi_interface_widgets::AiMode::Degraded
                                    } else {
                                        zaroxi_interface_widgets::AiMode::Dormant
                                    }
                                }
                            };
                            zaroxi_interface_widgets::AiBand {
                                mode,
                                tokens_used: used,
                                tokens_total: 0,
                                model: None,
                                latency_ms: self
                                    .ai_session
                                    .first_token_ms
                                    .map(|ms| ms.round() as u32),
                            }
                        };
                        let health_band = zaroxi_interface_widgets::HealthBand {
                            fps: current_fps_estimate(),
                            mem_mb: self
                                .last_mem_sample
                                .as_ref()
                                .map(|s| (s.rss_bytes / (1024 * 1024)) as u32),
                            lsp: zaroxi_interface_widgets::LspStatus::Healthy,
                        };
                        let status_rtl = cockpit_context.leaf.chars().any(|c| {
                                    matches!(c, '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}')
                                });
                        let instrument_status = zaroxi_interface_widgets::InstrumentStatus {
                            context: cockpit_context.clone(),
                            meta: cockpit_meta.clone(),
                            health: health_band,
                            ai: ai_band,
                            rtl: status_rtl,
                        };
                        // Refresh git-diff hunks BEFORE the fingerprint below so an
                        // edit's markers rebuild THIS frame: the fingerprint keys on
                        // `cockpit_diff_version`, so updating it first makes the skip
                        // check see the change (otherwise the rebuild lagged a frame).
                        refresh_cockpit_diff_hunks(
                            &self.editor_buffer,
                            self.large_file_mode,
                            self.committed_active_file.as_deref(),
                            &mut self.git_diff_provider,
                            &mut self.cockpit_diff_hunks,
                            &mut self.cockpit_diff_version,
                        );
                        let fp = instrument_status_fingerprint(
                            &instrument_status,
                            (sw, sh),
                            self.cockpit_diff_version,
                        );
                        let skip =
                            self.cockpit_text_active && fp == self.cockpit_status_fingerprint;
                        self.cockpit_status_fingerprint = fp;
                        if !skip {
                            // File tabs come exclusively from EditorGroup.
                            self.editor_group.check_invariants();
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
                                    "ZAROXI_VISIBLE_TAB_MODEL: frame {} opened_buffers={:?} large_file_mode={}",
                                    self.editor_group.diagnostic_line(),
                                    ob_tabs,
                                    self.large_file_mode,
                                );
                                // Check: preview path must not be in opened_buffers unless also pinned.
                                if let Some(pp) = self.editor_group.preview_path() {
                                    let in_ob = ob_tabs.iter().any(|b| b == pp);
                                    let is_pinned = self.editor_group.is_pinned(pp);
                                    if in_ob && !is_pinned {
                                        eprintln!(
                                            "ZAROXI_VISIBLE_TAB_MODEL: invariant_violation preview_path_in_opened_buffers path={} is_pinned={}",
                                            pp, is_pinned,
                                        );
                                    }
                                }
                                // Check: active_doc matches editor_group.active.
                                let eg_active = self.editor_group.active_path();
                                let doc_active = self
                                    .committed_active_file
                                    .as_deref()
                                    .and_then(|s| s.strip_prefix("buf:"));
                                if eg_active != doc_active {
                                    eprintln!(
                                        "ZAROXI_VISIBLE_TAB_MODEL: invariant_violation active_mismatch editor_group={:?} committed_active_file={:?}",
                                        eg_active, doc_active,
                                    );
                                }
                            }
                            let workbench_tabs = super::annotate_tabs_dirty(
                                self.tab_state.projected_tabs(
                                    &self.editor_group,
                                    self.committed_active_file.as_deref(),
                                ),
                                &dirty_doc_paths,
                            );
                            let cockpit_tabs: Vec<zaroxi_interface_widgets::CockpitTab> =
                                workbench_tabs
                                    .iter()
                                    .map(|t| zaroxi_interface_widgets::CockpitTab {
                                        title: t.title.clone(),
                                        active: t.active,
                                        closable: t.closable,
                                        kind: t.kind,
                                        is_preview: t.is_preview,
                                    })
                                    .collect();
                            let (dp_settings, dp_extensions, dp_placeholder) =
                                super::super::destination::cockpit_panels_for(
                                    self.tab_state.active(),
                                );
                            // ── Pre-compute minimap fields for CockpitInputs ──
                            let active_path_deref = self
                                .committed_active_file
                                .as_deref()
                                .map(|s| s.strip_prefix("buf:").unwrap_or(s));
                            let mm_max_rows = (cockpit_editor_rect.3 as usize).clamp(64, 2000);
                            ensure_minimap_projection(
                                &mut self.cockpit_minimap,
                                &mut self.cockpit_minimap_key,
                                active_path_deref,
                                &self.editor_buffer,
                                &self.doc_buffers,
                                mm_max_rows,
                            );
                            let mm_viewport = {
                                let scroll_top = self
                                    .composition
                                    .as_ref()
                                    .and_then(|c| c.metadata.as_ref())
                                    .map(|m| m.editor_scroll_top_line)
                                    .unwrap_or(0);
                                let visible = self
                                    .composition
                                    .as_ref()
                                    .and_then(|c| c.metadata.as_ref())
                                    .and_then(|m| m.editor_viewport_line_count)
                                    .unwrap_or(10)
                                    .max(1);
                                zaroxi_core_editor_minimap::viewport_fraction(
                                    scroll_top,
                                    visible,
                                    editor_total_lines,
                                )
                            };
                            let mm_current_line = zaroxi_core_editor_minimap::line_fraction(
                                self.editor_buffer.caret_line(),
                                editor_total_lines,
                            );
                            let mm_selection =
                                self.editor_buffer.selection_line_range().map(|(sl, el)| {
                                    (
                                        zaroxi_core_editor_minimap::line_fraction(
                                            sl,
                                            editor_total_lines,
                                        ),
                                        zaroxi_core_editor_minimap::line_fraction(
                                            el,
                                            editor_total_lines,
                                        ),
                                    )
                                });
                            // ── Minimap rail rect for hit-testing ──
                            if self.tab_state.is_editor_active() {
                                self.minimap_hit_rect =
                                    Some(super::super::cockpit::minimap_rect(cockpit_editor_rect));
                            } else {
                                self.minimap_hit_rect = None;
                            }
                            let mut inputs = super::super::cockpit::CockpitInputs {
                                width: sw as f32,
                                height: sh as f32,
                                editor_rect: cockpit_editor_rect,
                                status_rect: cockpit_status_rect,
                                rail_rect: cockpit_rail_rect,
                                tab_strip_rect: cockpit_tab_strip_rect,
                                tabs: cockpit_tabs,
                                rail_items: {
                                    let glyphs: [(u32, &str); 7] = [
                                        (0xf07b, "Explorer"),
                                        (0xf002, "Search"),
                                        (0xe702, "Source Ctrl"),
                                        (0xf188, "Debug"),
                                        (0xf12e, "Extensions"),
                                        (0xf013, "Settings"),
                                        (0xf007, "Account"),
                                    ];
                                    let sel = self.rail_selected_index;
                                    let hov = self.rail_hovered_index;
                                    glyphs
                                        .iter()
                                        .enumerate()
                                        .map(|(idx, &(cp, label))| {
                                            zaroxi_interface_widgets::ActivityItem {
                                                index: idx,
                                                glyph: char::from_u32(cp).unwrap_or('?'),
                                                label: label.to_string(),
                                                selected: idx == sel,
                                                hovered: Some(idx) == hov,
                                                pressed: false,
                                            }
                                        })
                                        .collect()
                                },
                                rail_bg_color: rail_style_colors.0,
                                rail_item_active: rail_style_colors.1,
                                rail_accent_color: rail_style_colors.2,
                                rail_text_active: rail_style_colors.3,
                                rail_text_muted: rail_style_colors.4,
                                rail_divider_color: rail_style_colors.5,
                                line_height: lc::LINE_HEIGHT,
                                diff_hunks: diff_hunks_to_viewport(
                                    &self.cockpit_diff_hunks,
                                    self.large_file_mode,
                                    self.composition
                                        .as_ref()
                                        .and_then(|c| c.metadata.as_ref())
                                        .map(|m| m.editor_scroll_top_line)
                                        .unwrap_or(0),
                                    cockpit_editor_rect.3,
                                    lc::LINE_HEIGHT,
                                    &self.editor_visual_to_logical,
                                    self.editor_chars_per_row,
                                    self.editor_wrap_visual_offset,
                                    self.committed_active_file.as_deref(),
                                    self.editor_buffer.buffer_version,
                                    self.cockpit_diff_version,
                                ),
                                viewport: mm_viewport,
                                minimap_projection: self.cockpit_minimap.clone(),
                                minimap_current_line: Some(mm_current_line),
                                minimap_search_hits: Vec::new(),
                                minimap_selection: mm_selection,
                                status: instrument_status,
                                settings_panel: dp_settings.clone(),
                                settings: Some(self.settings.clone()),
                                settings_dropdown: self.settings_dropdown.clone(),
                                cached_popup: self.cached_settings_popup.clone(),
                                extensions_panel: dp_extensions,
                                placeholder_panel: dp_placeholder,
                                welcome_panel: matches!(
                                    self.tab_state.active(),
                                    super::super::destination::WorkbenchTabId::Welcome,
                                ),
                                file_editor_active: self.tab_state.is_editor_active(),
                                tab_scroll_offset: self.tab_state.scroll_offset,
                                ..Default::default()
                            };
                            let (scene, text) = super::super::cockpit::build_cockpit_frame(
                                &mut inputs,
                                &cockpit_tokens,
                            );
                            self.cached_settings_popup = inputs.cached_popup.clone();
                            core.set_cockpit_scene(Some(scene));
                            let text_runs = text.len();
                            core.set_cockpit_text(text);
                            if text_runs > 0 {
                                self.cockpit_text_active = true;
                            }
                            self.rail_item_hit_rects = {
                                let rx = cockpit_rail_rect.0;
                                let ry = cockpit_rail_rect.1;
                                let rw = cockpit_rail_rect.2;
                                let rh = cockpit_rail_rect.3;
                                let count = 7usize;
                                let slot_w = if count > 0 { rw / count as f32 } else { 0.0 };
                                let mut rects = Vec::new();
                                for i in 0..count {
                                    let sx = rx + i as f32 * slot_w;
                                    rects.push((sx, ry, slot_w, rh));
                                }
                                rects
                            };
                            // ── Settings row hit rects ────────────
                            self.settings_hit_rects = super::super::cockpit::compute_settings_hits(
                                &taffy::Layout {
                                    location: taffy::geometry::Point {
                                        x: cockpit_editor_rect.0,
                                        y: cockpit_editor_rect.1,
                                    },
                                    size: taffy::geometry::Size {
                                        width: cockpit_editor_rect.2.max(0.0),
                                        height: cockpit_editor_rect.3.max(0.0),
                                    },
                                    ..Default::default()
                                },
                                dp_settings.as_ref().map(|(s, _)| s.as_slice()).unwrap_or(&[]),
                                dp_settings.as_ref().map(|(_, sel)| *sel).unwrap_or(0),
                                &self.settings,
                                &self.settings_dropdown,
                            );
                        }
                        // ── Tab hit rects (recomputed every frame so resize
                        // always produces correct hit geometry) ─────
                        self.tab_hit_rects = {
                            let workbench_tabs = super::annotate_tabs_dirty(
                                self.tab_state.projected_tabs(
                                    &self.editor_group,
                                    self.committed_active_file.as_deref(),
                                ),
                                &dirty_doc_paths,
                            );
                            let layout_tabs: Vec<zaroxi_interface_widgets::CockpitTab> =
                                workbench_tabs
                                    .iter()
                                    .map(|t| zaroxi_interface_widgets::CockpitTab {
                                        title: t.title.clone(),
                                        active: t.active,
                                        closable: t.closable,
                                        kind: t.kind,
                                        is_preview: t.is_preview,
                                    })
                                    .collect();
                            self.tab_state.ensure_active_visible(
                                cockpit_tab_strip_rect.2,
                                zaroxi_interface_widgets::FILE_TAB_W,
                            );
                            let layout_res = zaroxi_interface_widgets::workbench_tab_layout(
                                cockpit_tab_strip_rect,
                                &layout_tabs,
                                self.tab_state.scroll_offset,
                            );
                            self.tab_arrow_left_rect = layout_res.arrow_left;
                            self.tab_arrow_right_rect = layout_res.arrow_right;
                            workbench_tabs
                                .iter()
                                .zip(layout_res.geometries)
                                .map(|(t, (rect, close))| {
                                    super::super::destination::WorkbenchTabHit {
                                        rect,
                                        close_rect: close,
                                        id: t.id.clone(),
                                    }
                                })
                                .collect()
                        };
                        self.cockpit_retained_bytes =
                            self.cockpit_diff_hunks.len().saturating_mul(32) + 1024;
                    }
                }
                // ── end cockpit build ───────────────────────────

                match core.render_to_window(surface_size, &render_layout, &render_blocks) {
                    Ok(perf) => {
                        let total_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
                        self.needs_render = false;
                        if !self.startup_first_paint_done {
                            self.startup_first_paint_done = true;
                            if zaroxi_core_telemetry::startup_trace_enabled() {
                                eprintln!(
                                    "MEM_STARTUP: after_first_frame rss={:.1}MB",
                                    zaroxi_core_telemetry::rss_mb()
                                );
                            }
                        }
                        if self.cockpit_text_active && !self.cockpit_rendered_once {
                            self.cockpit_rendered_once = true;
                            if startup_trace {
                                eprintln!(
                                    "ZAROXI_STARTUP_TRACE: frame={} phase=cockpit_first_rendered",
                                    frame_id
                                );
                            }
                        }
                        self.frame_scheduler.on_frame_presented(Instant::now());
                        // Staged first paint: the renderer budgeted its
                        // shaping and deferred some lines. Re-arm a redraw
                        // so the remaining lines shape over the next
                        // frame(s) instead of freezing this one.
                        if perf.shaping_pending > 0 {
                            self.needs_render = true;
                            self.frame_scheduler.mark_redraw_requested();
                            z.window().request_redraw();
                        }
                        if perf_on {
                            // app_update = everything on the CPU app path
                            // not separately attributed to layout/syntax
                            // or the render-side phases.
                            let app_update_ms = (total_ms
                                - layout_ms
                                - syntax_ms
                                - perf.text_shape_ms
                                - perf.text_prepare_ms
                                - perf.gpu_encode_ms
                                - perf.gpu_submit_present_ms)
                                .max(0.0);
                            eprintln!(
                                "ZAROXI_PERF_TRACE: frame={} total_ms={:.2} app_update_ms={:.2} layout_ms={:.2} syntax_ms={:.2} text_shape_ms={:.2} text_prepare_ms={:.2} gpu_encode_ms={:.2} gpu_submit_present_ms={:.2} blocks={} text_cmds={} glyphs={} visible_lines={} total_lines={}",
                                frame_id,
                                total_ms,
                                app_update_ms,
                                layout_ms,
                                syntax_ms,
                                perf.text_shape_ms,
                                perf.text_prepare_ms,
                                perf.gpu_encode_ms,
                                perf.gpu_submit_present_ms,
                                render_blocks.len(),
                                perf.text_cmd_count,
                                perf.glyph_count,
                                editor_visible_lines,
                                editor_total_lines,
                            );
                        }
                        if pipeline_trace_enabled() {
                            let app_update_ms = (total_ms
                                - layout_ms
                                - syntax_ms
                                - perf.text_shape_ms
                                - perf.text_prepare_ms
                                - perf.gpu_encode_ms
                                - perf.gpu_submit_present_ms)
                                .max(0.0);
                            // Residual app_update not attributed to the
                            // instrumented sub-phases (status gather, poll,
                            // scroll-sync, region copy). Should be ~0.
                            let misc_ms = (app_update_ms
                                - widget_ms
                                - block_build_ms
                                - enrich_ms
                                - commit_open_ms
                                - poll_parse_results_ms)
                                .max(0.0);
                            eprintln!(
                                "ZAROXI_PIPELINE_TRACE: frame={} widget_ms={:.2} block_build_ms={:.2} enrich_ms={:.2} content_prep_ms={:.2} layout_ms={:.2} commit_open_ms={:.2} poll_parse_results_ms={:.2} misc_ms={:.2}",
                                frame_id,
                                widget_ms,
                                block_build_ms,
                                enrich_ms,
                                syntax_ms,
                                layout_ms,
                                commit_open_ms,
                                poll_parse_results_ms,
                                misc_ms,
                            );
                        }
                        if render_trace_enabled() {
                            eprintln!("ZAROXI_RENDER_TRACE: render_result frame={} ok", frame_id);
                        }

                        // ── Startup sub-phase breakdown ─────────────
                        if startup_trace {
                            let app_update_ms = (total_ms
                                - layout_ms
                                - syntax_ms
                                - perf.text_shape_ms
                                - perf.text_prepare_ms
                                - perf.gpu_encode_ms
                                - perf.gpu_submit_present_ms)
                                .max(0.0);
                            let misc_ms = (app_update_ms
                                - widget_ms
                                - block_build_ms
                                - enrich_ms
                                - commit_open_ms
                                - poll_parse_results_ms)
                                .max(0.0);
                            eprintln!(
                                "ZAROXI_STARTUP_TRACE: frame={} phase=render_breakdown layout_ms={:.2} syntax_ms={:.2} text_shape_ms={:.2} text_prepare_ms={:.2} gpu_encode_ms={:.2} gpu_submit_present_ms={:.2} widget_ms={:.2} block_build_ms={:.2} enrich_ms={:.2} commit_open_ms={:.2} poll_parse_results_ms={:.2} render_core_create_ms={:.2} gpu_upload_bytes={} elements_reused={} elements_rebuilt={} misc_ms={:.2}",
                                frame_id,
                                layout_ms,
                                syntax_ms,
                                perf.text_shape_ms,
                                perf.text_prepare_ms,
                                perf.gpu_encode_ms,
                                perf.gpu_submit_present_ms,
                                widget_ms,
                                block_build_ms,
                                enrich_ms,
                                commit_open_ms,
                                poll_parse_results_ms,
                                render_core_create_ms,
                                perf.gpu_upload_bytes,
                                perf.elements_reused,
                                perf.elements_rebuilt,
                                misc_ms,
                            );
                        }

                        // ── Observability subsystems (per frame) ──
                        // 1) Drain AI inference traces (non-blocking) into
                        //    the ZAROXI_AI_TRACE stream.
                        if let Some(rx) = self.ai_trace_rx.as_mut() {
                            // Fold AI trace events into the live session
                            // state (still prints ZAROXI_AI_TRACE lines).
                            self.ai_session.drain_from(rx);
                        }
                        // 2) Frame-paced memory sample + pressure response.
                        //    Runs unconditionally (eviction is functional,
                        //    not just diagnostic); trace emission self-gates.
                        if self.mem_monitor.tick() {
                            let rss = zaroxi_core_telemetry::read_rss_bytes().unwrap_or(0);
                            let vsz = zaroxi_core_telemetry::read_vsz_bytes().unwrap_or(0);
                            let pressure = self.mem_monitor.evaluate(rss);
                            let (shape_cache_bytes, gpu_bytes) = core
                                .text_renderer()
                                .map(|tr| (tr.mem_shape_cache_bytes(), tr.mem_gpu_bytes()))
                                .unwrap_or((0, 0));
                            let shape_cache_entries = core
                                .text_renderer()
                                .map(|tr| tr.shape_cache_entries())
                                .unwrap_or(0);
                            let atlas_entries =
                                core.text_renderer().map(|tr| tr.atlas_entry_count()).unwrap_or(0);
                            let rope_bytes = self.editor_buffer.char_count() as u64;
                            // Best-effort active-buffer feed (multi-doc
                            // feeding is via the tracker API on open/close).
                            self.buffer_tracker.note_open("active", editor_total_lines);
                            self.buffer_tracker.set_active("active");
                            self.buffer_tracker.set_visible(["active"]);
                            let sample = zaroxi_core_telemetry::MemorySample {
                                rss_bytes: rss,
                                vsz_bytes: vsz,
                                shape_cache_bytes,
                                shape_cache_entries,
                                atlas_entries,
                                rope_bytes,
                                gpu_bytes,
                                open_docs: self.buffer_tracker.open_count(),
                                total_lines: self.buffer_tracker.total_lines(),
                                pressure,
                            };
                            sample.emit();
                            self.buffer_tracker.emit(frame_id);
                            self.last_mem_sample = Some(sample);
                            if let Some(tr) = core.text_renderer() {
                                use zaroxi_core_telemetry::MemoryPressureLevel as Pl;
                                match pressure {
                                    Pl::Critical => tr.flush_glyph_cache(),
                                    Pl::Elevated => {
                                        tr.evict_shaped_cold(512);
                                    }
                                    Pl::Normal => {}
                                }
                            }
                        }
                        // Cockpit overlay (vello widgets): build the
                        // WidgetTree scene from app state. This is now the
                        // DEFAULT status + overview owner — no longer gated
                        // behind ZAROXI_COCKPIT. It is suppressed only when
                        // the explicit legacy fallback
                        // (ZAROXI_LEGACY_SHELL_SURFACES=1) is requested, so
                        // exactly one owner is active at a time.
                        let do_cockpit = super::super::cockpit::cockpit_surfaces_active();
                        if do_cockpit {
                            // Capture StyleTokens-derived rail colors before
                            // the cockpit block shadows `tokens` (StyleTokens)
                            // with CockpitTokens.
                            let rail_style_colors = (
                                tokens.rail_background.to_array(),
                                tokens.rail_item_active.to_array(),
                                tokens.rail_item_active_accent.to_array(),
                                tokens.text_primary.to_array(),
                                tokens.text_muted.to_array(),
                                tokens.divider_subtle.to_array(),
                            );
                            let tokens = super::super::cockpit::cockpit_tokens(
                                self.theme_mode,
                                system_is_dark,
                            );
                            // Track retained cockpit size.
                            self.cockpit_retained_bytes =
                                self.cockpit_diff_hunks.len().saturating_mul(32) + 1024;
                            // Refresh git diff change markers when the buffer
                            // version advances (per edit / on open). Idempotent and
                            // already run before the fingerprint above, so this is a
                            // no-op on the common path; it guards the case where this
                            // section is reached without the earlier refresh.
                            refresh_cockpit_diff_hunks(
                                &self.editor_buffer,
                                self.large_file_mode,
                                self.committed_active_file.as_deref(),
                                &mut self.git_diff_provider,
                                &mut self.cockpit_diff_hunks,
                                &mut self.cockpit_diff_version,
                            );
                            // ── Typed instrument-panel status model ──
                            // Context + metadata come from the shared
                            // presenter (`cockpit_context`/`cockpit_meta`);
                            // health + AI bands are runtime telemetry.
                            let ai_band = {
                                use zaroxi_application_ai::view_model::AiPhase;
                                let used = self.ai_session.tokens_streamed as u32;
                                let mode = match self.ai_session.phase {
                                    AiPhase::Idle => zaroxi_interface_widgets::AiMode::Dormant,
                                    AiPhase::PromptBuilding
                                    | AiPhase::Requesting
                                    | AiPhase::Streaming => zaroxi_interface_widgets::AiMode::Live,
                                    AiPhase::Complete => {
                                        if used > 0 {
                                            zaroxi_interface_widgets::AiMode::Degraded
                                        } else {
                                            zaroxi_interface_widgets::AiMode::Dormant
                                        }
                                    }
                                };
                                zaroxi_interface_widgets::AiBand {
                                    mode,
                                    tokens_used: used,
                                    // No backend context-window total / model
                                    // name yet -> stays unknown, so the band
                                    // shows a truthful dot/readout, never an
                                    // invented arc or flickering "AI idle".
                                    tokens_total: 0,
                                    model: None,
                                    latency_ms: self
                                        .ai_session
                                        .first_token_ms
                                        .map(|ms| ms.round() as u32),
                                }
                            };
                            let health_band = zaroxi_interface_widgets::HealthBand {
                                fps: current_fps_estimate(),
                                mem_mb: self
                                    .last_mem_sample
                                    .as_ref()
                                    .map(|s| (s.rss_bytes / (1024 * 1024)) as u32),
                                // No live LSP-health telemetry yet -> a steady
                                // "healthy" dot (stable, no churn).
                                lsp: zaroxi_interface_widgets::LspStatus::Healthy,
                            };
                            // RTL readiness: detect a right-to-left script in
                            // the context leaf (file/symbol) so the band order
                            // + alignment mirror for Arabic/Hebrew.
                            let status_rtl = cockpit_context.leaf.chars().any(|c| {
                                matches!(c,
                                            '\u{0590}'..='\u{08FF}'
                                            | '\u{FB1D}'..='\u{FDFF}'
                                            | '\u{FE70}'..='\u{FEFF}')
                            });
                            let instrument_status = zaroxi_interface_widgets::InstrumentStatus {
                                context: cockpit_context.clone(),
                                meta: cockpit_meta.clone(),
                                health: health_band,
                                ai: ai_band,
                                rtl: status_rtl,
                            };

                            // Cheap fingerprint: skip cockpit rebuild when
                            // nothing material changed (same status model,
                            // same window size, same symbol/diff versions).
                            let fp = instrument_status_fingerprint(
                                &instrument_status,
                                (sw, sh),
                                self.cockpit_diff_version,
                            );
                            let fp_match =
                                self.cockpit_text_active && fp == self.cockpit_status_fingerprint;
                            self.cockpit_status_fingerprint = fp;
                            if fp_match {
                                // Cockpit unchanged — already built pre-render.
                                self.cockpit_retained_bytes =
                                    self.cockpit_diff_hunks.len().saturating_mul(32) + 1024;
                            } else {
                                let workbench_tabs = super::annotate_tabs_dirty(
                                    self.tab_state.projected_tabs(
                                        &self.editor_group,
                                        self.committed_active_file.as_deref(),
                                    ),
                                    &dirty_doc_paths,
                                );
                                let cockpit_tabs: Vec<zaroxi_interface_widgets::CockpitTab> =
                                    workbench_tabs
                                        .iter()
                                        .map(|t| zaroxi_interface_widgets::CockpitTab {
                                            title: t.title.clone(),
                                            active: t.active,
                                            closable: t.closable,
                                            kind: t.kind,
                                            is_preview: t.is_preview,
                                        })
                                        .collect();
                                let (dp_settings, dp_extensions, dp_placeholder) =
                                    super::super::destination::cockpit_panels_for(
                                        self.tab_state.active(),
                                    );
                                // ── Pre-compute minimap fields for CockpitInputs ──
                                let active_path_deref2 = self
                                    .committed_active_file
                                    .as_deref()
                                    .map(|s| s.strip_prefix("buf:").unwrap_or(s));
                                let mm_max_rows2 = (cockpit_editor_rect.3 as usize).clamp(64, 2000);
                                ensure_minimap_projection(
                                    &mut self.cockpit_minimap,
                                    &mut self.cockpit_minimap_key,
                                    active_path_deref2,
                                    &self.editor_buffer,
                                    &self.doc_buffers,
                                    mm_max_rows2,
                                );
                                let mm_viewport2 = {
                                    let scroll_top = self
                                        .composition
                                        .as_ref()
                                        .and_then(|c| c.metadata.as_ref())
                                        .map(|m| m.editor_scroll_top_line)
                                        .unwrap_or(0);
                                    let visible = self
                                        .composition
                                        .as_ref()
                                        .and_then(|c| c.metadata.as_ref())
                                        .and_then(|m| m.editor_viewport_line_count)
                                        .unwrap_or(10)
                                        .max(1);
                                    zaroxi_core_editor_minimap::viewport_fraction(
                                        scroll_top,
                                        visible,
                                        editor_total_lines,
                                    )
                                };
                                let mm_current_line2 = zaroxi_core_editor_minimap::line_fraction(
                                    self.editor_buffer.caret_line(),
                                    editor_total_lines,
                                );
                                let mm_selection2 =
                                    self.editor_buffer.selection_line_range().map(|(sl, el)| {
                                        (
                                            zaroxi_core_editor_minimap::line_fraction(
                                                sl,
                                                editor_total_lines,
                                            ),
                                            zaroxi_core_editor_minimap::line_fraction(
                                                el,
                                                editor_total_lines,
                                            ),
                                        )
                                    });
                                // ── Minimap rail rect for hit-testing ──
                                if self.tab_state.is_editor_active() {
                                    self.minimap_hit_rect = Some(
                                        super::super::cockpit::minimap_rect(cockpit_editor_rect),
                                    );
                                } else {
                                    self.minimap_hit_rect = None;
                                }
                                let mut inputs = super::super::cockpit::CockpitInputs {
                                    width: sw as f32,
                                    height: sh as f32,
                                    // Editor + status bounds from the shell
                                    // layout: the overview/minimap nests at the
                                    // editor's right edge (editor-owned), and the
                                    // status bar uses the real status strip rect.
                                    editor_rect: cockpit_editor_rect,
                                    status_rect: cockpit_status_rect,
                                    // Activity rail rect from the shell layout
                                    // (bottom of the left column, cockpit-owned).
                                    rail_rect: cockpit_rail_rect,
                                    tab_strip_rect: cockpit_tab_strip_rect,
                                    tabs: cockpit_tabs,
                                    rail_items: {
                                        let glyphs: [(u32, &str); 7] = [
                                            (0xf07b, "Explorer"),
                                            (0xf002, "Search"),
                                            (0xe702, "Source Ctrl"),
                                            (0xf188, "Debug"),
                                            (0xf12e, "Extensions"),
                                            (0xf013, "Settings"),
                                            (0xf007, "Account"),
                                        ];
                                        let sel = self.rail_selected_index;
                                        let hov = self.rail_hovered_index;
                                        glyphs
                                            .iter()
                                            .enumerate()
                                            .map(|(idx, &(cp, label))| {
                                                zaroxi_interface_widgets::ActivityItem {
                                                    index: idx,
                                                    glyph: char::from_u32(cp).unwrap_or('?'),
                                                    label: label.to_string(),
                                                    selected: idx == sel,
                                                    hovered: Some(idx) == hov,
                                                    pressed: false,
                                                }
                                            })
                                            .collect()
                                    },
                                    rail_bg_color: rail_style_colors.0,
                                    rail_item_active: rail_style_colors.1,
                                    rail_accent_color: rail_style_colors.2,
                                    rail_text_active: rail_style_colors.3,
                                    rail_text_muted: rail_style_colors.4,
                                    rail_divider_color: rail_style_colors.5,
                                    line_height: lc::LINE_HEIGHT,
                                    // Live git change markers (added/modified/
                                    // removed) for the active file.
                                    diff_hunks: diff_hunks_to_viewport(
                                        &self.cockpit_diff_hunks,
                                        self.large_file_mode,
                                        self.composition
                                            .as_ref()
                                            .and_then(|c| c.metadata.as_ref())
                                            .map(|m| m.editor_scroll_top_line)
                                            .unwrap_or(0),
                                        cockpit_editor_rect.3,
                                        lc::LINE_HEIGHT,
                                        &self.editor_visual_to_logical,
                                        self.editor_chars_per_row,
                                        self.editor_wrap_visual_offset,
                                        self.committed_active_file.as_deref(),
                                        self.editor_buffer.buffer_version,
                                        self.cockpit_diff_version,
                                    ),
                                    // Accurate viewport band + structure-first
                                    // minimap projection from live editor state.
                                    viewport: mm_viewport2,
                                    minimap_projection: self.cockpit_minimap.clone(),
                                    minimap_current_line: Some(mm_current_line2),
                                    minimap_search_hits: Vec::new(),
                                    minimap_selection: mm_selection2,
                                    // Typed instrument-panel status model (the
                                    // three bands), built from the shared context
                                    // presenter + runtime health/AI telemetry.
                                    status: instrument_status,
                                    // prediction_cells / ai_regions remain empty:
                                    // there is no edit-prediction subsystem yet.
                                    settings_panel: dp_settings.clone(),
                                    settings: Some(self.settings.clone()),
                                    extensions_panel: dp_extensions,
                                    placeholder_panel: dp_placeholder,
                                    file_editor_active: self.tab_state.is_editor_active(),
                                    tab_scroll_offset: self.tab_state.scroll_offset,
                                    ..Default::default()
                                };
                                let (scene, text) = super::super::cockpit::build_cockpit_frame(
                                    &mut inputs,
                                    &tokens,
                                );
                                self.cached_settings_popup = inputs.cached_popup.clone();
                                // Vector visuals via the vello overlay; text
                                // via the cosmic-text pass (both applied next
                                // frame inside RenderCore).
                                core.set_cockpit_scene(Some(scene));
                                let text_runs = text.len();
                                core.set_cockpit_text(text);
                                if text_runs > 0 {
                                    self.cockpit_text_active = true;
                                }
                                // Compute rail item hit rects for interaction.
                                // Horizontal layout: each item occupies an equal-width slot.
                                self.rail_item_hit_rects = {
                                    let rx = cockpit_rail_rect.0;
                                    let ry = cockpit_rail_rect.1;
                                    let rw = cockpit_rail_rect.2;
                                    let rh = cockpit_rail_rect.3;
                                    let count = 7usize;
                                    let slot_w = if count > 0 { rw / count as f32 } else { 0.0 };
                                    let mut rects = Vec::new();
                                    for i in 0..count {
                                        let sx = rx + i as f32 * slot_w;
                                        rects.push((sx, ry, slot_w, rh));
                                    }
                                    rects
                                };
                                // ── Settings row hit rects ────────────
                                self.settings_hit_rects =
                                    super::super::cockpit::compute_settings_hits(
                                        &taffy::Layout {
                                            location: taffy::geometry::Point {
                                                x: cockpit_editor_rect.0,
                                                y: cockpit_editor_rect.1,
                                            },
                                            size: taffy::geometry::Size {
                                                width: cockpit_editor_rect.2.max(0.0),
                                                height: cockpit_editor_rect.3.max(0.0),
                                            },
                                            ..Default::default()
                                        },
                                        dp_settings
                                            .as_ref()
                                            .map(|(s, _)| s.as_slice())
                                            .unwrap_or(&[]),
                                        dp_settings.as_ref().map(|(_, sel)| *sel).unwrap_or(0),
                                        &self.settings,
                                        &self.settings_dropdown,
                                    );
                                // Cockpit built pre-render; the trace was
                                // emitted there.
                                eprintln!(
                                    "ZAROXI_COCKPIT: cockpit frame {}x{} lines={} text_runs={}",
                                    sw, sh, editor_total_lines, text_runs
                                );
                                // One-time rail theme trace: prove the widget
                                // uses theme-crate tokens, not custom colors.
                                if std::env::var("ZAROXI_RAIL_TRACE").as_deref() == Ok("1")
                                    && self.rail_item_hit_rects.is_empty()
                                {
                                    eprintln!(
                                        "ZAROXI_RAIL_TRACE: theme_tokens rail_bg=minimap_bg accent={:?} accent_soft={:?} text_primary={:?} text_muted={:?} divider={:?}",
                                        tokens.accent,
                                        tokens.accent_soft,
                                        tokens.text_primary,
                                        tokens.text_muted,
                                        tokens.divider,
                                    );
                                }
                            } // end unchanged-skip else
                        }

                        record_frame_presented();
                        // Retained per-element UI-node trace: which
                        // shell elements rebuilt vs. reused this frame,
                        // cross-referenced with the renderer's own
                        // per-element draw-payload reuse + GPU upload.
                        self.ui_node_tracker.record_frame(
                            frame_id,
                            &render_blocks,
                            (sw, sh),
                            system_is_dark,
                            ui_flags,
                            editor_visible_lines,
                            Some(&perf),
                        );

                        // ── Open-burst settle state ──
                        // Clear settling once the viewport shaped fully
                        // (no deferred lines) or the burst cap is hit.
                        let open_was_settling = open_settling;
                        if self.open_settling {
                            self.open_burst_frames += 1;
                            if perf.shaping_pending == 0
                                || self.open_burst_frames >= OPEN_BURST_MAX_FRAMES
                            {
                                self.open_settling = false;
                            }
                        }
                        // This frame handled any in-flight resize.
                        let was_resizing = self.resize_pending;
                        self.resize_pending = false;
                        // The one-time first-screenful frame has now run;
                        // subsequent open frames use the progressive budget.
                        if open_first_screenful {
                            self.open_first_screenful_pending = false;
                        }

                        // ── Startup: first-paint probe ─────────────
                        if !self.startup_first_paint_done {
                            self.startup_first_paint_done = true;
                            self.startup_first_paint_at = Some(std::time::Instant::now());
                            if startup_trace {
                                let postpaint_ms =
                                    frame_start.elapsed().as_secs_f32() * 1000.0 - total_ms;
                                let first_paint_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
                                eprintln!(
                                    "ZAROXI_STARTUP_TRACE: frame={} phase=first_paint total_ms={:.2} postpaint_misc_ms={:.2}",
                                    frame_id,
                                    first_paint_ms,
                                    postpaint_ms.max(0.0),
                                );
                            }
                        } else if startup_trace
                            && frame_id == 1
                            && let Some(reason) = &self.startup_second_layout_reason
                        {
                            eprintln!(
                                "ZAROXI_STARTUP_TRACE: frame={} phase=second_layout reason={}",
                                frame_id, reason,
                            );
                        }

                        // ── Phase 11: atomic first-paint presentation ──
                        // The atomic frame shapes the whole visible
                        // screenful in one pass. Once it is complete this
                        // is the single coherent first paint of the new
                        // file (content + chrome already match), so mark
                        // the snapshot presented and stop forcing the
                        // large one-shot budget. If — pathologically — the
                        // screenful did not complete, keep it armed so the
                        // next frame stays atomic (never present partial).
                        let screenful_rows = editor_visible_lines.max(1);
                        let visible_ready =
                            perf.lines_considered.saturating_sub(perf.shaping_pending);
                        let screenful_complete =
                            perf.shaping_pending == 0 || visible_ready >= screenful_rows;
                        if open_atomic_first_paint {
                            if screenful_complete {
                                self.open_atomic_first_paint = false;
                                let chrome_synced = self
                                    .open_present
                                    .as_ref()
                                    .map(|p| {
                                        self.work_content
                                            .as_ref()
                                            .and_then(|w| w.active_file.as_deref())
                                            == p.path.as_deref()
                                    })
                                    .unwrap_or(false);
                                if let Some(present) = self.open_present.as_mut()
                                    && !present.presented
                                {
                                    let now = Instant::now();
                                    let ready_ms =
                                        (now - present.started_at).as_secs_f32() * 1000.0;
                                    present.snapshot_ready_at = Some(now);
                                    present.presented = true;
                                    present.first_viewport_lines =
                                        visible_ready.min(screenful_rows);
                                    if open_present_trace_enabled() {
                                        eprintln!(
                                            "ZAROXI_OPEN_PRESENT_TRACE: token={} stage=snapshot_ready time_to_snapshot_ready_ms={:.2} first_viewport_lines={} first_viewport_shaped_lines={} first_viewport_pending_lines={}",
                                            present.token,
                                            ready_ms,
                                            present.first_viewport_lines,
                                            present.first_viewport_lines,
                                            0,
                                        );
                                        eprintln!(
                                            "ZAROXI_OPEN_PRESENT_TRACE: token={} stage=presented time_to_snapshot_ready_ms={:.2} time_to_present_ms={:.2} first_viewport_lines={} first_viewport_shaped_lines={} first_viewport_pending_lines={} presented_atomically={} used_head_preview={} promoted_head_preview={} top_viewport_repaints_after_present={} chrome_synced_at_present={}",
                                            present.token,
                                            ready_ms,
                                            ready_ms,
                                            present.first_viewport_lines,
                                            present.first_viewport_lines,
                                            0,
                                            1,
                                            present.used_head_preview as u8,
                                            present.promoted_head_preview as u8,
                                            present.top_repaints_after_present,
                                            chrome_synced as u8,
                                        );
                                    }
                                }
                            } else {
                                // Defensive: re-arm so the next frame stays
                                // atomic until the screenful is complete.
                                self.needs_render = true;
                                self.frame_scheduler.mark_redraw_requested();
                                z.window().request_redraw();
                            }
                        } else if open_was_settling
                            && !was_resizing
                            && self
                                .open_present
                                .as_ref()
                                .map(|p| p.presented && !screenful_complete)
                                .unwrap_or(false)
                        {
                            // Already-presented open re-shaped visible rows
                            // without a resize/edit: a success-criterion
                            // violation (top-of-viewport repaint after the
                            // atomic present). Count it so it is observable.
                            if let Some(present) = self.open_present.as_mut() {
                                present.top_repaints_after_present =
                                    present.top_repaints_after_present.saturating_add(1);
                            }
                        }

                        // Per-frame snapshot view (open-presentation trace).
                        if open_present_trace_enabled()
                            && let Some(present) = self.open_present.as_ref()
                            && (open_was_settling || open_atomic_first_paint)
                        {
                            // Reused = considered minus (freshly shaped +
                            // deferred): cache-hit rows that did not re-shape.
                            let visible_top_reused = perf
                                .lines_considered
                                .saturating_sub(perf.lines_shaped)
                                .saturating_sub(perf.shaping_pending);
                            eprintln!(
                                "ZAROXI_OPEN_PRESENT_TRACE: frame={} token={} open_snapshot_active={} open_snapshot_pending_lines={} visible_top_reused={} visible_top_rebuilt={} atomic_first_paint={} presented={}",
                                frame_id,
                                present.token,
                                (!present.presented) as u8,
                                perf.shaping_pending,
                                visible_top_reused,
                                perf.lines_shaped,
                                open_atomic_first_paint as u8,
                                present.presented as u8,
                            );
                        }

                        if settle_trace_enabled() {
                            let open_complete = open_was_settling && perf.shaping_pending == 0;
                            eprintln!(
                                "ZAROXI_SETTLE_TRACE: frame={} open_active={} open_viewport_complete={} open_viewport_pending={} open_viewport_shaped={} open_prefetch_dropped={} invalidation_scope={} geometry={} commit_deferred_due_to_open={} commit_deferred_due_to_resize={}",
                                frame_id,
                                open_was_settling as u8,
                                open_complete as u8,
                                perf.shaping_pending,
                                perf.lines_shaped,
                                perf.shaping_pending,
                                ui_flags.summary(),
                                was_resizing as u8,
                                self.commit_deferred_open as u8,
                                self.commit_deferred_resize as u8,
                            );
                        }
                        // ── Phase 6: open viewport / first-screenful ──
                        // Per-open-frame view of how much of the VISIBLE
                        // screenful is shaped. `first_paint_mode=
                        // screenful_complete` once every visible row has
                        // glyphs; remaining `progressive_pending` rows are
                        // below-the-fold/overscan filled at the low budget.
                        if open_was_settling
                            && (settle_trace_enabled() || file_open_trace_enabled())
                        {
                            let total = perf.lines_considered;
                            let pending = perf.shaping_pending;
                            let ready = total.saturating_sub(pending);
                            let screenful_rows = editor_visible_lines.max(1);
                            let screenful_ready = ready.min(screenful_rows);
                            let complete_visible = ready >= screenful_rows;
                            let has_syntax = !self.large_file_mode && self.latest_spans.is_some();
                            let with_syntax = if has_syntax { screenful_ready } else { 0 };
                            let without_syntax = screenful_ready.saturating_sub(with_syntax);
                            let mode =
                                if complete_visible { "screenful_complete" } else { "partial" };
                            eprintln!(
                                "ZAROXI_OPEN_VIEWPORT_TRACE: token={} open_visible_rows_total={} open_visible_rows_ready={} ready={} open_visible_rows_pending={} pending={} open_first_screenful_rows={} open_first_screenful_ready={} first_screenful_ready={} open_first_screenful_ms={:.2} open_progressive_rows_pending={} progressive_pending={} open_progressive_budget_ms={:.1} open_first_paint_complete_visible={} open_first_paint_with_syntax_rows={} open_first_paint_without_syntax_rows={} open_priority_rows_ready={} open_deferred_shape_rows={} shape_ms={:.2} open_shape_budget_ms={:.1} budget_ms={:.1} open_burst_blocked={} first_paint_mode={} mode={}",
                                self.committed_open_token,
                                total,
                                ready,
                                ready,
                                pending,
                                pending,
                                screenful_rows,
                                screenful_ready,
                                screenful_ready,
                                perf.text_shape_ms,
                                pending,
                                pending,
                                open_progressive_budget_ms(),
                                complete_visible as u8,
                                with_syntax,
                                without_syntax,
                                screenful_ready,
                                pending,
                                perf.text_shape_ms,
                                open_budget_ms,
                                open_budget_ms,
                                (pending > 0) as u8,
                                mode,
                                mode,
                            );
                        }
                        if !self.first_render_shown {
                            let legacy = super::super::cockpit::legacy_shell_surfaces();
                            let cockpit_ready = self.cockpit_rendered_once;
                            let geometry_stable = !self.resize_pending;
                            let ready_to_show = (cockpit_ready || legacy) && geometry_stable;
                            if ready_to_show {
                                z.window().set_visible(true);
                                self.first_render_shown = true;
                                self.startup_geometry_final = Some((sw, sh));
                                self.startup_first_visible_layout_stable = true;
                                if startup_trace {
                                    eprintln!(
                                        "ZAROXI_STARTUP_TRACE: frame={} phase=first_visible_layout_stable initial_geom={:?} final_geom={:?} changed_reason={} cockpit_ready={} legacy={} geometry_stable={}",
                                        frame_id,
                                        self.startup_geometry_initial,
                                        self.startup_geometry_final,
                                        self.startup_geometry_changed_reason
                                            .as_deref()
                                            .unwrap_or("none"),
                                        cockpit_ready,
                                        legacy,
                                        geometry_stable,
                                    );
                                }
                                eprintln!(
                                    "GuiApp: first full-renderer frame; window visible (cockpit_ready={} legacy={} geom_stable={})",
                                    cockpit_ready, legacy, geometry_stable,
                                );
                                // ── Post-settle cache trim ─────────
                                if !self.startup_settle_trimmed {
                                    self.startup_settle_trimmed = true;
                                    if let Some(ref core) = self.render_core
                                        && let Some(tr) = core.text_renderer()
                                    {
                                        let before_entries = tr.mem_shape_cache_bytes();
                                        tr.evict_shaped_cold(256);
                                        let after_entries = tr.mem_shape_cache_bytes();
                                        if std::env::var("ZAROXI_MEM_TRACE").as_deref() == Ok("1") {
                                            eprintln!(
                                                "ZAROXI_MEM_TRACE: frame={} phase=post_settle_trim shape_cache_before_kb={} shape_cache_after_kb={}",
                                                frame_id,
                                                before_entries as usize / 1024,
                                                after_entries as usize / 1024,
                                            );
                                        }
                                    }
                                    let before_syntax = self.editor_retained_bytes;
                                    let visible_start = self
                                        .composition
                                        .as_ref()
                                        .and_then(|c| c.metadata.as_ref())
                                        .map(|m| m.editor_scroll_top_line)
                                        .unwrap_or(0);
                                    let visible_end = visible_start + editor_visible_lines + 40;
                                    self.line_syntax_cache.retain(|&(line, _), _| {
                                        line >= visible_start && line < visible_end
                                    });
                                    self.cached_line_hashes.truncate(visible_end);
                                    self.editor_retained_bytes = self
                                        .line_syntax_cache
                                        .values()
                                        .map(|v| v.iter().map(|(s, _)| s.len()).sum::<usize>())
                                        .sum::<usize>()
                                        + self.cached_line_hashes.len() * 8
                                        + self
                                            .latest_spans
                                            .as_ref()
                                            .map(|s| s.len() * 32)
                                            .unwrap_or(0);
                                    if std::env::var("ZAROXI_MEM_TRACE").as_deref() == Ok("1") {
                                        eprintln!(
                                            "ZAROXI_MEM_TRACE: frame={} phase=post_settle_trim syntax_before_kb={} syntax_after_kb={} cached_hashes={}",
                                            frame_id,
                                            before_syntax / 1024,
                                            self.editor_retained_bytes / 1024,
                                            self.cached_line_hashes.len(),
                                        );
                                    }
                                }
                            } else if startup_trace {
                                eprintln!(
                                    "ZAROXI_STARTUP_TRACE: frame={} phase=first_visible_deferred cockpit_ready={} legacy={} geometry_stable={} resize_pending={}",
                                    frame_id,
                                    cockpit_ready,
                                    legacy,
                                    geometry_stable,
                                    self.resize_pending,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        if render_trace_enabled() {
                            eprintln!(
                                "ZAROXI_RENDER_TRACE: render_result frame={} err={:?}",
                                frame_id, e
                            );
                        }
                        // Retry on the next opportunity: stay dirty and
                        // re-arm a redraw, keeping scheduler state in sync.
                        self.needs_render = true;
                        self.frame_scheduler.mark_redraw_requested();
                        z.window().request_redraw();
                    }
                }
            }

            if std::env::var("ZAROXI_DEBUG_RENDER").as_deref() == Ok("1") {
                eprintln!("...");
            }
        }
    }
}
