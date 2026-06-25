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
pub(crate) mod background_open;
pub(crate) mod background_parse;
pub(crate) mod background_read;
pub(crate) mod debug;
mod editor_interaction;
mod input;
mod render_schedule;
mod render_state;
mod ui_nodes;

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

/// Half-period of the search-box caret blink. The caret is "on" for this long,
/// then "off" for this long, while the explorer search box holds focus.
const CARET_BLINK_INTERVAL_MS: u128 = 530;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(530);

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

/// Whether `ZAROXI_PIPELINE_TRACE=1` is set. Drives the finer-grained
/// `app_update` sub-phase breakdown (widget tree / block build / enrich).
pub(crate) fn pipeline_trace_enabled() -> bool {
    std::env::var("ZAROXI_PIPELINE_TRACE").as_deref() == Ok("1")
}

/// Whether `ZAROXI_SETTLE_TRACE=1` is set. Drives the per-frame open-completion
/// / commit-deferral line. Also implied by `ZAROXI_PERF_TRACE=1`.
pub(crate) fn settle_trace_enabled() -> bool {
    std::env::var("ZAROXI_SETTLE_TRACE").as_deref() == Ok("1") || perf_trace_enabled()
}

/// Whether the staged file-open trace is enabled (`ZAROXI_FILE_OPEN_TRACE=1`,
/// also implied by `ZAROXI_OPEN_TRACE=1`).
pub(crate) fn file_open_trace_enabled() -> bool {
    std::env::var("ZAROXI_FILE_OPEN_TRACE").as_deref() == Ok("1")
        || std::env::var("ZAROXI_OPEN_TRACE").as_deref() == Ok("1")
}

/// Whether the atomic open-presentation trace is enabled
/// (`ZAROXI_OPEN_PRESENT_TRACE=1`, also implied by `ZAROXI_OPEN_TRACE=1`). Drives
/// the per-open snapshot lifecycle line (read_scheduled → … → presented) and the
/// per-frame snapshot-active line, making any non-atomic first paint observable.
pub(crate) fn open_present_trace_enabled() -> bool {
    std::env::var("ZAROXI_OPEN_PRESENT_TRACE").as_deref() == Ok("1")
        || std::env::var("ZAROXI_OPEN_TRACE").as_deref() == Ok("1")
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
    editor_tabs: Option<Vec<String>>,
}

