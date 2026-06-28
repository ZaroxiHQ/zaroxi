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
- `activation.rs`         — widget activation routing, action dispatch & explorer CTA
- `input.rs`              — keyboard interpretation & mouse-wheel normalisation
- `editor_interaction.rs` — cursor projection, selection & hit-testing
- `render_state.rs`       — content hashing, editor-data caching
- `debug.rs`              — shared debug/trace helpers + operator diagnostics dumps

Phase 61 (Coordinator split): the remaining monolith carved into single-
responsibility modules so `mod.rs` is coordinator-only:
- `lifecycle.rs`      — winit `ApplicationHandler`; thin `window_event` dispatch
- `redraw.rs`         — `RedrawRequested` render pipeline + frame/build metrics
- `navigation.rs`     — non-editor pointer routing, picker outcome, settings action
- `tabs.rs`           — workbench tab open/focus/close lifecycle
- `open_pipeline.rs`  — request→commit→read/open→parse→present/settle open flow
*/

mod activation;
pub(crate) mod background_open;
pub(crate) mod background_parse;
pub(crate) mod background_read;
pub(crate) mod debug;
mod editor_interaction;
mod input;
mod lifecycle;
mod navigation;
mod open_pipeline;
mod redraw;
mod render_schedule;
mod render_state;
mod tabs;
mod ui_nodes;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub use open_pipeline::OpenPresentation;
pub use redraw::WidgetTreeFingerprint;
pub use render_schedule::{FrameScheduler, InvalidationFlags};

// Re-export the consolidated trace/diagnostics helpers so existing call sites
// (`super::perf_trace_enabled()` in sibling modules, bare names in this module)
// resolve unchanged after the helpers moved into `debug.rs`.
pub(crate) use debug::{
    caret_trace_enabled, decoration_trace_enabled, doc_lifecycle_trace_enabled,
    file_open_trace_enabled, first_open_trace_enabled, frame_trace_enabled,
    open_present_trace_enabled, perf_event, perf_trace_enabled, pipeline_trace_enabled,
    render_trace_enabled, scroll_trace_enabled, settle_trace_enabled, syntax_trace_enabled,
};

use winit::window::WindowAttributes;

static GUI_FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Cadence for polling background work (parse results, folder picker) while the
/// UI is otherwise idle. Relaxed enough to avoid pinning a CPU core, tight
/// enough that results land promptly.
const BACKGROUND_POLL_INTERVAL: Duration = Duration::from_millis(8);

