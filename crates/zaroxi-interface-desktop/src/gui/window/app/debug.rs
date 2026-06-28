/*!
Shared debug/trace helpers used across the app module and its children.
*/

pub(crate) fn gui_debug(msg: &str) {
    if std::env::var("ZAROXI_DEBUG_GUI").as_deref() == Ok("1") {
        eprintln!("{}", msg);
    }
}

pub(crate) fn event_label(event: &winit::event::WindowEvent) -> String {
    use winit::event::WindowEvent;
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            format!("CursorMoved({:.0},{:.0})", position.x, position.y)
        }
        WindowEvent::MouseInput { state, button, .. } => {
            format!("MouseInput({:?},{:?})", state, button)
        }
        WindowEvent::MouseWheel { .. } => "MouseWheel".into(),
        WindowEvent::RedrawRequested => "RedrawRequested".into(),
        WindowEvent::Resized(s) => format!("Resized({}x{})", s.width, s.height),
        WindowEvent::ScaleFactorChanged { .. } => "ScaleFactorChanged".into(),
        WindowEvent::CursorEntered { .. } => "CursorEntered".into(),
        WindowEvent::CursorLeft { .. } => "CursorLeft".into(),
        WindowEvent::Focused(f) => format!("Focused({})", f),
        WindowEvent::CloseRequested => "CloseRequested".into(),
        WindowEvent::ModifiersChanged(_) => "ModifiersChanged".into(),
        WindowEvent::Occluded(b) => format!("Occluded({})", b),
        WindowEvent::ThemeChanged(_) => "ThemeChanged".into(),
        WindowEvent::Touch(_) => "Touch".into(),
        WindowEvent::PinchGesture { .. } => "PinchGesture".into(),
        other => format!("other({})", variant_name(other)),
    }
}

fn variant_name(_ev: &winit::event::WindowEvent) -> &'static str {
    "unknown"
}

pub(crate) fn click_trace(msg: &str) {
    if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
        eprintln!("{}", msg);
    }
}

macro_rules! click_trace_fmt {
    ($($arg:tt)*) => {
        if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
            eprintln!($($arg)*);
        }
    };
}
pub(crate) use click_trace_fmt;

macro_rules! gui_debug_fmt {
    ($($arg:tt)*) => {
        if std::env::var("ZAROXI_DEBUG_GUI").as_deref() == Ok("1") {
            eprintln!($($arg)*);
        }
    };
}
pub(crate) use gui_debug_fmt;

pub(crate) fn zft_enabled() -> bool {
    std::env::var("ZAROXI_FILE_TABS").as_deref() == Ok("1")
}

pub(crate) fn zft(tag: &str, args: std::fmt::Arguments<'_>) {
    if zft_enabled() {
        eprintln!("ZAROXI_FILE_TABS: {tag} {args}");
    }
}

// ── Trace-flag helpers (consolidated diagnostics) ──

pub(crate) fn render_trace_enabled() -> bool {
    std::env::var("ZAROXI_RENDER_TRACE").as_deref() == Ok("1")
}

pub(crate) fn frame_trace_enabled() -> bool {
    std::env::var("ZAROXI_FRAME_TRACE").as_deref() == Ok("1")
}