impl WidgetTreeFingerprint {
    fn of(wc: &ShellWorkContent) -> Self {
        Self {
            explorer_empty_button: wc.explorer_empty_button.clone(),
            explorer_items_len: wc.explorer_panel_items.as_ref().map(|v| v.len()),
            explorer_scroll_top: wc.explorer_scroll_top,
            editor_lines_len: wc.editor_body.as_ref().map(|b| b.lines.len()),
            active_file: wc.active_file.clone(),
            editor_tabs: wc.editor_tabs.clone(),
        }
    }
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
    fn begin(token: u64, path: Option<String>) -> Self {
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
    symbols_version: u64,
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
    symbols_version.hash(&mut h);
    diff_version.hash(&mut h);
    h.finish()
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
    /// Frame-paced process memory monitor (`ZAROXI_MEM_TRACE`) driving
    /// pressure-based shaped-glyph cache eviction.
    pub mem_monitor: zaroxi_core_telemetry::MemoryMonitor,
    /// Per-document hot/warm/cold activity tracker (`ZAROXI_BUF_TRACE`).
    pub buffer_tracker: zaroxi_core_telemetry::BufferActivityTracker,
    /// Most recent memory sample, surfaced by the Ctrl+Shift+P dashboard.
    pub last_mem_sample: Option<zaroxi_core_telemetry::MemorySample>,
    /// Sender handed to async AI tasks for `ZAROXI_AI_TRACE` events.
    pub ai_tracer: zaroxi_application_ai::trace::AiTracer,
    /// Receiver drained once per frame to flush AI trace events into the
    /// `ZAROXI_PERF_TRACE` stream.
    pub ai_trace_rx: Option<zaroxi_application_ai::trace::AiTraceReceiver>,
    /// Live AI session state (phase, streamed-token count, latency/throughput)
    /// folded from the drained AI trace events.  The truthful operational
    /// surface for the assistant panel and cockpit status — no invented data.
    pub ai_session: zaroxi_application_ai::view_model::AiSessionState,
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
    /// Explorer tree vertical scroll offset, in rows (first visible row).
    /// Persisted across redraws; clamped each frame against the viewport.
    pub explorer_scroll_top: usize,
    /// Whether the explorer search box currently holds keyboard focus (routes
    /// typing to the filter query instead of the editor).
    pub explorer_search_active: bool,
    /// Rendered hit rect of the explorer search box (x, y, w, h), for click-to-
    /// focus. Set each frame from the sidebar render.
    pub explorer_search_rect: Option<(f32, f32, f32, f32)>,
    /// Index of the currently selected activity rail item (0=Explorer, 1=Search,
    /// 2=Source Ctrl, 3=Debug, 4=Extensions, 5=Settings, 6=Account). Default 0.
    pub rail_selected_index: usize,
    /// Index of the currently hovered activity rail item, or None.
    pub rail_hovered_index: Option<usize>,
    /// Hit rects for each rail item, set each frame from the cockpit rail layout.
    /// `Vec<(rect_x, rect_y, rect_w, rect_h)>` in logical px.
    pub rail_item_hit_rects: Vec<(f32, f32, f32, f32)>,
    /// Selected entry index in the Extensions destination (drives the detail
    /// pane). Updated when an extensions sidebar row is clicked.
    pub extensions_selected_index: usize,
    /// Selected category index in the Settings destination (drives the rows
    /// pane). Updated when a settings sidebar category is clicked.
    pub settings_selected_section: usize,
    /// Hit rects for destination sidebar rows (Extensions list / Settings
    /// categories), set each frame from the sidebar render. `(x, y, w, h)`.
    pub sidebar_row_hit_rects: Vec<(f32, f32, f32, f32)>,
    /// Keyboard-selected row within the (filtered) explorer list, for arrow-key
    /// navigation while searching. Absolute index into the visible item set.
    pub explorer_search_sel: Option<usize>,
    /// Epoch the search caret blink is phased from (reset on focus / keystroke
    /// so the caret is solid while typing).
    pub explorer_caret_blink_epoch: std::time::Instant,
    /// Explorer rows that fit in the viewport (last render), for scroll-into-view
    /// during keyboard navigation.
    pub explorer_visible_rows: usize,
    pub last_render_size: (u32, u32),
    pub pending_scroll_frac: f32,
    pub picker_in_flight: bool,
    pub pending_picker_rx: Option<mpsc::Receiver<PickerOutcome>>,
    pub last_widget_tree_size: (u32, u32),
    pub last_widget_tree_fingerprint: Option<WidgetTreeFingerprint>,
    pub render_core: Option<zaroxi_core_engine_render::renderer::core::RenderCore>,
    /// Set to `true` once the cockpit has produced its first text run, so the
    /// shell path stops emitting breadcrumb text (avoiding duplication).
    pub cockpit_text_active: bool,
    /// True after at least one render pass completed with a cockpit text run
    /// active, so the compositor has presented a frame containing cockpit
    /// content.  Gated on so the window stays hidden until cockpit is actually
    /// visible, preventing a shell-only → shell+cockpit startup flash.
    pub cockpit_rendered_once: bool,
    /// Timestamp of the most recent file open, for status-model latency
    /// probes. When status traces are active, the latency from this timestamp
    /// to the next status model construction is reported as
    /// `status_model_latency_ms_from_open`.
    pub last_open_started_at: Option<std::time::Instant>,
    /// Timestamp of the most recent editor focus change (tab switch / file
    /// focus), for `status_model_latency_ms_from_focus_change`.
    pub last_focus_change_at: Option<std::time::Instant>,
    /// Monotonically-incrementing generation counter for status model
    /// constructions, useful for correlating status trace events to source
    /// events (open, focus, frame).
    pub status_model_generation: u64,
    /// Whether the first stable shell has been painted. Set after the first
    /// `render_to_window` call returns without a cockpit overlay.
    pub startup_first_paint_done: bool,
    /// Timestamp of the first shell paint, for latency probes.
    pub startup_first_paint_at: Option<std::time::Instant>,
    /// Reason string for the second layout re-run (if any), set by the
    /// layout controller before the first resize-invalidated frame.
    pub startup_second_layout_reason: Option<String>,
    /// Approximate retained bytes in cockpit instrumentation (symbols, diff
    /// hunks, widget tree allocations).  Updated each frame the cockpit runs.
    pub cockpit_retained_bytes: usize,
    /// Approximate retained bytes in editor-side state (spans cache, syntax
    /// line cache, rope head). Estimated from key allocations.
    pub editor_retained_bytes: usize,
    /// Fingerprint of the last built cockpit `InstrumentStatus`. When unchanged
    /// across frames (same context, health, AI, RTL), cockpit rebuild is skipped
    /// to avoid ~1ms+ vello scene construction per idle frame.
    pub cockpit_status_fingerprint: u64,
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
    /// Structural minimap symbols (function/type/import) derived from
    /// `latest_spans`, consumed by the cockpit's `SemanticMinimap`.  Recomputed
    /// only when `cockpit_symbols_version` falls behind `latest_spans_version`
    /// (i.e. after a reparse), so it is not rebuilt on every frame.
    pub cockpit_minimap_symbols: Vec<zaroxi_interface_widgets::components::MinimapSymbol>,
    /// `latest_spans_version` the `cockpit_minimap_symbols` were extracted from.
    pub cockpit_symbols_version: u64,
    /// Cached git diff provider (per-file baseline + status cache).  The git
    /// lookup runs once per file; per-edit diffs reuse the cached baseline.
    pub git_diff_provider: zaroxi_core_platform_git::GitDiffProvider,
    /// Per-line change markers for the active file, derived from the git diff,
    /// consumed by the cockpit's `LivingDiffLayer`.  Recomputed only when
    /// `cockpit_diff_version` falls behind the editor buffer version.
    pub cockpit_diff_hunks: Vec<zaroxi_interface_widgets::components::DiffHunk>,
    /// `editor_buffer.buffer_version` the `cockpit_diff_hunks` were computed for.
    pub cockpit_diff_version: u64,
    /// Background parse worker for off-thread tree-sitter parsing.
    pub parse_worker: Option<background_parse::BackgroundParseWorker>,
    /// `editor_buffer.buffer_version` captured when the active file was last
    /// loaded (or saved). The document is considered modified when the live
    /// buffer version diverges from this baseline.
    pub saved_buffer_version: u64,
    /// Redraw coalescing + frame pacing. `needs_render` is the dirty flag; this
    /// owns the pacing/cadence and outstanding-redraw bookkeeping.
    pub frame_scheduler: FrameScheduler,
    /// Retained per-element UI-node fingerprints driving `ZAROXI_UI_TRACE`
    /// (which shell elements rebuilt vs. reused, and why) each frame.
    pub ui_node_tracker: ui_nodes::UiNodeTracker,
    /// True from the moment a file is opened until the freshly-visible editor
    /// viewport has shaped fully (no deferred lines). While set, the renderer
    /// runs an "open burst" budget so the viewport completes in one pass
    /// instead of settling across several frames.
    pub open_settling: bool,
    /// Consecutive open-burst frames so far; caps the burst so a pathological
    /// queue can never wedge the budget permanently raised.
    pub open_burst_frames: u32,
    /// True while a window resize / scale change is in flight. Used to defer
    /// syntax/content commits off the geometry-reset frame.
    pub resize_pending: bool,
    /// Set when a parse-result commit was deferred this frame because the open
    /// viewport was still settling (for `ZAROXI_SETTLE_TRACE`).
    pub commit_deferred_open: bool,
    /// Set when a parse-result commit was deferred this frame because a resize
    /// was in flight (for `ZAROXI_SETTLE_TRACE`).
    pub commit_deferred_resize: bool,
    /// Monotonic open-request generation. Incremented on every `request_open`
    /// (explorer selection / programmatic open). The newest token always wins;
    /// any in-flight or pending open carrying an older token is stale.
    pub open_token: u64,
    /// Token of the open that was last actually committed (buffer materialized).
    pub committed_open_token: u64,
    /// True when the next open-settle frame should spend the one-time
    /// first-screenful budget (shape the visible rows at once), after which it
    /// drops to the progressive budget for below-the-fold rows. Set when a
    /// buffer becomes current (sync open or background commit-on-ready).
    pub open_first_screenful_pending: bool,
    /// Latest requested-but-not-yet-committed work content + its token. Only
    /// the newest is kept: a newer `request_open` supersedes (drops) the
    /// previous pending open before it ever does the heavy buffer load, so
    /// rapid file switching never runs stale opens.
    pub pending_open: Option<(u64, ShellWorkContent)>,
    /// Active file path of the last committed open (for change detection at
    /// commit time, since `work_content` is updated eagerly for instant chrome).
    pub committed_active_file: Option<String>,
    /// Total open requests seen (for `ZAROXI_FILE_OPEN_TRACE`).
    pub file_switch_count: u64,
    /// True between a `request_open` and its commit: the editor still shows the
    /// previous content (or a placeholder) while the new file loads.
    pub visible_loading_state: bool,
    /// When the latest open was requested, for time-to-viewport accounting.
    pub open_request_at: Option<std::time::Instant>,
    /// Wall time (ms) the most recent explorer open spent in the *upstream*
    /// synchronous prep (`open_buffer_by_path` disk read + buffer build) before
    /// `request_open` was even reached. This is the dominant time-to-first-text
    /// cost for huge files and is surfaced via `ZAROXI_FILE_OPEN_TRACE`.
    pub last_upstream_open_prep_ms: f32,
    /// Background worker that performs the blocking disk read / buffer load off
    /// the UI thread. Spawned lazily on the first explorer file open.
    pub read_worker: Option<background_read::BackgroundReadWorker>,
    /// Monotonic read-request generation (distinct from `open_token`): gates the
    /// background read so a stale file's read result never activates.
    pub read_token: u64,
    /// True while a background read is in flight (no buffer activated yet). Keeps
    /// the loop polling for the result.
    pub read_pending: bool,
    /// When the in-flight background read was scheduled (read-latency trace).
    pub read_started_at: Option<std::time::Instant>,
    /// Latest requested read token, shared with the read worker so it can skip
    /// starting a read that a newer file click already superseded.
    pub read_generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Background worker that materializes large files' ropes off the UI thread.
    /// Spawned lazily on the first heavy open.
    pub open_worker: Option<background_open::BackgroundOpenWorker>,
    /// True while a background open (rope build) for the winning token is in
    /// flight (no buffer committed yet). Keeps the redraw loop polling.
    pub background_open_pending: bool,
    /// When the in-flight background open job was scheduled (commit-latency).
    pub open_worker_started_at: Option<std::time::Instant>,
    /// Phase 11 — the in-flight atomic open presentation (read-schedule → present),
    /// or `None` when no open is being staged. Newest open replaces the snapshot.
    pub open_present: Option<OpenPresentation>,
    /// True for the single first-paint frame of a freshly-committed open: that
    /// frame shapes the entire visible screenful in one pass (large one-shot
    /// budget) so the new file is presented coherently and atomically, never as a
    /// partially-shaped top viewport that re-settles over the next frames. Cleared
    /// once the screenful is shaped; below-the-fold then fills progressively.
    pub open_atomic_first_paint: bool,
    /// Initial window geometry recorded on the first Resized event (or first
    /// render frame if no resize occurred).  Used to detect whether the
    /// compositor negotiated a different size than what was requested.
    pub startup_geometry_initial: Option<(u32, u32)>,
    /// Window geometry recorded when the window was first made visible —
    /// the "final" settled geometry for the first visible paint.
    pub startup_geometry_final: Option<(u32, u32)>,
    /// Human-readable reason the window geometry changed between initial and final
    /// (e.g. "compositor_resize", "no_change").
    pub startup_geometry_changed_reason: Option<String>,
    /// True once the first visible frame uses the final stable layout (no pending
    /// resize, cockpit active).
    pub startup_first_visible_layout_stable: bool,
    /// True after the one-time post-settle cache trim has run (eviction of cold
    /// shape-cache entries + line-syntax-cache prune).
    pub startup_settle_trimmed: bool,
    /// Monotonic version counter for the text instance buffer, incremented each
    /// time the persistent buffer is reallocated (growth). Used by the renderer
    /// to detect stale buffers from across-frame resizes.
    pub text_instance_buffer_version: u64,
}

/// Phase 6 two-tier open shaping budget.
///
/// Phase 5 enforced one strict per-frame budget during the whole open, which —
/// at a low value — made the visible screenful's *uncached* rows trickle in one
/// per frame. Phase 6 instead spends a single bounded **first-screenful** budget
/// on the first post-commit open frame (so the visible rows shape at once), then
/// drops to a low **progressive** budget for everything below the fold. Neither
/// tier ever reintroduces a hundreds-of-ms burst.
const OPEN_FIRST_SCREENFUL_BUDGET_MS_DEFAULT: f32 = 20.0;
const OPEN_PROGRESSIVE_BUDGET_MS_DEFAULT: f32 = 6.0;

/// One-time budget (ms) for the first post-commit open frame: large enough to
/// fully shape the visible screenful's new rows in a single bounded frame.
/// `ZAROXI_OPEN_FIRST_SCREENFUL_BUDGET_MS`, clamped 4..=40. Deliberately ignores
/// the legacy `ZAROXI_OPEN_SHAPE_BUDGET_MS` so a pathological `=1` cannot starve
/// the first screenful.
pub(crate) fn open_first_screenful_budget_ms() -> f32 {
    std::env::var("ZAROXI_OPEN_FIRST_SCREENFUL_BUDGET_MS")
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(4.0, 40.0))
        .unwrap_or(OPEN_FIRST_SCREENFUL_BUDGET_MS_DEFAULT)
}

