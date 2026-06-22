//! Frame scheduling: redraw coalescing and frame pacing.
//!
//! The event loop owns a single [`FrameScheduler`] that turns many small
//! invalidations (input, scroll, resize, content, syntax) into at most one
//! presented frame per pacing budget. It deliberately does **not** busy-wait:
//! when a frame is deferred for pacing, the loop sleeps with
//! `ControlFlow::WaitUntil(deadline)` until the budget elapses.
//!
//! The dirty flag itself lives on `GuiApp::needs_render`; this type owns only
//! the *pacing* and *coalescing* state (last present time, outstanding-redraw
//! bookkeeping, and the merged set of reasons for the pending frame).

use std::time::{Duration, Instant};

/// Default target cadence when `ZAROXI_TARGET_FPS` is unset (120 FPS ≈ 8.33 ms).
const DEFAULT_TARGET_FPS: f64 = 120.0;

/// Why the next frame is needed. Multiple reasons are merged into one frame so
/// a burst of events (e.g. a scroll flood) still produces a single redraw.
///
/// These map onto the per-element dirty reasons the retained UI nodes track:
/// `resize` → geometry dirty, `content` → content dirty, `style` → style/theme
/// dirty, `syntax` → syntax-highlight dirty, `cursor_selection` → cursor /
/// selection dirty, `scroll` → scroll-offset dirty. `input` is the generic
/// editing reason kept for callers that have not yet been split into a more
/// specific reason.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvalidationFlags {
    /// Keyboard / editing / caret movement.
    pub input: bool,
    /// Wheel / trackpad / scrollbar movement (scroll-offset dirty).
    pub scroll: bool,
    /// Window resize or scale-factor change (geometry dirty).
    pub resize: bool,
    /// Work-content / document / panel change (content dirty).
    pub content: bool,
    /// Background syntax-highlight result applied (syntax-highlight dirty).
    pub syntax: bool,
    /// Caret move / selection change with no content edit (cursor/selection dirty).
    pub cursor_selection: bool,
    /// Theme / style-token change (style/theme dirty).
    pub style: bool,
}

impl InvalidationFlags {
    /// Input/editing invalidation.
    pub fn input() -> Self {
        Self { input: true, ..Self::default() }
    }
    /// Scroll invalidation (scroll-offset dirty).
    pub fn scroll() -> Self {
        Self { scroll: true, ..Self::default() }
    }
    /// Resize / scale-factor invalidation (geometry dirty).
    pub fn resize() -> Self {
        Self { resize: true, ..Self::default() }
    }
    /// Content / document invalidation.
    pub fn content() -> Self {
        Self { content: true, ..Self::default() }
    }
    /// Syntax-highlight invalidation.
    pub fn syntax() -> Self {
        Self { syntax: true, ..Self::default() }
    }
    /// Cursor / selection invalidation (no content edit).
    pub fn cursor_selection() -> Self {
        Self { cursor_selection: true, ..Self::default() }
    }
    /// Theme / style-token invalidation.
    pub fn style() -> Self {
        Self { style: true, ..Self::default() }
    }

    /// Merge another set of reasons into this one.
    pub fn merge(&mut self, other: InvalidationFlags) {
        self.input |= other.input;
        self.scroll |= other.scroll;
        self.resize |= other.resize;
        self.content |= other.content;
        self.syntax |= other.syntax;
        self.cursor_selection |= other.cursor_selection;
        self.style |= other.style;
    }

    /// Compact label of the active reasons, for opt-in frame tracing.
    pub fn summary(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if self.input {
            parts.push("input");
        }
        if self.scroll {
            parts.push("scroll");
        }
        if self.resize {
            parts.push("resize");
        }
        if self.content {
            parts.push("content");
        }
        if self.syntax {
            parts.push("syntax");
        }
        if self.cursor_selection {
            parts.push("cursor_sel");
        }
        if self.style {
            parts.push("style");
        }
        if parts.is_empty() { "none".to_string() } else { parts.join("+") }
    }
}

/// Small, explicit redraw scheduler: coalesces invalidations and paces redraws
/// toward a target cadence without busy-waiting.
pub struct FrameScheduler {
    /// Minimum interval between presented frames (the frame budget).
    target_interval: Duration,
    /// When the last frame was presented (None before the first frame).
    last_present: Option<Instant>,
    /// Whether a winit redraw request is currently outstanding. Prevents
    /// re-issuing `request_redraw` while one is already queued.
    redraw_requested: bool,
    /// Merged reasons for the pending (not-yet-presented) frame.
    pending: InvalidationFlags,
}