/// Half-period of the search-box caret blink. The caret is "on" for this long,
/// then "off" for this long, while the explorer search box holds focus.
const CARET_BLINK_INTERVAL_MS: u128 = 530;
const CARET_BLINK_INTERVAL: Duration = Duration::from_millis(530);
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
use zaroxi_domain_settings::Settings;
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
    /// Live settings state — the single source of truth for all user preferences.
    pub settings: Settings,
    /// Hit rects for interactive settings rows, set each frame from the cockpit
    /// layout. Used to route pointer events to settings actions.
    pub settings_hit_rects: Vec<zaroxi_interface_widgets::SettingsRowHit>,
    /// Dropdown open state for the settings panel — tracks which select
    /// dropdown (if any) is currently expanded.
    pub settings_dropdown: zaroxi_interface_widgets::SettingsDropdownState,
    /// Cached popup geometry, frozen when a dropdown opens. Prevents visual
    /// drift caused by frame-to-frame layout rounding of the editor region.
    pub cached_settings_popup: Option<zaroxi_interface_widgets::PopupMenu>,
    pub shift_held: bool,
    pub ctrl_held: bool,
    /// Whether the platform "command" modifier (Super/Cmd on macOS) is held.
    /// Edit shortcuts (Save/Undo/Redo/clipboard) accept either Ctrl or Cmd so
    /// the same bindings work on Linux/Windows (Ctrl) and macOS (Cmd).
    pub cmd_held: bool,
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
    /// Active file identity the cached editor data was shaped for. Ensures
    /// cross-file cache pollution is impossible — the cache is invalidated
    /// when the active document identity changes.
    pub cached_editor_active_file: Option<String>,
    pub layout_controller: ShellLayoutController,
    pub editor_viewport: Option<EditorViewport>,
    /// Visual-to-logical line mapping from the most recent editor content
    /// preparation.  Index = visual row (within the window), value = logical
    /// line index.  Used by hit-testing and cursor projection.
    pub editor_visual_to_logical: Vec<usize>,
    /// Characters per visual row from the most recent wrap pass.
    pub editor_chars_per_row: usize,
    /// Visual row offset within the wrapped content window where the
    /// scroll_top logical line begins.  Used for content_offset_y so the
    /// renderer positions the viewport correctly.
    pub editor_wrap_visual_offset: usize,
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
    /// The single canonical workbench tab state — the sole authority for the
    /// visible tab strip, active tab identity, and scroll position. No other
    /// structure may act as an alternative tab authority.
    pub tab_state: super::destination::WorkbenchTabState,
    /// Hit rects for destination sidebar rows (Extensions list / Settings
    /// categories), set each frame from the sidebar render. `(x, y, w, h)`.
    pub sidebar_row_hit_rects: Vec<(f32, f32, f32, f32)>,
    /// Hit rects for the unified tab strip (file + non-file tabs), set each
    /// frame from the cockpit tab-strip layout so clicks route to focus/close.
    pub tab_hit_rects: Vec<super::destination::WorkbenchTabHit>,
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
    ///
    /// Stored with **absolute** 0-based document line indices. The diff layer
    /// expects viewport-relative rows, so they are converted (and clipped to the
    /// visible window) every frame by `diff_hunks_to_viewport` — never fed
    /// raw, which would pin a stale band to a fixed screen row on scroll.
    pub cockpit_diff_hunks: Vec<zaroxi_interface_widgets::components::DiffHunk>,
    /// `editor_buffer.buffer_version` the `cockpit_diff_hunks` were computed for.
    pub cockpit_diff_version: u64,
    /// Background parse worker for off-thread tree-sitter parsing.
    pub parse_worker: Option<background_parse::BackgroundParseWorker>,
    /// Per-path document buffers; keyed by file path so the active tab's
    /// buffer can be looked up during render/edit/save.  Uses
    /// `DocumentBuffer` which wraps either `ropey::Rope` (small files)
    /// or `PieceTable` (large files) behind a common API.
    pub doc_buffers:
        std::collections::HashMap<String, zaroxi_core_editor_largefile::DocumentBuffer>,
    /// Authoritative per-document state for **inactive** normal (Rope-backed)
    /// editable files, keyed by canonical file path (the `buf:` prefix stripped).
    ///
    /// The single ACTIVE document's state lives in `editor_buffer`; every other
    /// open normal document is parked here with its full state — text, caret,
    /// selection, dirty baseline, and undo/redo history. On a tab switch the
    /// outgoing document is checked IN here and the incoming one is checked OUT
    /// into `editor_buffer`, so unsaved edits and history survive tab switching
    /// without ever reloading from disk. Large files use `doc_buffers` instead
    /// and are never parked here.
    pub open_documents: std::collections::HashMap<String, EditorBufferState>,
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
    /// Horizontal scroll offset for the unified tab strip is now owned by
    /// [`super::destination::WorkbenchTabState::scroll_offset`].
    /// Left overflow arrow hit rect `(x,y,w,h)` when the tab strip needs
    /// scrolling and there are tabs scrolled off the left edge.
    pub tab_arrow_left_rect: Option<(f32, f32, f32, f32)>,
    /// Right overflow arrow hit rect `(x,y,w,h)` when tabs are scrolled off
    /// the right edge.
    pub tab_arrow_right_rect: Option<(f32, f32, f32, f32)>,
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

/// Prefix a leading dot to the label of any file tab whose document is dirty.
/// Kept as a free function so it can be applied at render time over a disjoint
/// `tab_state` borrow without holding a whole-`self` borrow. `dirty_paths` holds
/// canonical paths with the `buf:` prefix already stripped.
pub(crate) fn annotate_tabs_dirty(
    mut tabs: Vec<super::destination::UnifiedTab>,
    dirty_paths: &std::collections::HashSet<String>,
) -> Vec<super::destination::UnifiedTab> {
    for tab in tabs.iter_mut() {
        if let super::destination::WorkbenchTabId::FileBuffer(bid) = &tab.id {
            let key = bid.strip_prefix("buf:").unwrap_or(bid);
            if dirty_paths.contains(key) {
                tab.title = format!("● {}", tab.title);
            }
        }
    }
    tabs
}