/// Per-frame budget (ms) for progressive below-the-fold / overscan fill after
/// the first screenful is shown. `ZAROXI_OPEN_PROGRESSIVE_BUDGET_MS` (legacy
/// `ZAROXI_OPEN_SHAPE_BUDGET_MS`), clamped 1..=16.
pub(crate) fn open_progressive_budget_ms() -> f32 {
    std::env::var("ZAROXI_OPEN_PROGRESSIVE_BUDGET_MS")
        .ok()
        .or_else(|| std::env::var("ZAROXI_OPEN_SHAPE_BUDGET_MS").ok())
        .and_then(|s| s.trim().parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(1.0, 16.0))
        .unwrap_or(OPEN_PROGRESSIVE_BUDGET_MS_DEFAULT)
}

/// Phase 11 atomic first paint: the one-shot shape budget (ms) spent on the
/// single first-paint frame of a freshly-committed open. Deliberately large so
/// the entire VISIBLE screenful (viewport-windowed for every file size — see
/// `render_state::prepare_editor_data`) shapes in one pass, presenting the new
/// file coherently in a single frame instead of trickling rows across several.
/// It is screenful-bounded in practice (only visible + overscan rows are in the
/// render blocks), so it never shapes the whole document.
pub(crate) const OPEN_ATOMIC_FIRST_PAINT_BUDGET_MS: f32 = 100_000.0;

/// Defensive cap on consecutive open-settle frames before force-clearing the
/// open-settling flag, in case `shaping_pending` never reaches zero. High enough
/// that a legitimate progressive viewport fill (heavy file shaped a few rows per
/// frame under the hard cap) completes well within it.
const OPEN_BURST_MAX_FRAMES: u32 = 600;

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

/// At/above this size the rope is materialized on the background open worker
/// instead of the UI thread, so a heavy open never monopolizes input/render.
/// Smaller files build in well under a frame and stay fully synchronous (no
/// placeholder flicker).
const BACKGROUND_OPEN_LINE_THRESHOLD: usize = HUGE_FILE_LINE_THRESHOLD;
/// Byte equivalent of the background-open threshold (long-line files).
const BACKGROUND_OPEN_BYTE_THRESHOLD: usize = 512 * 1024;

impl GuiApp {
    pub fn editor_cursor_line(&self) -> usize {
        self.editor_buffer.caret_line()
    }