impl FrameScheduler {
    /// Build a scheduler, honouring `ZAROXI_TARGET_FPS` (clamped to 1..=1000).
    pub fn new() -> Self {
        let fps = std::env::var("ZAROXI_TARGET_FPS")
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .filter(|f| f.is_finite() && *f >= 1.0 && *f <= 1000.0)
            .unwrap_or(DEFAULT_TARGET_FPS);
        Self {
            target_interval: Duration::from_secs_f64(1.0 / fps),
            last_present: None,
            redraw_requested: false,
            pending: InvalidationFlags::default(),
        }
    }

    /// Record a reason for the pending frame.
    pub fn note_reason(&mut self, reason: InvalidationFlags) {
        self.pending.merge(reason);
    }

    /// Whether at least one frame budget has elapsed since the last present,
    /// i.e. it is time to paint again.
    pub fn budget_elapsed(&self, now: Instant) -> bool {
        match self.last_present {
            None => true,
            Some(last) => now.duration_since(last) >= self.target_interval,
        }
    }

    /// The instant at which a deferred frame should be painted (never in the
    /// past relative to `now`).
    pub fn next_deadline(&self, now: Instant) -> Instant {
        match self.last_present {
            None => now,
            Some(last) => {
                let deadline = last + self.target_interval;
                if deadline > now { deadline } else { now }
            }
        }
    }

    /// Whether a winit redraw request is already outstanding.
    pub fn redraw_outstanding(&self) -> bool {
        self.redraw_requested
    }

    /// Mark that a winit redraw request has been issued.
    pub fn mark_redraw_requested(&mut self) {
        self.redraw_requested = true;
    }

    /// Clear the outstanding-redraw flag when a `RedrawRequested` arrives.
    pub fn on_redraw_received(&mut self) {
        self.redraw_requested = false;
    }

    /// Record that a frame was presented at `now`, clearing pacing state.
    pub fn on_frame_presented(&mut self, now: Instant) {
        self.last_present = Some(now);
        self.pending = InvalidationFlags::default();
    }

    /// Reasons accumulated for the pending frame (for opt-in tracing).
    pub fn pending_summary(&self) -> String {
        self.pending.summary()
    }

    /// The merged invalidation reasons accumulated for the pending frame.
    pub fn pending(&self) -> InvalidationFlags {
        self.pending
    }
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_coalesces_reasons() {
        let mut flags = InvalidationFlags::scroll();
        flags.merge(InvalidationFlags::input());
        flags.merge(InvalidationFlags::syntax());
        assert!(flags.input && flags.scroll && flags.syntax);
        assert!(!flags.resize && !flags.content);
        assert_eq!(flags.summary(), "input+scroll+syntax");
    }

    #[test]
    fn budget_not_elapsed_within_interval() {
        let mut sched = FrameScheduler::new();
        let t0 = Instant::now();
        // Force a known short interval for deterministic behaviour.
        sched.target_interval = Duration::from_millis(8);
        assert!(sched.budget_elapsed(t0), "first frame always allowed");
        sched.on_frame_presented(t0);
        assert!(!sched.budget_elapsed(t0 + Duration::from_millis(2)));
        assert!(sched.budget_elapsed(t0 + Duration::from_millis(8)));
    }

    #[test]
    fn deadline_is_last_present_plus_interval() {
        let mut sched = FrameScheduler::new();
        sched.target_interval = Duration::from_millis(8);
        let t0 = Instant::now();
        sched.on_frame_presented(t0);
        // Mid-window: deadline is in the future at last + interval.
        let now = t0 + Duration::from_millis(3);
        assert_eq!(sched.next_deadline(now), t0 + Duration::from_millis(8));
        // Past the window: deadline clamps to now.
        let later = t0 + Duration::from_millis(20);
        assert_eq!(sched.next_deadline(later), later);
    }

    #[test]
    fn outstanding_redraw_roundtrip() {
        let mut sched = FrameScheduler::new();
        assert!(!sched.redraw_outstanding());
        sched.mark_redraw_requested();
        assert!(sched.redraw_outstanding());
        sched.on_redraw_received();
        assert!(!sched.redraw_outstanding());
    }
}