/// Pure scroll-follow math: given the caret's logical line, the current scroll
/// top (first visible logical line), the visible row count, and the total line
/// count, return the new scroll top that keeps the caret inside the viewport
/// with the smallest movement.
///
/// This is the single invariant behind `ensure_caret_visible`, factored out so
/// it can be unit-tested without a full `GuiApp`. It never returns a value that
/// would push the caret out of view, and at EOF it clamps to the last full
/// screen (never snapping back to line 0).
pub(crate) fn scroll_top_to_keep_caret_visible(
    caret_line: usize,
    cur_top: usize,
    visible: usize,
    total_lines: usize,
) -> usize {
    let visible = visible.max(1);
    let total = total_lines.max(1);
    let caret = caret_line.min(total - 1);
    let max_top = total.saturating_sub(visible);
    let new_top = if caret < cur_top {
        // Caret is above the window — scroll up so it becomes the first row.
        caret
    } else if caret >= cur_top + visible {
        // Caret is below the window — scroll down so it becomes the last row.
        caret + 1 - visible
    } else {
        // Already visible — do not move.
        cur_top
    };
    new_top.min(max_top)
}

/// Pure caret→visual-row projection: map a caret's logical line to the visual
/// row used by the renderer, honoring soft-wrap.
///
/// - When not wrapping (`chars_per_row == 0` or an empty window map), the caret
///   row is the absolute logical line (the renderer offsets it by the scroll
///   origin via `content_offset_y`).
/// - When wrapping, the row is the caret line's first visual row within the
///   window plus the wrapped sub-row implied by the visual column.
///
/// If the caret's logical line is not present in the window map (off-window or a
/// transiently stale map), it clamps to the nearest edge — the LAST row when the
/// caret is at/below the window, never snapping back to row 0. This is the single
/// tested rule the render path uses; it never invents a caret position the model
/// did not produce.
pub(crate) fn caret_visual_row(
    logical_cursor: usize,
    cursor_visual_col: usize,
    visual_to_logical: &[usize],
    chars_per_row: usize,
) -> usize {
    if visual_to_logical.is_empty() || chars_per_row == 0 {
        return logical_cursor;
    }
    let base = visual_to_logical.iter().position(|&ll| ll == logical_cursor).unwrap_or_else(|| {
        let first = visual_to_logical.first().copied().unwrap_or(0);
        if logical_cursor < first {
            // Caret above the window → first row.
            0
        } else {
            // Caret at/below the window, or a transiently incomplete map (e.g.
            // the just-created EOF line not yet in the windowed map): place it on
            // the row just past the last mapped line, advancing one row per
            // logical line beyond it. This NEVER snaps back to the old/top line,
            // so Enter-at-EOF always moves the caret downward.
            let last = visual_to_logical.last().copied().unwrap_or(0);
            let last_row = visual_to_logical.len().saturating_sub(1);
            last_row + logical_cursor.saturating_sub(last)
        }
    });
    let wrapped_rows_for_line =
        visual_to_logical.iter().filter(|&&ll| ll == logical_cursor).count().saturating_sub(1);
    base + (cursor_visual_col / chars_per_row.max(1)).min(wrapped_rows_for_line)
}