pub(crate) fn scroll_trace_enabled() -> bool {
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

/// Whether the first-open materialization trace is enabled
/// (`ZAROXI_DEBUG_FIRST_OPEN=1`). Drives the shared first-open contract
/// diagnostics: activation source (explorer vs tab), file kind (normal vs
/// large), rope/window materialization, editor_data text/line length, parse
/// request length/version, parse application, and explorer subtree rebuild
/// reason. Temporary; guarded so it is zero-cost when disabled.
pub(crate) fn first_open_trace_enabled() -> bool {
    std::env::var("ZAROXI_DEBUG_FIRST_OPEN").as_deref() == Ok("1")
}

/// Whether the document-lifecycle trace is enabled (`ZAROXI_DOC_LIFECYCLE=1`).
/// Drives the per-document state diagnostics: tab-switch check-in / check-out
/// (whether in-memory edited state was retained instead of reloaded from disk),
/// dirty-flag transitions, undo/redo push/pop, save start/success/failure, and
/// which document a shortcut targeted. Temporary; zero-cost when disabled.
pub(crate) fn doc_lifecycle_trace_enabled() -> bool {
    std::env::var("ZAROXI_DOC_LIFECYCLE").as_deref() == Ok("1")
}

/// Whether the editor-decoration trace is enabled (`ZAROXI_DEBUG_DECORATION=1`).
/// Drives the per-frame line-background/decoration diagnostics: which layer
/// emitted a row band, its document line + visual row, the decoration source
/// kind, active file, buffer version, visible range, and whether it was kept or
/// dropped (and why). Temporary; zero-cost when disabled.
pub(crate) fn decoration_trace_enabled() -> bool {
    std::env::var("ZAROXI_DEBUG_DECORATION").as_deref() == Ok("1")
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

// ── Operator diagnostics dumps (Ctrl+Shift+P / Ctrl+Shift+M) ──

use super::{GUI_FRAME_COUNTER, GuiApp};
use std::sync::atomic::Ordering;

impl GuiApp {
    /// Print a consolidated, human-readable performance dashboard across all
    /// observability subsystems (memory pressure, multi-buffer activity, cache
    /// footprint). Bound to Ctrl+Shift+P. The fine-grained per-event TS/AI/LSP
    /// latency data streams inline as `ZAROXI_*_TRACE` lines; this is the
    /// at-a-glance snapshot.
    pub fn dashboard(&self) {
        let frame = GUI_FRAME_COUNTER.load(Ordering::Relaxed);
        let rss_now = zaroxi_core_telemetry::read_rss_bytes().unwrap_or(0);
        let vsz_now = zaroxi_core_telemetry::read_vsz_bytes().unwrap_or(0);
        eprintln!("==================== ZAROXI PERFORMANCE DASHBOARD ====================");
        eprintln!(
            "  memory   : rss={:.1} MB  vsz={:.1} MB  (live snapshot from /proc/self)",
            rss_now as f64 / (1024.0 * 1024.0),
            vsz_now as f64 / (1024.0 * 1024.0),
        );
        match &self.last_mem_sample {
            Some(s) => eprintln!(
                "  last_sample (frame {}): pressure={}  shape_cache={} KB ({} entries)  atlas_entries={}  gpu={} KB  rope={} KB  open_docs={}  total_lines={}",
                frame,
                s.pressure,
                s.shape_cache_bytes / 1024,
                s.shape_cache_entries,
                s.atlas_entries,
                s.gpu_bytes / 1024,
                s.rope_bytes / 1024,
                s.open_docs,
                s.total_lines,
            ),
            None => eprintln!(
                "  last_sample: (no sample yet \u{2014} sample every {} frames)",
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
        if let Some(ref comp) = self.composition {
            if let Some(ref meta) = comp.metadata {
                eprintln!(
                    "  composit : opened_buffer_count={}  active_buffer={}",
                    meta.opened_buffer_count,
                    meta.active_buffer
                        .as_ref()
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "<none>".to_string()),
                );
            }
            if let Some(ref tr) = self.render_core.as_ref().and_then(|rc| rc.text_renderer()) {
                let shape_count = tr.shape_cache_entries();
                let atlas_count = tr.atlas_entry_count();
                eprintln!(
                    "  renderer : shape_cache_entries={}  atlas_glyph_entries={}",
                    shape_count, atlas_count,
                );
            }
        }
        eprintln!("  latency  : per-event TS / AI / LSP timings stream as ZAROXI_TS_TRACE,");
        eprintln!("             ZAROXI_AI_TRACE, ZAROXI_LSP_TRACE (set ZAROXI_PERF_TRACE=1)");
        eprintln!("=====================================================================");
    }

    /// Dedicated memory report (Ctrl+Shift+M) with per-subsystem breakdown
    /// and configured limits.
    pub fn memory_report(&self) {
        let rss = zaroxi_core_telemetry::read_rss_bytes().unwrap_or(0);
        let vsz = zaroxi_core_telemetry::read_vsz_bytes().unwrap_or(0);
        let rss_mb = rss as f64 / (1024.0 * 1024.0);
        let vsz_mb = vsz as f64 / (1024.0 * 1024.0);

        eprintln!("==================== ZAROXI MEMORY REPORT ====================");
        eprintln!(
            "  system   : rss={:.1} MB  vsz={:.1} MB  ratio={:.1}%",
            rss_mb,
            vsz_mb,
            if vsz > 0 { (rss as f64 / vsz as f64) * 100.0 } else { 0.0 }
        );

        // Glyph caches
        if let Some(ref core) = self.render_core {
            if let Some(tr) = core.text_renderer() {
                let shape_bytes = tr.mem_shape_cache_bytes();
                let shape_entries = tr.shape_cache_entries();
                let atlas_entries = tr.atlas_entry_count();
                let gpu_bytes = tr.mem_gpu_bytes();
                let shape_kb = shape_bytes / 1024;
                let gpu_kb = gpu_bytes / 1024;
                eprintln!(
                    "  glyphs   : line_shape_cache={} KB ({} entries)  atlas_glyph_cache={} entries  gpu_buffers={} KB",
                    shape_kb, shape_entries, atlas_entries, gpu_kb
                );
                eprintln!(
                    "            limits: atlas_cap=65536 entries  shape_cache_cap_lru=2048 lines"
                );
            }
        }

        // Rope / editor buffer
        let char_count = self.editor_buffer.char_count();
        let line_count = self.editor_buffer.line_count();
        let rope_kb = char_count as u64 / 1024;
        eprintln!(
            "  rope     : current_buffer_chars={}  current_buffer_lines={}  (~{} KB char data)",
            char_count, line_count, rope_kb
        );

        // Memory pressure
        let budget_mb = zaroxi_core_telemetry::DEFAULT_BUDGET_MB;
        let pressure = if budget_mb > 0 {
            zaroxi_core_telemetry::classify(
                rss,
                budget_mb * 1024 * 1024,
                zaroxi_core_telemetry::DEFAULT_ELEVATED_PCT,
                zaroxi_core_telemetry::DEFAULT_CRITICAL_PCT,
            )
        } else {
            zaroxi_core_telemetry::MemoryPressureLevel::Normal
        };
        eprintln!(
            "  pressure : level={}  budget={} MB  elevated_threshold=70%  critical_threshold=90%",
            pressure, budget_mb
        );

        // Opened buffers
        if let Some(ref comp) = self.composition {
            if let Some(ref meta) = comp.metadata {
                eprintln!(
                    "  buffers  : opened_count={}  active={}",
                    meta.opened_buffer_count,
                    meta.active_buffer
                        .as_ref()
                        .map(|b| b.to_string())
                        .unwrap_or_else(|| "<none>".to_string())
                );
            }
        }

        // Per-document costs
        if let Some(ref comp) = self.composition {
            if let Some(ref meta) = comp.metadata {
                let doc_count = meta.opened_buffer_count;
                if doc_count > 0 {
                    eprintln!("  per_doc  : {doc_count} open document(s):");
                    for item in meta.opened_buffers.iter() {
                        let display = &item.display;
                        let rss_delta = self.editor_buffer.char_count() as u64;
                        let lines = self.editor_buffer.line_count();
                        eprintln!(
                            "            doc[{}] {}: text_kb={} lines={} (single-buffer view)",
                            item.buffer_id,
                            display.as_deref().unwrap_or("<none>"),
                            rss_delta / 1024,
                            lines,
                        );
                    }
                    let est_overhead = self.editor_buffer.char_count() as f64 / 1024.0;
                    eprintln!(
                        "            total_doc_rope_kb={:.1}  (~{:.1} KB char data)",
                        est_overhead, est_overhead
                    );
                } else {
                    eprintln!("  per_doc  : no documents open");
                }
            }
        }

        // wgpu device state
        eprintln!(
            "  wgpu     : power_preference=None  limits=custom_reduced_{}MB_buffer_{}px_texture",
            128, 4096
        );
        eprintln!("            max_texture_dimension_2d=4096  max_buffer_size=128MB");

        // Trace hints
        eprintln!("  trace    : set ZAROXI_MEM_TRACE=1 for per-frame sampling");
        eprintln!("             set ZAROXI_PERF_TRACE=1 for all diagnostic streams");
        eprintln!("====================================================================");
    }
}