    /// Print a consolidated, human-readable performance dashboard across all
    /// observability subsystems (memory pressure, multi-buffer activity, cache
    /// footprint). Bound to Ctrl+Shift+P. The fine-grained per-event TS/AI/LSP
    /// latency data streams inline as `ZAROXI_*_TRACE` lines; this is the
    /// at-a-glance snapshot.
    pub fn dashboard(&self) {
        let frame = GUI_FRAME_COUNTER.load(Ordering::Relaxed);
        let rss_now = zaroxi_core_telemetry::read_rss_bytes().unwrap_or(0);
        eprintln!("==================== ZAROXI PERFORMANCE DASHBOARD ====================");
        match &self.last_mem_sample {
            Some(s) => eprintln!(
                "  memory   : rss={:.1} MB  pressure={}  shape_cache={} KB  gpu={} KB  rope={} KB",
                s.rss_bytes as f64 / (1024.0 * 1024.0),
                s.pressure,
                s.shape_cache_bytes / 1024,
                s.gpu_bytes / 1024,
                s.rope_bytes / 1024,
            ),
            None => eprintln!(
                "  memory   : rss={:.1} MB  (no sample yet \u{2014} sample every {} frames)",
                rss_now as f64 / (1024.0 * 1024.0),
                zaroxi_core_telemetry::memory::DEFAULT_SAMPLE_FRAMES,
            ),
        }
        let (hot, warm, cold) = self.buffer_tracker.class_counts(frame);
        eprintln!(
            "  buffers  : open={} total_lines={} hot={} warm={} cold={}",
            self.buffer_tracker.open_count(),
            self.buffer_tracker.total_lines(),
            hot,
            warm,
            cold,
        );
        eprintln!("  latency  : per-event TS / AI / LSP timings stream as ZAROXI_TS_TRACE,");
        eprintln!("             ZAROXI_AI_TRACE, ZAROXI_LSP_TRACE (set ZAROXI_PERF_TRACE=1)");
        eprintln!("=====================================================================");
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

    /// Stage A — request an open. Cheap and non-blocking: stamps a new open
    /// token, updates the chrome (explorer selection / title / status) instantly
    /// via `work_content`, and records the content as *pending*. The heavy
    /// buffer load happens later in `commit_open` (next frame). A newer
    /// `request_open` supersedes the pending one, so rapid explorer switching
    /// never runs a stale file's heavy open work. The newest selection wins.
    fn request_open(&mut self, wc: ShellWorkContent) {
        self.open_token += 1;
        let token = self.open_token;
        self.file_switch_count += 1;
        // Supersede any not-yet-committed open: its heavy load never runs.
        if let Some((stale_token, _)) = self.pending_open.take()
            && file_open_trace_enabled()
        {
            eprintln!(
                "ZAROXI_FILE_OPEN_TRACE: token={} stage=cancelled cancelled=1 superseded_by={} commit_skipped_stale=1 t_ms=0.00",
                stale_token, token,
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

    /// Stages B–E — commit the newest pending open. Runs once per frame from the
    /// redraw loop. Does the heavy work (buffer materialization, large-file
    /// decision, background syntax kickoff, open burst) for the *latest* token
    /// only; superseded requests were already dropped in `request_open`, so no
    /// stale buffer is ever materialized or committed.
    fn commit_open(&mut self) {
        let (token, wc) = match self.pending_open.take() {
            Some(p) => p,
            None => return,
        };
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
        let buffer_changed = self.committed_active_file.as_deref() != wc.active_file.as_deref()
            || detected_language != self.current_language;
        self.current_language = detected_language;
        if buffer_changed {
            self.latest_spans = None;
            self.latest_spans_version = 0;
            if let Some(ref mut worker) = self.parse_worker {
                worker.clear_result();
            }
            // Trim retained editor caches on file switch to bound RSS.
            self.line_syntax_cache.clear();
            self.cached_line_hashes.clear();
            self.editor_retained_bytes = 0;
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
        // Only (re)materialize the editor buffer when the active file actually
        // changed. A same-file commit — e.g. the instant "loading" chrome ack
        // returned by `dispatch_activation` before the off-thread read lands, or
        // a status-message refresh — must NOT re-populate/re-background the
        // buffer it already holds; it is a pure chrome update.
        if buffer_changed && let Some(ref body) = wc.editor_body {
            // Detect large-file mode from the incoming content view.
            self.large_file_mode = Self::is_large_file(&body.lines);
            let open_bytes: usize = body.lines.iter().map(|l| l.len()).sum();
            if Self::should_background_open(&body.lines) {
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
                let open_buffer_ms = open_t.elapsed().as_secs_f32() * 1000.0;
                self.finalize_buffer_commit(buffer_changed);
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
            if self.large_file_mode
                && std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1")
            {
                eprintln!(
                    "ZAROXI_DEBUG_LARGE_FILE: large_file_mode ON lines={} bytes={} backgrounded={}",
                    body.lines.len(),
                    body.lines.iter().map(|l| l.len()).sum::<usize>(),
                    backgrounded,
                );
            }
        }
        self.committed_active_file = wc.active_file.clone();
        if !backgrounded {
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
        // The freshly loaded content is the saved baseline for dirty tracking.
        self.saved_buffer_version = self.editor_buffer.buffer_version;
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
        // Spawn background parse worker for off-thread syntax highlighting.
        if self.parse_worker.is_none() {
            self.parse_worker =
                Some(background_parse::BackgroundParseWorker::spawn(Arc::clone(&self.parser_pool)));
        }
        // Large/huge files skip full-document tree-sitter parsing entirely —
        // viewport-only plain-text fallback is already active via large_file_mode.
        if self.large_file_mode {
            if std::env::var("ZAROXI_DEBUG_LARGE_FILE").as_deref() == Ok("1") {
                eprintln!(
                    "ZAROXI_DEBUG_LARGE_FILE: finalize_buffer_commit SKIPPED bg parse (large_file_mode lines={})",
                    self.editor_buffer.line_count(),
                );
            }
        } else if let Some(ref mut worker) = self.parse_worker {
            let text = self.editor_buffer.to_string();
            let version = self.editor_buffer.buffer_version;
            let language = self.current_language;
            worker.schedule_parse(background_parse::BufferSnapshot { version, text, language });
        }
    }

    /// Commit-on-ready: install a completed background-open rope for the winning
    /// token. Stale results (a newer open superseded this one) are dropped so no
    /// old content ever flashes in. No-op when no result is pending.
    fn poll_open_results(&mut self) {
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

    /// Drain background *read* outcomes (Phase 8/10/11). The `Head` outcome is
    /// telemetry only: it no longer performs a separate visible swap, because a
    /// head preview painted before the registered buffer is active leaves the
    /// chrome (tab/title/status) showing the *previous* file for a frame and
    /// forces a second top-of-viewport settle when the `Full` buffer lands. The
    /// old file (or loading shell) therefore stays visible until the single,
    /// coherent atomic first paint at the `Full` activation. The `Full` outcome
    /// activates the registered buffer and feeds it into the token-gated
    /// `request_open` path. Stale outcomes (a newer file was clicked) are dropped.
    fn poll_read_results(&mut self) {
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
                if file_open_trace_enabled() {
                    let is_full = matches!(outcome, background_read::ReadOutcome::Full { .. });
                    let (cancelled, read_ms) = match &outcome {
                        background_read::ReadOutcome::Full { cancelled, read_ms, .. } => {
                            (*cancelled as u8, *read_ms)
                        }
                        background_read::ReadOutcome::Head { .. } => (0, 0.0),
                    };
                    eprintln!(
                        "ZAROXI_FILE_OPEN_TRACE: read_token={} stage=read_stale_dropped superseded_by={} is_full={} read_skipped_before_start={} wasted_read_ms={:.2}",
                        tok, self.read_token, is_full as u8, cancelled, read_ms,
                    );
                }
                if matches!(outcome, background_read::ReadOutcome::Full { .. })
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
                            self.request_open(wc.clone());
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
        // Commit a completed background-open rope (winning token only); this
        // invalidates the UI so the freshly materialized buffer paints.
        self.poll_open_results();
        // Commit a completed off-thread read (winning token only); this issues a
        // `request_open` which invalidates and schedules the next frame.
        self.poll_read_results();

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
        } else if self.picker_in_flight
            || self.parse_result_pending()
            || self.background_open_pending
            || self.read_pending
        {
            // Background work is in flight; poll on a relaxed cadence so the
            // result is applied promptly without pinning a CPU core.
            active_loop.set_control_flow(ControlFlow::WaitUntil(now + BACKGROUND_POLL_INTERVAL));
        } else if self.explorer_search_active {
            // Search box focused and otherwise idle: blink the caret by waking
            // at each toggle and arming a repaint. Bounded to the focused state,
            // so it never affects frame pacing or background polling elsewhere.
            self.invalidate(InvalidationFlags::content());
            active_loop.set_control_flow(ControlFlow::WaitUntil(now + CARET_BLINK_INTERVAL));
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
                if self.startup_geometry_initial.is_none() {
                    self.startup_geometry_initial = Some((size.width, size.height));
                    self.startup_geometry_changed_reason =
                        Some("compositor_resize_before_first_paint".to_string());
                } else {
                    self.startup_geometry_final = Some((size.width, size.height));
                }
                self.resize_pending = true;
                self.invalidate(InvalidationFlags::resize());
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                debug::gui_debug("GuiApp: ScaleFactorChanged -> invalidating");
                self.resize_pending = true;
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

                // Rail hover detection (cockpit-owned surface, not in shell tree).
                {
                    let px = position.x as f32;
                    let py = position.y as f32;
                    let mut hit_idx = None;
                    for (i, &(rx, ry, rw, rh)) in self.rail_item_hit_rects.iter().enumerate() {
                        if px >= rx && px < rx + rw && py >= ry && py < ry + rh {
                            hit_idx = Some(i);
                            break;
                        }
                    }
                    if hit_idx != self.rail_hovered_index {
                        self.rail_hovered_index = hit_idx;
                        self.cockpit_status_fingerprint = 0;
                        self.needs_render = true;
                    }
                }

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
                // Rail item click (cockpit-owned, separate from shell tree).
                if let ElementState::Released = state {
                    let rail_idx = self.rail_hovered_index;
                    if let Some(idx) = rail_idx {
                        self.rail_selected_index = idx;
                        self.cockpit_status_fingerprint = 0;
                        self.needs_render = true;
                        let id = zaroxi_core_engine_style::WidgetId::list_item(idx);
                        self.handle_actions(vec![zaroxi_core_engine_ui::WidgetAction::Activated(
                            id,
                        )]);
                        return;
                    }
                }
                // Destination sidebar row click (Extensions list / Settings
                // categories). Updates the active selection so the cockpit
                // detail / rows pane re-renders for the clicked item, and the
                // sidebar re-highlights the selected row.
                if let ElementState::Released = state {
                    use super::destination::WorkbenchDestination as D;
                    let dest = D::from_rail_index(self.rail_selected_index);
                    if matches!(dest, D::Extensions | D::Settings) {
                        let hit =
                            self.sidebar_row_hit_rects.iter().position(|&(rx, ry, rw, rh)| {
                                x >= rx && x < rx + rw && y >= ry && y < ry + rh
                            });
                        if let Some(row) = hit {
                            match dest {
                                D::Extensions => self.extensions_selected_index = row,
                                D::Settings => self.settings_selected_section = row,
                                _ => {}
                            }
                            self.cockpit_status_fingerprint = 0;
                            self.needs_render = true;
                            return;
                        }
                    }
                }
                // Explorer search box focus: clicking the box grabs keyboard
                // focus; clicking anywhere else releases it (the filter itself
                // persists until cleared with Escape).
                if let ElementState::Released = state {
                    let in_search = self.explorer_search_rect.is_some_and(|(sx, sy, sw, sh)| {
                        x >= sx && x < sx + sw && y >= sy && y < sy + sh
                    });
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
                let commit_open_ms =
                    _t_commit.map(|t| t.elapsed().as_secs_f32() * 1000.0).unwrap_or(0.0);
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

                // Exact monospace advance, captured before the window borrow so
                // the explorer presenter can size ellipsis / highlight columns.
                let mono_advance = self.monospace_advance_x().unwrap_or(lc::CHAR_WIDTH_STUB);

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
                    let content_changed =
                        match (&self.last_widget_tree_fingerprint, &new_fingerprint) {
                            (Some(old), Some(new)) => old != new,
                            _ => true,
                        };
                    let rebuild_tree = self.last_widget_tree_size != (sw, sh) || content_changed;

                    self.last_widget_tree_size = (sw, sh);
                    if new_fingerprint.is_some() {
                        self.last_widget_tree_fingerprint = new_fingerprint;
                    }

                    let mut widget_tree = if rebuild_tree {
                        zaroxi_core_engine_ui::build_shell_widget_tree(
                            engine_layout,
                            &tokens,
                            self.work_content.as_ref(),
                        )
                    } else {
                        self.widget_tree.take().unwrap_or_else(|| {
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

                    // Store the fully-interacted tree (move, not clone) so enrich
                    // passes below can read interaction state from `self.widget_tree`.
                    self.widget_tree = Some(widget_tree);
                    let widget_ms = widget_t.elapsed().as_secs_f32() * 1000.0;

                    // ── Startup trace (first 10 frames) ──────────────────
                    let startup_trace = std::env::var("ZAROXI_STARTUP_TRACE").as_deref() == Ok("1")
                        && frame_id < 10;
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

                    // Editor + status rects (owned Copy tuples) for the cockpit
                    // overview/status anchoring further down. Captured here while
                    // `shell_regions` is borrowed so the cockpit block needs no
                    // further borrow of the layout controller.
                    let cockpit_editor_rect: (f32, f32, f32, f32) = editor_region
                        .map(|r| {
                            (
                                r.rect.x as f32,
                                r.rect.y as f32,
                                r.rect.width as f32,
                                r.rect.height as f32,
                            )
                        })
                        .unwrap_or((0.0, 0.0, 0.0, 0.0));
                    let cockpit_status_rect: (f32, f32, f32, f32) =
                        crate::gui::region_dispatch::find_region_by_role(
                            shell_regions,
                            zaroxi_core_engine_style::PanelRole::StatusBar,
                        )
                        .map(|r| {
                            (
                                r.rect.x as f32,
                                r.rect.y as f32,
                                r.rect.width as f32,
                                r.rect.height as f32,
                            )
                        })
                        .unwrap_or((0.0, 0.0, 0.0, 0.0));
                    let cockpit_rail_rect: (f32, f32, f32, f32) =
                        crate::gui::region_dispatch::find_region_by_role(
                            shell_regions,
                            zaroxi_core_engine_style::PanelRole::NavigationRail,
                        )
                        .map(|r| {
                            (
                                r.rect.x as f32,
                                r.rect.y as f32,
                                r.rect.width as f32,
                                r.rect.height as f32,
                            )
                        })
                        .unwrap_or((0.0, 0.0, 0.0, 0.0));
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
                        &self.work_content,
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
                        self.editor_buffer.buffer_version,
                    );
                    // Estimate retained editor bytes for memory trace.
                    self.editor_retained_bytes = self
                        .line_syntax_cache
                        .iter()
                        .map(|(_, v)| v.iter().map(|(s, _)| s.len()).sum::<usize>())
                        .sum::<usize>()
                        + self.cached_line_hashes.len() * 8
                        + self.latest_spans.as_ref().map(|s| s.len() * 32).unwrap_or(0);

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
                    let mut explorer_data =
                        super::presenters::shape_explorer_content(&self.work_content);
                    // Exact monospace advance for ellipsis truncation + match-run
                    // highlight positioning; blink-phased caret; keyboard nav row.
                    explorer_data.char_advance = mono_advance;
                    explorer_data.selected_row = self.explorer_search_sel;
                    explorer_data.search_caret_visible = self.explorer_search_active
                        && (self.explorer_caret_blink_epoch.elapsed().as_millis()
                            / CARET_BLINK_INTERVAL_MS)
                            .is_multiple_of(2);
                    let mut ai_data = super::presenters::shape_ai_content(&self.work_content);
                    // Surface truthful AI session status in the assistant panel.
                    // Additive only: fills the subtitle when the panel has none
                    // of its own, so an active/completed request is visible
                    // without clobbering real content. Idle -> None -> unchanged.
                    if ai_data.ai_subtitle.as_deref().map(str::trim).unwrap_or("").is_empty() {
                        if let Some(status) = self.ai_session.status_label() {
                            ai_data.ai_subtitle = Some(status);
                        }
                    }

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
                    // ── Startup trace: status model ──────────────────────
                    let _ts = if startup_trace { Some(std::time::Instant::now()) } else { None };
                    let status_data = super::presenters::shape_status_content(&status_inputs);
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
                        super::status_bar::instrument_context(&status_data);
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
                    let destination = super::destination::WorkbenchDestination::from_rail_index(
                        self.rail_selected_index,
                    );
                    let sidebar_list = super::destination::sidebar_rows(
                        destination,
                        self.extensions_selected_index,
                        self.settings_selected_section,
                    );
                    let ctx = super::frame::ShellBlockContext {
                        editor_data,
                        explorer_data,
                        status_bar_data: status_data,
                        ai_data,
                        terminal_tabs: self
                            .work_content
                            .as_ref()
                            .and_then(|wc| wc.terminal_tabs.clone()),
                        destination,
                        sidebar_list,
                        cockpit_text_active: self.cockpit_text_active,
                    };

                    // ── Startup trace: shell block composition ───────────
                    let _tc = if startup_trace { Some(std::time::Instant::now()) } else { None };
                    let (mut render_blocks, explorer_cta_rect, explorer_search_rect, sidebar_rows) =
                        super::frame::compose_blocks(shell_regions, &tokens, &ctx);
                    if let Some(t) = _tc {
                        eprintln!(
                            "ZAROXI_STARTUP_TRACE: frame={} phase=first_frame_shell_build ms={:.2}",
                            frame_id,
                            t.elapsed().as_secs_f32() * 1000.0
                        );
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
                        let max_scroll =
                            sidebar_items.saturating_sub(sidebar_visible).max(1) as f32;
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

                    let scroll_blocks = super::frame::compute_scrollbar_blocks(
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
                        core.set_shape_budget_ms(if open_settling {
                            Some(open_budget_ms)
                        } else {
                            None
                        });
                        // ── Cockpit build BEFORE render pass ────────────
                        // Built here so the cockpit scene is available
                        // for this frame's render pass, not next frame's.
                        {
                            let cur_line = self.editor_buffer.caret_line();
                            let do_cockpit = super::cockpit::cockpit_surfaces_active();
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
                                    super::cockpit::cockpit_tokens(self.theme_mode, system_is_dark);
                                let ai_band = {
                                    use zaroxi_application_ai::view_model::AiPhase;
                                    let used = self.ai_session.tokens_streamed as u32;
                                    let mode = match self.ai_session.phase {
                                        AiPhase::Idle => zaroxi_interface_widgets::AiMode::Dormant,
                                        AiPhase::PromptBuilding
                                        | AiPhase::Requesting
                                        | AiPhase::Streaming => {
                                            zaroxi_interface_widgets::AiMode::Live
                                        }
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
                                let instrument_status =
                                    zaroxi_interface_widgets::InstrumentStatus {
                                        context: cockpit_context.clone(),
                                        meta: cockpit_meta.clone(),
                                        health: health_band,
                                        ai: ai_band,
                                        rtl: status_rtl,
                                    };
                                let fp = instrument_status_fingerprint(
                                    &instrument_status,
                                    (sw, sh),
                                    self.cockpit_symbols_version,
                                    self.cockpit_diff_version,
                                );
                                let skip = self.cockpit_text_active
                                    && fp == self.cockpit_status_fingerprint;
                                self.cockpit_status_fingerprint = fp;
                                if !skip {
                                    let inputs = super::cockpit::CockpitInputs {
                                        width: sw as f32,
                                        height: sh as f32,
                                        editor_rect: cockpit_editor_rect,
                                        status_rect: cockpit_status_rect,
                                        rail_rect: cockpit_rail_rect,
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
                                        line_height: 18.0,
                                        total_lines: editor_total_lines,
                                        minimap_symbols: self.cockpit_minimap_symbols.clone(),
                                        diff_hunks: self.cockpit_diff_hunks.clone(),
                                        viewport: super::cockpit::cursor_viewport(
                                            cur_line,
                                            editor_total_lines,
                                        ),
                                        status: instrument_status,
                                        settings_panel: {
                                            use super::destination::WorkbenchDestination as D;
                                            if D::from_rail_index(self.rail_selected_index)
                                                == D::Settings
                                            {
                                                let sections = super::destination::settings_sections(
                                                    &format!("{:?}", self.theme_mode),
                                                );
                                                let sel = self
                                                    .settings_selected_section
                                                    .min(sections.len().saturating_sub(1));
                                                Some((sections, sel))
                                            } else {
                                                None
                                            }
                                        },
                                        extensions_panel: {
                                            use super::destination::WorkbenchDestination as D;
                                            if D::from_rail_index(self.rail_selected_index)
                                                == D::Extensions
                                            {
                                                let entries = super::destination::extension_entries();
                                                let sel = self
                                                    .extensions_selected_index
                                                    .min(entries.len().saturating_sub(1));
                                                Some((entries, sel))
                                            } else {
                                                None
                                            }
                                        },
                                        placeholder_panel:
                                            super::destination::WorkbenchDestination::from_rail_index(
                                                self.rail_selected_index,
                                            )
                                            .placeholder(),
                                        ..Default::default()
                                    };
                                    let (scene, text) = super::cockpit::build_cockpit_frame(
                                        &inputs,
                                        &cockpit_tokens,
                                    );
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
                                        let slot_w =
                                            if count > 0 { rw / count as f32 } else { 0.0 };
                                        let mut rects = Vec::new();
                                        for i in 0..count {
                                            let sx = rx + i as f32 * slot_w;
                                            rects.push((sx, ry, slot_w, rh));
                                        }
                                        rects
                                    };
                                }
                                let cockpit_bytes_est =
                                    self.cockpit_minimap_symbols.len().saturating_mul(64)
                                        + self.cockpit_diff_hunks.len().saturating_mul(32)
                                        + 1024;
                                self.cockpit_retained_bytes = cockpit_bytes_est;
                            }
                        }
                        // ── end cockpit build ───────────────────────────

                        match core.render_to_window(surface_size, &render_layout, &render_blocks) {
                            Ok(perf) => {
                                let total_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
                                self.needs_render = false;
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
                                    eprintln!(
                                        "ZAROXI_RENDER_TRACE: render_result frame={} ok",
                                        frame_id
                                    );
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
                                    let pressure = self.mem_monitor.evaluate(rss);
                                    let (shape_cache_bytes, gpu_bytes) = core
                                        .text_renderer()
                                        .map(|tr| (tr.mem_shape_cache_bytes(), tr.mem_gpu_bytes()))
                                        .unwrap_or((0, 0));
                                    let rope_bytes = self.editor_buffer.char_count() as u64;
                                    // Best-effort active-buffer feed (multi-doc
                                    // feeding is via the tracker API on open/close).
                                    self.buffer_tracker.note_open("active", editor_total_lines);
                                    self.buffer_tracker.set_active("active");
                                    self.buffer_tracker.set_visible(["active"]);
                                    let sample = zaroxi_core_telemetry::MemorySample {
                                        rss_bytes: rss,
                                        shape_cache_bytes,
                                        rope_bytes,
                                        gpu_bytes,
                                        open_docs: self.buffer_tracker.open_count(),
                                        total_lines: self.buffer_tracker.total_lines(),
                                        pressure,
                                    };
                                    sample.emit();
                                    self.buffer_tracker.emit(frame_id);
                                    self.last_mem_sample = Some(sample);
                                    // ── Memory trace ──────────────────────
                                    if std::env::var("ZAROXI_MEM_TRACE").as_deref() == Ok("1") {
                                        let shape_bytes = shape_cache_bytes as usize;
                                        let shape_entries = core
                                            .text_renderer()
                                            .map(|tr| tr.shape_cache_entries())
                                            .unwrap_or(0);
                                        eprintln!(
                                            "ZAROXI_MEM_TRACE: frame={} rss_mb={:.0} shape_cache_kb={} shape_cache_entries={} gpu_kb={} cockpit_retained_kb={} editor_retained_kb={} rope_kb={}",
                                            frame_id,
                                            rss as f64 / (1024.0 * 1024.0),
                                            shape_bytes / 1024,
                                            shape_entries,
                                            gpu_bytes as usize / 1024,
                                            self.cockpit_retained_bytes / 1024,
                                            self.editor_retained_bytes / 1024,
                                            rope_bytes as usize / 1024,
                                        );
                                    }
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
                                let do_cockpit = super::cockpit::cockpit_surfaces_active();
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
                                    let tokens = super::cockpit::cockpit_tokens(
                                        self.theme_mode,
                                        system_is_dark,
                                    );
                                    // Track retained cockpit size (symbols, hunks).
                                    let cockpit_bytes_est =
                                        self.cockpit_minimap_symbols.len().saturating_mul(64)
                                            + self.cockpit_diff_hunks.len().saturating_mul(32)
                                            + 1024;
                                    self.cockpit_retained_bytes = cockpit_bytes_est;
                                    // Live state via disjoint field access (core
                                    // is mutably borrowed, so avoid &self methods).
                                    let cur_line = self.editor_buffer.caret_line();
                                    // Refresh structural minimap symbols only when
                                    // a fresh parse result arrived (spans change on
                                    // reparse, not per frame): incremental
                                    // invalidation, no per-frame full-file rescan.
                                    // Disjoint field access keeps this clear of the
                                    // `core` borrow above.
                                    if self.latest_spans_version != self.cockpit_symbols_version {
                                        let symbols = if self.large_file_mode {
                                            // Large files skip span-based symbols to
                                            // avoid materializing the whole document.
                                            Vec::new()
                                        } else {
                                            match self.latest_spans.as_ref() {
                                                Some(spans) if !spans.is_empty() => {
                                                    let source = self.editor_buffer.to_string();
                                                    super::cockpit::extract_minimap_symbols(
                                                        spans, &source,
                                                    )
                                                }
                                                _ => Vec::new(),
                                            }
                                        };
                                        eprintln!(
                                            "ZAROXI_COCKPIT_SYMBOLS: recomputed n={} spans_version={} large_file={}",
                                            symbols.len(),
                                            self.latest_spans_version,
                                            self.large_file_mode,
                                        );
                                        self.cockpit_minimap_symbols = symbols;
                                        self.cockpit_symbols_version = self.latest_spans_version;
                                    }
                                    // Refresh git diff change markers when the
                                    // buffer version advances (per edit / on open).
                                    // The provider caches the baseline so git is
                                    // invoked at most once per file; per-edit cost
                                    // is the pure in-memory line diff. (An async
                                    // worker is the eventual home for the first
                                    // git lookup.)
                                    if self.editor_buffer.buffer_version
                                        != self.cockpit_diff_version
                                    {
                                        let hunks = if self.large_file_mode {
                                            Vec::new()
                                        } else if let Some(path) =
                                            self.committed_active_file.clone()
                                        {
                                            let current = self.editor_buffer.to_string();
                                            match self
                                                .git_diff_provider
                                                .diff_file(std::path::Path::new(&path), &current)
                                            {
                                                Some(fd) => fd
                                                    .changed_lines
                                                    .iter()
                                                    .map(|c| {
                                                        zaroxi_interface_widgets::components::DiffHunk {
                                                            line: c.line,
                                                            added: c.added,
                                                        }
                                                    })
                                                    .collect(),
                                                None => Vec::new(),
                                            }
                                        } else {
                                            Vec::new()
                                        };
                                        eprintln!(
                                            "ZAROXI_COCKPIT_DIFF: hunks={} buffer_version={} large_file={}",
                                            hunks.len(),
                                            self.editor_buffer.buffer_version,
                                            self.large_file_mode,
                                        );
                                        self.cockpit_diff_hunks = hunks;
                                        self.cockpit_diff_version =
                                            self.editor_buffer.buffer_version;
                                    }
                                    // ── Typed instrument-panel status model ──
                                    // Context + metadata come from the shared
                                    // presenter (`cockpit_context`/`cockpit_meta`);
                                    // health + AI bands are runtime telemetry.
                                    let ai_band = {
                                        use zaroxi_application_ai::view_model::AiPhase;
                                        let used = self.ai_session.tokens_streamed as u32;
                                        let mode = match self.ai_session.phase {
                                            AiPhase::Idle => {
                                                zaroxi_interface_widgets::AiMode::Dormant
                                            }
                                            AiPhase::PromptBuilding
                                            | AiPhase::Requesting
                                            | AiPhase::Streaming => {
                                                zaroxi_interface_widgets::AiMode::Live
                                            }
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
                                    let instrument_status =
                                        zaroxi_interface_widgets::InstrumentStatus {
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
                                        self.cockpit_symbols_version,
                                        self.cockpit_diff_version,
                                    );
                                    let fp_match = self.cockpit_text_active
                                        && fp == self.cockpit_status_fingerprint;
                                    self.cockpit_status_fingerprint = fp;
                                    if fp_match {
                                        // Cockpit unchanged — already built pre-render.
                                        let cockpit_bytes_est =
                                            self.cockpit_minimap_symbols.len().saturating_mul(64)
                                                + self.cockpit_diff_hunks.len().saturating_mul(32)
                                                + 1024;
                                        self.cockpit_retained_bytes = cockpit_bytes_est;
                                    } else {
                                        let inputs = super::cockpit::CockpitInputs {
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
                                                            glyph: char::from_u32(cp)
                                                                .unwrap_or('?'),
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
                                            line_height: 18.0,
                                            total_lines: editor_total_lines,
                                            // Live structural symbols (function/type/
                                            // import) from tree-sitter highlight spans
                                            // mapped to lines via the rope byte index.
                                            minimap_symbols: self.cockpit_minimap_symbols.clone(),
                                            // Live git change markers (added/modified/
                                            // removed) for the active file.
                                            diff_hunks: self.cockpit_diff_hunks.clone(),
                                            // Cursor-centered viewport band for the
                                            // minimap thumb, from live editor state.
                                            viewport: super::cockpit::cursor_viewport(
                                                cur_line,
                                                editor_total_lines,
                                            ),
                                            // Typed instrument-panel status model (the
                                            // three bands), built from the shared context
                                            // presenter + runtime health/AI telemetry.
                                            status: instrument_status,
                                            // prediction_cells / ai_regions remain empty:
                                            // there is no edit-prediction subsystem yet.
                                            settings_panel: {
                                                use super::destination::WorkbenchDestination as D;
                                                if D::from_rail_index(self.rail_selected_index)
                                                    == D::Settings
                                                {
                                                    let sections =
                                                        super::destination::settings_sections(
                                                            &format!("{:?}", self.theme_mode),
                                                        );
                                                    let sel = self
                                                        .settings_selected_section
                                                        .min(sections.len().saturating_sub(1));
                                                    Some((sections, sel))
                                                } else {
                                                    None
                                                }
                                            },
                                            extensions_panel: {
                                                use super::destination::WorkbenchDestination as D;
                                                if D::from_rail_index(self.rail_selected_index)
                                                    == D::Extensions
                                                {
                                                    let entries =
                                                        super::destination::extension_entries();
                                                    let sel = self
                                                        .extensions_selected_index
                                                        .min(entries.len().saturating_sub(1));
                                                    Some((entries, sel))
                                                } else {
                                                    None
                                                }
                                            },
                                            placeholder_panel:
                                                super::destination::WorkbenchDestination::from_rail_index(
                                                    self.rail_selected_index,
                                                )
                                                .placeholder(),
                                            ..Default::default()
                                        };
                                        let (scene, text) =
                                            super::cockpit::build_cockpit_frame(&inputs, &tokens);
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
                                            let slot_w =
                                                if count > 0 { rw / count as f32 } else { 0.0 };
                                            let mut rects = Vec::new();
                                            for i in 0..count {
                                                let sx = rx + i as f32 * slot_w;
                                                rects.push((sx, ry, slot_w, rh));
                                            }
                                            rects
                                        };
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
                                        let first_paint_ms =
                                            frame_start.elapsed().as_secs_f32() * 1000.0;
                                        eprintln!(
                                            "ZAROXI_STARTUP_TRACE: frame={} phase=first_paint total_ms={:.2} postpaint_misc_ms={:.2}",
                                            frame_id,
                                            first_paint_ms,
                                            postpaint_ms.max(0.0),
                                        );
                                    }
                                } else if startup_trace && frame_id == 1 {
                                    if let Some(reason) = &self.startup_second_layout_reason {
                                        eprintln!(
                                            "ZAROXI_STARTUP_TRACE: frame={} phase=second_layout reason={}",
                                            frame_id, reason,
                                        );
                                    }
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
                                    let open_complete =
                                        open_was_settling && perf.shaping_pending == 0;
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
                                    let has_syntax =
                                        !self.large_file_mode && self.latest_spans.is_some();
                                    let with_syntax = if has_syntax { screenful_ready } else { 0 };
                                    let without_syntax =
                                        screenful_ready.saturating_sub(with_syntax);
                                    let mode = if complete_visible {
                                        "screenful_complete"
                                    } else {
                                        "partial"
                                    };
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
                                    let legacy = super::cockpit::legacy_shell_surfaces();
                                    let cockpit_ready = self.cockpit_rendered_once;
                                    let geometry_stable = !self.resize_pending;
                                    let ready_to_show =
                                        (cockpit_ready || legacy) && geometry_stable;
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
                                            if let Some(ref core) = self.render_core {
                                                if let Some(tr) = core.text_renderer() {
                                                    let before_entries = tr.mem_shape_cache_bytes();
                                                    tr.evict_shaped_cold(256);
                                                    let after_entries = tr.mem_shape_cache_bytes();
                                                    if std::env::var("ZAROXI_MEM_TRACE").as_deref()
                                                        == Ok("1")
                                                    {
                                                        eprintln!(
                                                            "ZAROXI_MEM_TRACE: frame={} phase=post_settle_trim shape_cache_before_kb={} shape_cache_after_kb={}",
                                                            frame_id,
                                                            before_entries as usize / 1024,
                                                            after_entries as usize / 1024,
                                                        );
                                                    }
                                                }
                                            }
                                            let before_syntax = self.editor_retained_bytes;
                                            let visible_start = self
                                                .composition
                                                .as_ref()
                                                .and_then(|c| c.metadata.as_ref())
                                                .map(|m| m.editor_scroll_top_line)
                                                .unwrap_or(0);
                                            let visible_end =
                                                visible_start + editor_visible_lines + 40;
                                            self.line_syntax_cache.retain(|&(line, _), _| {
                                                line >= visible_start && line < visible_end
                                            });
                                            self.cached_line_hashes.truncate(visible_end);
                                            self.editor_retained_bytes = self
                                                .line_syntax_cache
                                                .iter()
                                                .map(|(_, v)| {
                                                    v.iter().map(|(s, _)| s.len()).sum::<usize>()
                                                })
                                                .sum::<usize>()
                                                + self.cached_line_hashes.len() * 8
                                                + self
                                                    .latest_spans
                                                    .as_ref()
                                                    .map(|s| s.len() * 32)
                                                    .unwrap_or(0);
                                            if std::env::var("ZAROXI_MEM_TRACE").as_deref()
                                                == Ok("1")
                                            {
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
