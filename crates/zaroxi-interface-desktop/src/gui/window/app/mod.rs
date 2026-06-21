/*!
GuiApp implementation and winit ApplicationHandler lifecycle methods.

Phase 57: slimmed to a thin winit-to-engine bridge; widget interaction
(hit-testing, hover, press, scrollbar drag, focus) now lives in
`zaroxi_core_engine_ui::WidgetInteractionModel`.

Phase 58: added keyboard focus traversal (Tab/Shift+Tab/Enter/Escape) and
`on_widget_activated` callback.

Phase 59: built-in `dispatch_activation` method that routes WidgetId to
DesktopComposition actions (set active buffer, window controls, etc.).
The callback remains as an override capability.

Editor Phase 1: extracted editor shell layout/rendering into
`editor_shell` module. `GuiApp` now delegates region layout to
`ShellLayoutController` (Taffy-based) and uses `EditorViewport`
for strict clipping boundaries.

Phase 60 (Architecture Refactor): `app.rs` split into focused sub-modules
so that `mod.rs` only contains the struct, thin winit-lifecycle wiring,
and high-level delegation.  Detail lives in:
- `activation.rs`         — widget activation routing & explorer CTA
- `input.rs`              — keyboard interpretation & mouse-wheel normalisation
- `editor_interaction.rs` — cursor projection, selection & hit-testing
- `render_state.rs`       — content hashing, editor-data caching
- `debug.rs`              — shared debug/trace helpers
*/

mod activation;
pub(crate) mod background_parse;
pub(crate) mod debug;
mod editor_interaction;
mod input;
mod render_schedule;
mod render_state;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub use render_schedule::{FrameScheduler, InvalidationFlags};

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::ControlFlow,
    window::WindowAttributes,
};

static GUI_FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Cadence for polling background work (parse results, folder picker) while the
/// UI is otherwise idle. Relaxed enough to avoid pinning a CPU core, tight
/// enough that results land promptly.
const BACKGROUND_POLL_INTERVAL: Duration = Duration::from_millis(8);

fn render_trace_enabled() -> bool {
    std::env::var("ZAROXI_RENDER_TRACE").as_deref() == Ok("1")
}

fn frame_trace_enabled() -> bool {
    std::env::var("ZAROXI_FRAME_TRACE").as_deref() == Ok("1")
}

fn scroll_trace_enabled() -> bool {
    std::env::var("ZAROXI_SCROLL_TRACE").as_deref() == Ok("1")
}

/// Whether `ZAROXI_PERF_TRACE=1` is set. Drives the consolidated per-frame
/// `ZAROXI_PERF_TRACE` line and the event-scoped (file/workspace open, edit,
/// cursor move, scroll) timing lines.
pub(crate) fn perf_trace_enabled() -> bool {
    std::env::var("ZAROXI_PERF_TRACE").as_deref() == Ok("1")
}

/// Emit an event-scoped perf line (no-op unless `ZAROXI_PERF_TRACE=1`).
/// `detail` is appended verbatim (e.g. `lines=120 bytes=4096`).
pub(crate) fn perf_event(label: &str, start: std::time::Instant, detail: &str) {
    if !perf_trace_enabled() {
        return;
    }
    let ms = start.elapsed().as_secs_f32() * 1000.0;
    eprintln!("ZAROXI_PERF_TRACE: event={} ms={:.2} {}", label, ms, detail);
}