// ── Large-file thresholds ──

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
    ///
    /// Normal (Rope-backed) files track dirtiness on the authoritative
    /// `EditorBufferState`. Large files keep their canonical content in
    /// `doc_buffers` (PieceTable), whose `is_modified()` is the source of truth.
    pub fn document_modified(&self) -> bool {
        if self.large_file_mode
            && let Some(key) =
                self.committed_active_file.as_deref().map(|s| s.strip_prefix("buf:").unwrap_or(s))
            && let Some(db) = self.doc_buffers.get(key)
        {
            return db.is_modified();
        }
        self.editor_buffer.is_dirty()
    }

    /// The single shared post-move/post-edit invariant: keep the active caret's
    /// logical line inside the editor viewport by adjusting the scroll origin
    /// (`editor_scroll_top_line`) with the minimal movement.
    ///
    /// Called after every caret-affecting action (arrows, Home/End, newline,
    /// backspace/delete across lines, paste/cut, undo/redo, mouse-click
    /// reposition) via the shared editor redraw path. Uses the rope's
    /// authoritative line count so it is correct immediately after edits, with no
    /// dependence on the workspace projection's possibly-stale `line_count`.
    ///
    /// No-op for large-file mode (the piece-table viewport manages its own
    /// scroll) and before the editor viewport size is known. It writes the
    /// scroll origin directly (the single source of truth read by the renderer)
    /// and clears any queued wheel deltas so they cannot fight the caret-follow.
    pub(crate) fn ensure_caret_visible(&mut self) {
        if self.large_file_mode {
            return;
        }
        // Fallback visible-row count from the viewport content height, used only
        // if the renderer has not yet published `editor_viewport_line_count`.
        let vp_visible = self
            .editor_viewport
            .as_ref()
            .map(|vp| lc::visible_lines_from_region(vp.content_rect.3));
        let total_lines = self.editor_buffer.line_count().max(1);
        let caret_line = self.editor_buffer.caret_line();
        let trace = caret_trace_enabled();

        let mut changed = false;
        if let Some(comp) = self.composition.as_mut()
            && let Some(meta) = comp.metadata.as_mut()
        {
            // Prefer the renderer's published visible-row count so caret-follow
            // uses the exact same window height as the rendered visible range.
            let Some(visible) = meta.editor_viewport_line_count.or(vp_visible).map(|v| v.max(1))
            else {
                return;
            };
            let cur_top = meta.editor_scroll_top_line;
            let new_top =
                scroll_top_to_keep_caret_visible(caret_line, cur_top, visible, total_lines);
            changed = new_top != cur_top;
            if changed {
                meta.editor_scroll_top_line = new_top;
                meta.editor_scroll_px = new_top as f32 * lc::LINE_HEIGHT;
                // Drop queued wheel deltas so they don't override caret-follow.
                comp.pending_scroll_lines = 0;
                comp.pending_vscroll_px = 0.0;
            }
            if trace {
                eprintln!(
                    "ZAROXI_CARET_VIEWPORT: ensure_visible applied={} caret_line={} cur_top={} new_top={} visible={} total={}",
                    changed, caret_line, cur_top, new_top, visible, total_lines,
                );
            }
        }
        if changed {
            self.invalidate(InvalidationFlags::scroll());
        }
    }

    /// Set of canonical document paths (the `buf:` prefix stripped) that
    /// currently have unsaved edits, gathered from the single authoritative
    /// source for each document: the active `editor_buffer`, parked
    /// `open_documents` entries, and (for large files) `doc_buffers`. Computed
    /// once per frame before the render borrow so the tab strip can be annotated
    /// without holding a whole-`self` borrow.
    pub(crate) fn dirty_doc_paths(&self) -> std::collections::HashSet<String> {
        let mut dirty = std::collections::HashSet::new();
        if self.document_modified()
            && let Some(key) =
                self.committed_active_file.as_deref().map(|s| s.strip_prefix("buf:").unwrap_or(s))
        {
            dirty.insert(key.to_string());
        }
        for (key, doc) in &self.open_documents {
            if doc.is_dirty() {
                dirty.insert(key.clone());
            }
        }
        for (key, db) in &self.doc_buffers {
            if db.is_modified() {
                dirty.insert(key.clone());
            }
        }
        dirty
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
}

impl GuiApp {
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
}

#[cfg(test)]
mod caret_viewport_tests {
    use super::scroll_top_to_keep_caret_visible;

    // Window shows `visible` rows starting at `cur_top`; document has `total` lines.

    #[test]
    fn caret_already_visible_does_not_move() {
        // top=10, visible=20 → window [10,30); caret 15 is inside.
        assert_eq!(scroll_top_to_keep_caret_visible(15, 10, 20, 100), 10);
    }

    #[test]
    fn caret_below_window_scrolls_down_minimally() {
        // window [0,20); caret at line 25 → new top so caret is last row: 25+1-20=6.
        assert_eq!(scroll_top_to_keep_caret_visible(25, 0, 20, 100), 6);
    }

    #[test]
    fn caret_above_window_scrolls_up_to_caret() {
        // window [40,60); caret at line 12 → top becomes 12.
        assert_eq!(scroll_top_to_keep_caret_visible(12, 40, 20, 100), 12);
    }