fn record_frame_presented() {
    if std::env::var("ZAROXI_FPS_TRACE").as_deref() != Ok("1") {
        return;
    }
    let now = std::time::Instant::now();
    use std::sync::Mutex;
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

use crate::DesktopComposition;
use crate::folder_picker::{DynFolderPicker, PickerOutcome};
use crate::gui::window::editor_buf::EditorBufferState;
use crate::gui::window::editor_shell::{EditorViewport, ShellLayoutController};
use crate::gui::window::explorer_panel::ExplorerPanelActions;
use crate::gui::{ShellFrame, ShellWorkContent};
use zaroxi_application_workspace::ports::{SessionId, WorkspaceService, WorkspaceView};
use zaroxi_core_engine_ui::WidgetId;
use zaroxi_core_engine_ui::layout_constants as lc;
use zaroxi_core_platform_syntax::highlight::HighlightSpan;
use zaroxi_core_platform_syntax::language::LanguageId;
use zaroxi_core_platform_syntax::parser::ParserPool;
use zaroxi_kernel_types::Id;

pub type WidgetActivationHandler = Box<dyn FnMut(&WidgetId) -> Option<ShellWorkContent>>;

pub struct GuiApp {
    pub window_attributes: WindowAttributes,
    pub title: String,
    pub maybe_window: Option<zaroxi_core_engine_window::ZaroxiWindow>,
    pub shell: ShellFrame,
    pub work_content: Option<ShellWorkContent>,
    pub requested_initial_frame: bool,
    pub already_logged_existing: bool,
    pub first_render_shown: bool,
    pub widget_tree: Option<zaroxi_core_engine_ui::ShellWidgetTree>,
    pub interaction: zaroxi_core_engine_ui::WidgetInteractionModel,
    pub editor_buffer: EditorBufferState,
    pub theme_mode: zaroxi_interface_theme::theme::ZaroxiTheme,
    pub shift_held: bool,
    pub ctrl_held: bool,
    pub on_widget_activated: Option<WidgetActivationHandler>,
    pub composition: Option<DesktopComposition>,
    pub workspace_view: Option<Arc<dyn WorkspaceView>>,
    pub workspace_service: Option<Arc<dyn WorkspaceService>>,
    pub session_id: Option<SessionId>,
    pub workspace_id: Option<Id>,
    pub folder_picker: Option<DynFolderPicker>,
    pub explorer_actions: Option<ExplorerPanelActions>,
    pub explorer_button_rect: Option<(f32, f32, f32, f32)>,
    pub parser_pool: Arc<ParserPool>,
    pub cached_editor_data: Option<crate::gui::window::editor::EditorContentData>,
    pub cached_editor_lines_hash: u64,
    /// Spans version the cached editor data was shaped with. Part of the editor
    /// cache key so stored plain-text content is never reused after highlight
    /// spans arrive (see `render_state::prepare_editor_data`).
    pub cached_editor_spans_version: u64,
    pub layout_controller: ShellLayoutController,
    pub editor_viewport: Option<EditorViewport>,
    pub needs_render: bool,
    pub last_explorer_ids: Vec<String>,
    pub last_render_size: (u32, u32),
    pub pending_scroll_frac: f32,
    pub picker_in_flight: bool,
    pub pending_picker_rx: Option<mpsc::Receiver<PickerOutcome>>,
    pub last_widget_tree_size: (u32, u32),
    pub last_widget_tree_content: Option<ShellWorkContent>,
    pub render_core: Option<zaroxi_core_engine_render::renderer::core::RenderCore>,
    /// Per-line syntax-colored span cache keyed by (line_index, content_fnv_hash).
    /// Avoids recomputing spans for lines whose content didn't change.
    pub line_syntax_cache: HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    /// Per-line raw-content fnv hash from the last cache build.
    pub cached_line_hashes: Vec<u64>,
    /// Whether the current file exceeds large-file thresholds (>1000 lines or >100KB).
    /// When true, full-document syntax highlighting is disabled to prevent O(file_size)
    /// parse perf stalls; only viewport-visible lines are rendered.
    pub large_file_mode: bool,
    /// Detected language for the currently open buffer.  Source of truth is
    /// the active file path (`LanguageId::from_path`), assigned in
    /// `set_work_content`.  Defaults to `PlainText` when no path is known.
    pub current_language: LanguageId,
    /// Latest accepted full-document highlight spans for the current buffer,
    /// produced by the background parse worker.  Cleared when the buffer
    /// changes so stale spans are never reused across files.
    pub latest_spans: Option<Vec<HighlightSpan>>,
    /// Buffer version the `latest_spans` correspond to.  Used to detect when a
    /// fresh parse result has arrived and to avoid re-applying the same result.
    pub latest_spans_version: u64,
    /// Background parse worker for off-thread tree-sitter parsing.
    pub parse_worker: Option<background_parse::BackgroundParseWorker>,
    /// `editor_buffer.buffer_version` captured when the active file was last
    /// loaded (or saved). The document is considered modified when the live
    /// buffer version diverges from this baseline.
    pub saved_buffer_version: u64,
    /// Redraw coalescing + frame pacing. `needs_render` is the dirty flag; this
    /// owns the pacing/cadence and outstanding-redraw bookkeeping.
    pub frame_scheduler: FrameScheduler,
}

// ── Large-file thresholds ──

/// Maximum line count before entering large-file mode (skips full-document
/// syntax highlighting to avoid tree-sitter O(n) parse stalls per keystroke).
const LARGE_FILE_LINE_THRESHOLD: usize = 1000;

/// Maximum byte count before entering large-file mode.
const LARGE_FILE_BYTE_THRESHOLD: usize = 100_000;

/// Maximum line count before the background parser receives empty/plain-text
/// snapshots instead of full-file text.  Above this threshold full-tree-sitter
/// parsing is too slow to be useful and we degrade to viewport-only plain text.
pub(crate) const HUGE_FILE_LINE_THRESHOLD: usize = 50_000;

impl GuiApp {
    pub fn editor_cursor_line(&self) -> usize {
        self.editor_buffer.caret_line()
    }

    pub fn editor_cursor_col(&self) -> usize {
        self.editor_buffer.caret_col()
    }

    pub fn editor_selection_range(&self) -> Option<(usize, usize, usize, usize)> {
        self.editor_buffer.selection_range()
    }

    pub fn editor_selection_active(&self) -> bool {
        self.editor_buffer.selection_active
    }

    /// Whether the active document has unsaved edits since it was loaded/saved.
    fn document_modified(&self) -> bool {
        self.editor_buffer.buffer_version != self.saved_buffer_version
    }

    /// Compact selection summary for the status bar, when a selection is active.
    fn status_selection(&self) -> Option<super::status_bar::SelectionInfo> {
        let (start_line, _, end_line, _) = self.editor_selection_range()?;
        let chars = self.editor_buffer.selected_text().map(|t| t.chars().count()).unwrap_or(0);
        if chars == 0 {
            return None;
        }
        Some(super::status_bar::SelectionInfo {
            chars,
            lines: end_line.saturating_sub(start_line) + 1,
        })
    }

    /// Diagnostics counts for the status bar, only when a provider is ready.
    fn status_diagnostics(&self) -> Option<super::status_bar::DiagnosticCounts> {
        let snapshot = self.composition.as_ref()?.latest_diagnostics_snapshot()?;
        if snapshot.provider != crate::diagnostics::ProviderState::Ready {
            return None;
        }
        Some(super::status_bar::DiagnosticCounts {
            errors: snapshot.errors,
            warnings: snapshot.warnings,
            infos: snapshot.infos,
            hints: snapshot.hints,
        })
    }

    /// Return the monospace character advance from the font system,
    /// falling back to the layout-constant stub when the renderer isn't available.
    pub fn monospace_advance_x(&self) -> Option<f32> {
        self.render_core
            .as_ref()
            .and_then(|core| core.text_renderer().and_then(|tr| tr.monospace_advance_x()))
    }

    /// Set the work_content and sync the editor buffer from its content.
    fn set_work_content(&mut self, wc: ShellWorkContent) {
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
        let prev_active = self.work_content.as_ref().and_then(|w| w.active_file.clone());
        let buffer_changed = prev_active.as_deref() != wc.active_file.as_deref()
            || detected_language != self.current_language;
        self.current_language = detected_language;
        if buffer_changed {
            self.latest_spans = None;
            self.latest_spans_version = 0;
            if let Some(ref mut worker) = self.parse_worker {
                worker.clear_result();
            }
        }

        if let Some(ref body) = wc.editor_body {
            self.editor_buffer.populate_from_lines(&body.lines, body.cursor_line, body.cursor_col);
            // The freshly loaded content is the saved baseline for dirty tracking.
            self.saved_buffer_version = self.editor_buffer.buffer_version;
            // Detect large-file mode from the incoming content view.
            self.large_file_mode = Self::is_large_file(&body.lines);
            if self.large_file_mode
                && std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1")
            {
                eprintln!(
                    "ZAROXI_DEBUG_LARGE_FILE: large_file_mode ON lines={} bytes={}",
                    body.lines.len(),
                    body.lines.iter().map(|l| l.len()).sum::<usize>(),
                );
            }

            // Spawn background parse worker for off-thread syntax highlighting.
            if self.parse_worker.is_none() {
                self.parse_worker = Some(background_parse::BackgroundParseWorker::spawn(
                    Arc::clone(&self.parser_pool),
                ));
            }
            // Schedule initial background syntax parse.
            // For large/huge files (>=1000 lines or >=100KB), skip full-document
            // tree-sitter parsing entirely — viewport-only plain-text fallback
            // is already active in the render path via `large_file_mode`.
            if self.large_file_mode {
                if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_DEBUG_LARGE_FILE: set_work_content SKIPPED bg parse (large_file_mode lines={})",
                        self.editor_buffer.line_count(),
                    );
                }
            } else if let Some(ref mut worker) = self.parse_worker {
                // Synchronous first-paint highlight: compute spans for the
                // (small) file on the main thread so the very first frame is
                // already styled — no plain-text flash on open. The compiled
                // query is cached process-wide, so this is cheap on repeat
                // opens. The background worker is still scheduled for
                // resilience; its same-version result is deduplicated by
                // `poll_parse_results`.
                let text = self.editor_buffer.to_string();
                let version = self.editor_buffer.buffer_version;
                let language = self.current_language;
                worker.schedule_parse(background_parse::BufferSnapshot {
                    version,
                    text: text.clone(),
                    language,
                });

                let spans = background_parse::compute_spans(&self.parser_pool, language, &text);
                if !spans.is_empty() {
                    self.latest_spans = Some(spans);
                    self.latest_spans_version = version;
                    self.cached_editor_lines_hash = 0;
                    self.line_syntax_cache.clear();
                }
            }
        }
        self.work_content = Some(wc);
        perf_event(
            "open_document",
            ev_start,
            &format!(
                "lines={} large_file={} lang={:?}",
                self.editor_buffer.line_count(),
                self.large_file_mode,
                self.current_language,
            ),
        );
    }

    /// Check whether a file exceeds large-file thresholds and should
    /// enter reduced-feature mode to avoid perf stalls and crashes.
    fn is_large_file(lines: &[String]) -> bool {
        let line_count = lines.len();
        if line_count > LARGE_FILE_LINE_THRESHOLD {
            return true;
        }
        let byte_count: usize = lines.iter().map(|l| l.len() + 1).sum();
        byte_count > LARGE_FILE_BYTE_THRESHOLD
    }

    /// Whether the current file is huge enough that full-document tree-sitter
    /// parsing should be skipped entirely in favour of plain-text fallback.
    fn is_huge_file(&self) -> bool {
        let total = self.editor_buffer.line_count();
        total > HUGE_FILE_LINE_THRESHOLD
    }

    /// Recompute syntax highlighting after an edit.
    ///
    /// Called after every edit. For non-large files this performs a
    /// **synchronous** re-highlight of the current buffer so `latest_spans`
    /// stays in lockstep with the edited text. This is what keeps highlighting
    /// visually stable while typing: previously the async worker left the old
    /// spans (with pre-edit byte offsets) applied to the new text for a few
    /// frames, mis-colouring everything after the edit point until the parse
    /// landed — the visible "syntax churn". The compiled query is cached, so a
    /// full reparse of a small file is cheap.
    ///
    /// Size-aware policy:
    /// - Small files (<1000 lines, <100KB): synchronous re-highlight.
    /// - Large/huge files: skip parsing entirely; plain-text fallback.
    pub(crate) fn schedule_background_parse(&mut self) {
        if self.large_file_mode {
            if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DEBUG_LARGE_FILE: schedule_bg_parse SKIPPED (large_file_mode lines={})",
                    self.editor_buffer.line_count(),
                );
            }
            return;
        }

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

        // Synchronous re-highlight keeps spans aligned with the current text.
        // Only overwrite the stored highlights when we have a result (a
        // supported language with non-empty text), so an unsupported/empty
        // parse never flashes existing colours away.
        let spans = background_parse::compute_spans(&self.parser_pool, language, &text);
        if !spans.is_empty() {
            self.latest_spans = Some(spans);
            self.latest_spans_version = version;
            // The line hash changes on every edit, so the editor cache already
            // rebuilds; clearing keeps the per-line syntax cache consistent.
            self.cached_editor_lines_hash = 0;
            self.line_syntax_cache.clear();
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
        let accepted = if let Some(ref mut worker) = self.parse_worker {
            let current = worker.latest_version();
            let got = match worker.poll_result() {
                Some(result) if result.version == current => {
                    Some((result.spans.clone(), result.version))
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

        if let Some((spans, version)) = accepted {
            // Only apply strictly-newer results. Synchronous re-highlighting on
            // edit advances `latest_spans_version` to the current buffer
            // version, so any stale async result (an older version still in the
            // worker channel) is dropped silently and can never overwrite the
            // current highlights.
            if version > self.latest_spans_version {
                if std::env::var("ZAROXI_DEBUG_PARSE_PIPELINE").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_DEBUG_PARSE_PIPELINE: spans_stored v={} span_count={} lang={:?}",
                        version,
                        spans.len(),
                        self.current_language,
                    );
                }
                self.latest_spans = Some(spans);
                self.latest_spans_version = version;
                // Force the editor shaping caches to rebuild with the new spans.
                self.cached_editor_lines_hash = 0;
                self.line_syntax_cache.clear();
                self.invalidate(InvalidationFlags::syntax());
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

impl GuiApp {
    pub fn dispatch_activation(&mut self, id: &WidgetId) -> Option<ShellWorkContent> {
        activation::dispatch_activation(self, id)
    }

    fn request_render(&mut self) {
        self.invalidate(InvalidationFlags::content());
    }

    /// Mark the UI dirty for `reason` and schedule a (possibly paced) redraw.
    /// Multiple invalidations before the next frame are coalesced into a single
    /// presented frame.
    pub(crate) fn invalidate(&mut self, reason: InvalidationFlags) {
        if render_trace_enabled() {
            let pending = GUI_FRAME_COUNTER.load(Ordering::Relaxed) + 1;
            eprintln!(
                "ZAROXI_RENDER_TRACE: invalidate reason={} frame_pending={} already_dirty={}",
                reason.summary(),
                pending,
                self.needs_render
            );
        }
        self.frame_scheduler.note_reason(reason);
        self.needs_render = true;
        self.schedule_redraw();
    }

    /// Issue a winit redraw now if the frame budget has elapsed and none is
    /// already outstanding; otherwise leave it for `about_to_wait`, which parks
    /// the loop with `WaitUntil(deadline)` and issues the redraw once the budget
    /// elapses. This is what coalesces event bursts into one paced frame.
    fn schedule_redraw(&mut self) {
        if self.frame_scheduler.redraw_outstanding() {
            return;
        }
        if !self.frame_scheduler.budget_elapsed(Instant::now()) {
            return;
        }
        if let Some(z) = self.maybe_window.as_ref() {
            z.window().request_redraw();
            self.frame_scheduler.mark_redraw_requested();
        }
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
                            self.set_work_content(wc);
                            self.last_widget_tree_content = None;
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
                    self.set_work_content(wc);
                    self.last_widget_tree_content = None;
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
                    self.set_work_content(wc);
                    self.last_widget_tree_content = None;
                    self.request_render();
                }
            }
        }
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
                        .or_else(|| activation::dispatch_activation(self, id));

                    if let Some(ref wc) = content {
                        let changed = self.work_content.as_ref().is_none_or(|old| {
                            old.explorer_items != wc.explorer_items
                                || old.active_file != wc.active_file
                                || old.editor_tabs != wc.editor_tabs
                                || old.editor_body.as_ref().map(|b| &b.lines)
                                    != wc.editor_body.as_ref().map(|b| &b.lines)
                        });
                        if changed {
                            self.set_work_content(wc.clone());
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

impl winit::application::ApplicationHandler for GuiApp {
    fn new_events(&mut self, active_loop: &winit::event_loop::ActiveEventLoop, cause: StartCause) {
        if self.maybe_window.is_none() && matches!(cause, StartCause::Init) {
            debug::gui_debug("GuiApp: attempting to create window (StartCause::Init)");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    debug::gui_debug_fmt!("GuiApp: created engine window id={:?}", wid);
                    zaroxi_w.window().set_title(&self.title);
                    zaroxi_w.window().set_outer_position(PhysicalPosition::new(100, 100));
                    self.maybe_window = Some(zaroxi_w);

                    self.request_render();
                    active_loop.set_control_flow(ControlFlow::Wait);
                    debug::gui_debug("GuiApp: window created (hidden); initial redraw requested");
                }
                Err(e) => {
                    eprintln!("GuiApp: failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else if self.maybe_window.is_some() {
            if !self.already_logged_existing {
                debug::gui_debug("GuiApp: new_events called but window already created");
                self.already_logged_existing = true;
            }
        } else {
            debug::gui_debug_fmt!("GuiApp: new_events called with cause={:?} (no creation)", cause);
        }
    }

    fn resumed(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        if self.maybe_window.is_none() {
            debug::gui_debug("GuiApp: resumed -> attempting to create window");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    debug::gui_debug_fmt!("GuiApp: created engine window on resumed id={:?}", wid);
                    self.maybe_window = Some(zaroxi_w);

                    self.request_render();
                    debug::gui_debug(
                        "GuiApp: window created on resumed (hidden); initial redraw requested",
                    );
                }
                Err(e) => {
                    eprintln!("GuiApp: resumed failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else {
            debug::gui_debug("GuiApp: resumed called but window already created");
        }
    }

    fn about_to_wait(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        self.process_picker_result();

        // Apply any completed background parse result; this may invalidate the
        // UI so freshly parsed highlight spans become visible.
        self.poll_parse_results();

        if self.requested_initial_frame {
            self.invalidate(InvalidationFlags::content());
            self.requested_initial_frame = false;
        }

        let now = Instant::now();

        if self.needs_render || self.interaction.scrollbar_drag_active() {
            // A frame is pending. Honour the pacing budget: issue the redraw now
            // if the budget has elapsed, otherwise park until the deadline. No
            // busy spinning — the loop sleeps until there is real work.
            if self.frame_scheduler.budget_elapsed(now) {
                self.schedule_redraw();
                active_loop.set_control_flow(ControlFlow::Wait);
            } else {
                active_loop.set_control_flow(ControlFlow::WaitUntil(
                    self.frame_scheduler.next_deadline(now),
                ));
            }
        } else if self.picker_in_flight || self.parse_result_pending() {
            // Background work is in flight; poll on a relaxed cadence so the
            // result is applied promptly without pinning a CPU core.
            active_loop.set_control_flow(ControlFlow::WaitUntil(now + BACKGROUND_POLL_INTERVAL));
        } else {
            active_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn window_event(
        &mut self,
        active_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.process_picker_result();

        // ── Gated focus / pointer-enter diagnostics (ZAROXI_LIVE_DIAG=1) ──
        if std::env::var("ZAROXI_LIVE_DIAG").as_deref() == Ok("1") {
            match &event {
                WindowEvent::Focused(f) => {
                    eprintln!("ZAROXI_LIVE: window Focused({})", f);
                }
                WindowEvent::CursorEntered { .. } => {
                    eprintln!("ZAROXI_LIVE: CursorEntered");
                }
                WindowEvent::CursorLeft { .. } => {
                    eprintln!("ZAROXI_LIVE: CursorLeft");
                }
                _ => {}
            }
        }

        // ── Gated full event trace (ZAROXI_DEBUG_CLICK=1) ──
        debug::click_trace_fmt!("ZAROXI_EVT: {}", debug::event_label(&event));

        match event {
            WindowEvent::CloseRequested => {
                active_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(z) = self.maybe_window.as_mut() {
                    z.update_size(size.width, size.height);
                    debug::gui_debug_fmt!("GuiApp: Resized -> {size:?}, invalidating");
                }
                self.invalidate(InvalidationFlags::resize());
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                debug::gui_debug("GuiApp: ScaleFactorChanged -> invalidating");
                self.invalidate(InvalidationFlags::resize());
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.interaction.set_cursor_pos(position.x as f32, position.y as f32);
                debug::click_trace_fmt!(
                    "ZAROXI_CLICK: CursorMoved x={:.1} y={:.1} widget_tree={}",
                    position.x,
                    position.y,
                    self.widget_tree.is_some()
                );

                if let Some(ref mut tree) = self.widget_tree {
                    let actions = self.interaction.on_pointer_moved(
                        tree,
                        position.x as f32,
                        position.y as f32,
                    );
                    self.handle_actions(actions);

                    // Drag-selection: extend selection range while mouse is held
                    if self.editor_selection_active() {
                        editor_interaction::update_drag_selection(self, position);
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(ref mut tree) = self.widget_tree {
                    let actions = self.interaction.on_pointer_leave(tree);
                    self.handle_actions(actions);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let ev_start = std::time::Instant::now();
                input::process_mouse_wheel(self, &delta);
                perf_event("scroll", ev_start, "");
            }
            WindowEvent::MouseInput { state, button, .. } if button == MouseButton::Left => {
                let (x, y) = match self.interaction.cursor_pos_f32() {
                    Some(pos) => pos,
                    None => {
                        debug::click_trace(
                            "ZAROXI_CLICK: MouseInput — cursor_pos is None, skipping",
                        );
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
                            let id =
                                zaroxi_core_engine_ui::WidgetId::button(lc::BTN_ID_EXPLORER_CTA);
                            debug::click_trace("ZAROXI_CLICK: dispatching Activated(Explorer CTA)");
                            self.handle_actions(vec![
                                zaroxi_core_engine_ui::WidgetAction::Activated(id),
                            ]);
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
            WindowEvent::RedrawRequested => {
                let frame_id = GUI_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed);
                let perf_on = perf_trace_enabled();
                let frame_start = std::time::Instant::now();
                // A redraw arrived: clear the outstanding-redraw bookkeeping so a
                // later invalidation can schedule a fresh one.
                self.frame_scheduler.on_redraw_received();

                // Apply any completed background parse result before shaping the
                // editor content for this frame so fresh highlight spans are
                // used immediately (may invalidate the UI).
                self.poll_parse_results();

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

                    // Notify compositor before rendering this frame.
                    // Required on Wayland to register for the next frame callback.
                    z.window().pre_present_notify();

                    let system_is_dark = z
                        .window()
                        .theme()
                        .map(|t| matches!(t, winit::window::Theme::Dark))
                        .unwrap_or(true);
                    let resolved = self.theme_mode.resolve(system_is_dark);
                    let variant = resolved;

                    let layout_t = std::time::Instant::now();
                    let _ = self.layout_controller.get_or_compute(sw, sh, resolved);
                    let layout_ms = layout_t.elapsed().as_secs_f32() * 1000.0;
                    self.editor_viewport = Some(*self.layout_controller.viewport());
                    self.shell.work_content = self.work_content.clone();

                    let mut sem = variant.colors(false);

                    let debug_theme_active =
                        std::env::var("ZAROXI_DEBUG_THEME").as_deref() == Ok("1");
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

                    let tokens = super::style_tokens_adapter::resolve_style_tokens(
                        &sem,
                        &Default::default(),
                    );

                    if !self.first_render_shown && debug_theme_active {
                        debug::gui_debug_fmt!(
                            "ZAROXI_STYLE_TOKENS: app_bg={:?} titlebar_bg={:?} editor_bg={:?} sidebar_bg={:?}",
                            tokens.app_background.to_array(),
                            tokens.titlebar_background.to_array(),
                            tokens.editor_content_background.to_array(),
                            tokens.sidebar_background.to_array(),
                        );
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
                        let norm_offset = (meta.editor_scroll_top_line as f32
                            / max_scroll.max(1.0))
                        .clamp(0.0, 1.0);
                        let editor_id = WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                        self.interaction.set_scroll_offset(&editor_id, norm_offset);
                    }

                    let engine_layout = self.layout_controller.engine_shell_layout();

                    let content_changed = self
                        .last_widget_tree_content
                        .as_ref()
                        .and_then(|old| {
                            self.work_content.as_ref().map(|new| {
                                old.explorer_empty_button != new.explorer_empty_button
                                    || old.explorer_panel_items.as_ref().map(|v| v.len())
                                        != new.explorer_panel_items.as_ref().map(|v| v.len())
                                    || old.editor_body.as_ref().map(|b| b.lines.len())
                                        != new.editor_body.as_ref().map(|b| b.lines.len())
                                    || old.active_file != new.active_file
                                    || old.editor_tabs != new.editor_tabs
                            })
                        })
                        .unwrap_or(true);
                    let rebuild_tree = self.last_widget_tree_size != (sw, sh) || content_changed;

                    self.last_widget_tree_size = (sw, sh);
                    if let Some(ref wc) = self.work_content {
                        self.last_widget_tree_content = Some(wc.clone());
                    }

                    let mut widget_tree = if rebuild_tree {
                        zaroxi_core_engine_ui::build_shell_widget_tree(
                            engine_layout,
                            &tokens,
                            self.work_content.as_ref(),
                        )
                    } else {
                        self.widget_tree.clone().unwrap_or_else(|| {
                            zaroxi_core_engine_ui::build_shell_widget_tree(
                                engine_layout,
                                &tokens,
                                self.work_content.as_ref(),
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
                            let new_h =
                                (track_rect.height * thumb_ratio).max(min_h).min(track_rect.height);
                            thumb_rect.height = new_h;
                        }
                    }

                    self.interaction.apply_scroll_offsets(&mut widget_tree);
                    self.widget_tree = Some(widget_tree.clone());

                    if scroll_trace_enabled() {
                        let engine_layout = self.layout_controller.engine_shell_layout();
                        let content_right =
                            engine_layout.content_area.x + engine_layout.content_area.width;
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

                    let shell_regions = self.layout_controller.shell_regions();
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
                        super::renderbridge::build_render_layout(shell_regions, &tokens);

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
                    let visible_line_range: Option<(usize, usize)> =
                        self.composition.as_ref().and_then(|comp| comp.metadata.as_ref()).map(
                            |meta| {
                                let scroll_top = meta.editor_scroll_top_line;
                                let scroll_end = scroll_top + editor_visible_lines;
                                (scroll_top, scroll_end.max(scroll_top + 1))
                            },
                        );

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
                    let editor_data = render_state::prepare_editor_data(
                        &self.shell.work_content,
                        &mut self.cached_editor_data,
                        &mut self.cached_editor_lines_hash,
                        &mut self.cached_editor_spans_version,
                        self.latest_spans.as_deref().unwrap_or(&[]),
                        self.latest_spans_version,
                        &sem,
                        &mut self.line_syntax_cache,
                        &mut self.cached_line_hashes,
                        large_file_mode,
                        visible_line_range,
                        Some(self.editor_buffer.rope()),
                    );

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
                    let explorer_data =
                        super::presenters::shape_explorer_content(&self.shell.work_content);
                    let ai_data = super::presenters::shape_ai_content(&self.shell.work_content);

                    let status_inputs = super::status_bar::StatusInputs {
                        file_label: status_file_label.as_deref(),
                        workspace_name: status_workspace_name.as_deref(),
                        cursor_line,
                        cursor_col,
                        text_sample: Some(status_text_sample.as_str()),
                        modified: status_modified,
                        parsing: status_parsing,
                        readonly: false,
                        selection: status_selection,
                        diagnostics: status_diagnostics,
                    };
                    let status_data = super::presenters::shape_status_content(&status_inputs);
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

                    let ctx = super::frame::ShellBlockContext {
                        editor_data,
                        explorer_data,
                        status_bar_data: status_data,
                        ai_data,
                        terminal_tabs: self
                            .work_content
                            .as_ref()
                            .and_then(|wc| wc.terminal_tabs.clone()),
                    };

                    let (mut render_blocks, explorer_cta_rect) =
                        super::frame::compose_blocks(shell_regions, &tokens, &ctx);
                    self.explorer_button_rect = explorer_cta_rect;

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
                                    block
                                        .content_spans
                                        .as_ref()
                                        .map(|s| !s.is_empty())
                                        .unwrap_or(false),
                                );
                            }
                        }
                    }
                    debug::click_trace_fmt!(
                        "ZAROXI_REDRAW: cta_rect={:?}",
                        explorer_cta_rect
                            .map(|(x, y, w, h)| format!("({:.0},{:.0},{:.0}x{:.0})", x, y, w, h))
                    );

                    let editor_total_lines = self.editor_buffer.line_count();
                    let line_h = 16.0f32;

                    if let Some(ref mut comp) = self.composition {
                        comp.set_editor_viewport_lines(editor_visible_lines);
                    }

                    let sidebar_region = crate::gui::region_dispatch::find_region_by_role(
                        shell_regions,
                        zaroxi_core_engine_style::PanelRole::SidePanel,
                    );
                    let sidebar_visible = sidebar_region
                        .map(|r| (r.rect.height as f32 / line_h).max(1.0) as usize)
                        .unwrap_or(1);

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

                    let scroll_blocks = super::frame::compute_scrollbar_blocks(
                        shell_regions,
                        &tokens,
                        editor_total_lines,
                        editor_visible_lines,
                        0,
                        sidebar_visible,
                        0,
                        bottom_visible,
                        editor_scroll_offset,
                    );
                    render_blocks.extend(scroll_blocks);

                    // ── Scrollbar hover/active state bridging ──
                    if let Some(ref tree) = self.widget_tree {
                        for w in &tree.widgets {
                            if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
                                id: zaroxi_core_engine_ui::WidgetId::Scrollbar { index },
                                state,
                                ..
                            } = w
                                && *index == lc::SCROLLBAR_ID_EDITOR
                            {
                                let highlight_color = match *state {
                                    zaroxi_core_engine_ui::InteractionState::Hover
                                    | zaroxi_core_engine_ui::InteractionState::Active => {
                                        let mut c = tokens.editor_scrollbar_thumb.to_array();
                                        c[3] = (c[3] * 2.0).min(1.0);
                                        Some(c)
                                    }
                                    _ => None,
                                };
                                if let Some(color) = highlight_color {
                                    for block in &mut render_blocks {
                                        if block.id == "scrollbar_thumb_editor" {
                                            block.header_color = Some(color);
                                            break;
                                        }
                                    }
                                }
                                break;
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
                            let narrow_or_tall =
                                blk.rect.w <= 10.0 || blk.rect.h > blk.rect.w * 2.0;
                            if narrow_or_tall {
                                eprintln!(
                                    "ZAROXI_SEAM: win={}x{} id='{}' x={:.1} y={:.1} w={:.1} h={:.1}",
                                    sw, sh, blk.id, blk.rect.x, blk.rect.y, blk.rect.w, blk.rect.h,
                                );
                            }
                        }
                    }

                    let is_content_block = |id: &str| {
                        id.contains("ContentArea")
                            || id.contains("content_area")
                            || id == "editor_content"
                    };
                    if let Some(vp) = &self.editor_viewport {
                        for block in &mut render_blocks {
                            if is_content_block(&block.id) {
                                block.cursor_line = Some(cursor_line);
                                block.cursor_col = Some(cursor_col);
                                block.selection_range = selection_range;
                                block.clip_rect = Some(zaroxi_core_engine_render::Rect {
                                    x: vp.clip_rect.0,
                                    y: vp.clip_rect.1,
                                    w: vp.clip_rect.2,
                                    h: vp.clip_rect.3,
                                });
                                if let Some(ref comp) = self.composition
                                    && let Some(meta) = &comp.metadata
                                {
                                    block.content_offset_x =
                                        meta.editor_horizontal_offset_px.unwrap_or(0.0);
                                    let off_y =
                                        meta.editor_scroll_top_line as f32 * lc::LINE_HEIGHT;
                                    block.content_offset_y = off_y;
                                    if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref() == Ok("1") {
                                        eprintln!(
                                            "ZAROXI_SCROLL: block content_offset x={:.1} y={:.1} top_line={}",
                                            block.content_offset_x,
                                            off_y,
                                            meta.editor_scroll_top_line
                                        );
                                    }
                                }
                            }
                        }

                        // Apply vertical scroll offset to the gutter lane block
                        if let Some(ref comp) = self.composition
                            && let Some(meta) = &comp.metadata
                        {
                            let off_y = meta.editor_scroll_top_line as f32 * lc::LINE_HEIGHT;
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
                                block.cursor_col = Some(cursor_col);
                                block.selection_range = selection_range;
                            }
                        }
                    }

                    // ── Renderer lifecycle ──
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
                            rblock_hash =
                                rblock_hash.wrapping_mul(31).wrapping_add(blk.id.len() as u64);
                            rblock_hash =
                                rblock_hash.wrapping_mul(31).wrapping_add(blk.content.len() as u64);
                            rblock_hash = rblock_hash
                                .wrapping_mul(31)
                                .wrapping_add((blk.rect.x * 100.0) as u64);
                            rblock_hash = rblock_hash
                                .wrapping_mul(31)
                                .wrapping_add((blk.rect.y * 100.0) as u64);
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

                    // Create persistent RenderCore on first frame.
                    let core_exists = self.render_core.is_some();
                    if !core_exists {
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
                                self.render_core = Some(core);
                            }
                            Err(e) => {
                                eprintln!("GuiApp: failed to create RenderCore: {:?}", e);
                                return;
                            }
                        }
                    }

                    if let Some(ref mut core) = self.render_core {
                        let surface_size = winit::dpi::PhysicalSize::new(sw, sh);
                        match core.render_to_window(surface_size, &render_layout, &render_blocks) {
                            Ok(perf) => {
                                self.needs_render = false;
                                self.frame_scheduler.on_frame_presented(Instant::now());
                                if perf_on {
                                    let total_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
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
                                if render_trace_enabled() {
                                    eprintln!(
                                        "ZAROXI_RENDER_TRACE: render_result frame={} ok",
                                        frame_id
                                    );
                                }
                                record_frame_presented();
                                if !self.first_render_shown {
                                    z.window().set_visible(true);
                                    self.first_render_shown = true;
                                    eprintln!("GuiApp: first full-renderer frame; window visible");
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
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
                self.ctrl_held = modifiers.state().control_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let ev_kind = input::classify_editor_key(self, &event.logical_key);
                let ev_start = std::time::Instant::now();
                let actions = input::handle_keyboard_press(self, &event.logical_key);
                if let Some(kind) = ev_kind {
                    perf_event(
                        kind,
                        ev_start,
                        &format!(
                            "ln={} col={}",
                            self.editor_cursor_line(),
                            self.editor_cursor_col()
                        ),
                    );
                }
                self.handle_actions(actions);
            }
            _ => {}
        }
    }
}