    #[test]
    fn down_arrow_one_past_bottom_advances_one_line() {
        // window [0,20); caret moved to line 20 (one past bottom) → top=1.
        assert_eq!(scroll_top_to_keep_caret_visible(20, 0, 20, 100), 1);
    }

    #[test]
    fn eof_clamps_to_last_screen_never_zero() {
        // 100-line doc, 20 visible, caret at last line (99) → top=80 (last screen),
        // NOT 0. This is the regression: EOF movement must not snap to the top.
        assert_eq!(scroll_top_to_keep_caret_visible(99, 0, 20, 100), 80);
    }

    #[test]
    fn newline_at_eof_follows_caret_down() {
        // Just inserted a newline at EOF: total grew to 51, caret on the new last
        // line (50), window was [0,20) → top=31 so the new line is the last row.
        assert_eq!(scroll_top_to_keep_caret_visible(50, 0, 20, 51), 31);
    }

    #[test]
    fn document_shorter_than_viewport_stays_at_top() {
        // 5 lines, 20 visible → max_top=0; caret anywhere keeps top at 0.
        assert_eq!(scroll_top_to_keep_caret_visible(4, 0, 20, 5), 0);
    }

    #[test]
    fn caret_line_clamped_to_total_bounds() {
        // Defensive: caret_line beyond total is clamped to the last line.
        assert_eq!(scroll_top_to_keep_caret_visible(999, 0, 20, 30), 10);
    }

    #[test]
    fn zero_visible_is_treated_as_one_row() {
        // visible=0 must not panic / divide; treated as 1.
        assert_eq!(scroll_top_to_keep_caret_visible(50, 0, 0, 100), 50);
    }

    #[test]
    fn start_of_file_up_does_not_underflow() {
        // caret at line 0, already at top → stays 0 (no underflow).
        assert_eq!(scroll_top_to_keep_caret_visible(0, 0, 20, 100), 0);
    }
}

#[cfg(test)]
mod caret_projection_tests {
    use super::caret_visual_row;

    #[test]
    fn non_wrap_returns_absolute_line() {
        // No wrap (chars_per_row=0): the caret row is the absolute logical line;
        // the renderer offsets it by the scroll origin.
        assert_eq!(caret_visual_row(7, 0, &[], 0), 7);
    }

    #[test]
    fn enter_at_eof_maps_to_new_line_row() {
        // After Enter at EOF of "abc": rope has 2 lines, window map = [0, 1],
        // caret on the new line 1, col 0. The caret row must be 1 (the new line),
        // NOT 0 (the old line) — the reported regression.
        assert_eq!(caret_visual_row(1, 0, &[0, 1], 80), 1);
    }

    #[test]
    fn wrapped_line_caret_advances_to_correct_subrow() {
        // Window map: row 0=line0, 1=line1, 2/3/4=line2 (wrapped over 3 rows),
        // 5=line3. Caret on line 2 at visual col 25 with 10 chars/row → sub-row
        // 2 → base row 2 + 2 = 4.
        let map = [0, 1, 2, 2, 2, 3];
        assert_eq!(caret_visual_row(2, 25, &map, 10), 4);
    }

    #[test]
    fn caret_just_below_window_advances_one_row() {
        // Caret one logical line past the last mapped line → the next row down,
        // never the old/top line.
        assert_eq!(caret_visual_row(43, 0, &[40, 41, 42], 80), 3);
    }

    #[test]
    fn stale_map_eof_new_line_does_not_snap_to_old_line() {
        // THE regression: a transiently stale 1-entry window map ([0]) after
        // Enter-at-EOF must NOT resolve the new line (1) to row 0 (the old line).
        // It advances to row 1 (just below the old line).
        assert_eq!(caret_visual_row(1, 0, &[0], 80), 1);
    }

    #[test]
    fn caret_above_window_clamps_to_first_row() {
        // Caret above the window → row 0.
        assert_eq!(caret_visual_row(5, 0, &[40, 41, 42], 80), 0);
    }

    #[test]
    fn caret_at_col0_of_found_line_is_that_lines_first_row() {
        let map = [10, 11, 12, 13];
        assert_eq!(caret_visual_row(12, 0, &map, 80), 2);
    }
}
