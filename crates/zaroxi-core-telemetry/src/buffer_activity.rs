//! Multi-buffer activity tracking (`ZAROXI_BUF_TRACE`).
//!
//! With several files open, per-document metrics matter: which buffer is *hot*
//! (receiving edits), *warm* (visible but idle), or *cold* (open but off-screen).
//! Cold buffers are the first candidates whose shaped-glyph cache entries should
//! be evicted under memory pressure.
//!
//! This is a pure, dependency-free tracker fed by the UI loop (open/close,
//! edit, active, visible). It reports totals and a cold-first eviction order,
//! and formats a `ZAROXI_BUF_TRACE` line compatible with the existing traces.

use std::collections::HashMap;

/// Number of frames since the last edit after which a buffer is no longer
/// considered "hot". Default tuned for ~60fps (≈1s). Caller may override via
/// [`BufferActivityTracker::with_hot_window`].
pub const DEFAULT_HOT_WINDOW_FRAMES: u64 = 60;

/// Activity classification for an open document.
///
/// Ordered `Hot < Warm < Cold` so the natural sort yields eviction priority in
/// reverse (Cold buffers sort highest → evicted first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BufferActivity {
    /// Active buffer that received an edit within the hot window.
    Hot,
    /// Visible on screen but not currently being edited.
    Warm,
    /// Open but not visible — first to have caches evicted.
    Cold,
}

impl BufferActivity {
    /// Stable lowercase tag for trace lines.
    pub fn as_str(&self) -> &'static str {
        match self {
            BufferActivity::Hot => "hot",
            BufferActivity::Warm => "warm",
            BufferActivity::Cold => "cold",
        }
    }
}

/// Per-document bookkeeping.
#[derive(Debug, Clone)]
struct BufferState {
    line_count: usize,
    last_edit_frame: Option<u64>,
    visible: bool,
}

/// Tracks open documents and classifies their activity for metrics + eviction.
#[derive(Debug, Clone, Default)]
pub struct BufferActivityTracker {
    buffers: HashMap<String, BufferState>,
    active: Option<String>,
    hot_window_frames: u64,
}

impl BufferActivityTracker {
    /// Create an empty tracker with the default hot window.
    pub fn new() -> Self {
        Self { hot_window_frames: DEFAULT_HOT_WINDOW_FRAMES, ..Self::default() }
    }

    /// Override the hot-window length (in frames).
    pub fn with_hot_window(mut self, frames: u64) -> Self {
        self.hot_window_frames = frames.max(1);
        self
    }

    /// Register (or update) an open document with its current line count.
    pub fn note_open(&mut self, id: impl Into<String>, line_count: usize) {
        let id = id.into();
        let entry = self
            .buffers
            .entry(id)
            .or_insert(BufferState { line_count, last_edit_frame: None, visible: false });
        entry.line_count = line_count;
    }

    /// Remove a closed document. Clears `active` if it referred to this id.
    pub fn note_close(&mut self, id: &str) {
        self.buffers.remove(id);
        if self.active.as_deref() == Some(id) {
            self.active = None;
        }
    }

    /// Record an edit to `id` at `frame` (also refreshes its line count).
    pub fn note_edit(&mut self, id: &str, frame: u64, line_count: usize) {
        if let Some(b) = self.buffers.get_mut(id) {
            b.last_edit_frame = Some(frame);
            b.line_count = line_count;
        }
    }

    /// Mark `id` as the active document.
    pub fn set_active(&mut self, id: impl Into<String>) {
        self.active = Some(id.into());
    }

    /// Replace the set of currently-visible document ids.
    pub fn set_visible<I, S>(&mut self, visible: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let want: std::collections::HashSet<String> =
            visible.into_iter().map(|s| s.as_ref().to_string()).collect();
        for (id, b) in self.buffers.iter_mut() {
            b.visible = want.contains(id);
        }
    }

    /// Classify a document's activity at `current_frame`.
    pub fn classify(&self, id: &str, current_frame: u64) -> Option<BufferActivity> {
        let b = self.buffers.get(id)?;
        let is_active = self.active.as_deref() == Some(id);
        let recently_edited = b
            .last_edit_frame
            .map(|f| current_frame.saturating_sub(f) <= self.hot_window_frames)
            .unwrap_or(false);
        Some(if is_active && recently_edited {
            BufferActivity::Hot
        } else if b.visible {
            BufferActivity::Warm
        } else {
            BufferActivity::Cold
        })
    }

    /// Document ids ordered for eviction: Cold first, then Warm, then Hot;
    /// within a class, least-recently-edited first.
    pub fn eviction_order(&self, current_frame: u64) -> Vec<String> {
        let mut ids: Vec<String> = self.buffers.keys().cloned().collect();
        ids.sort_by(|a, b| {
            let ca = self.classify(a, current_frame).unwrap_or(BufferActivity::Cold);
            let cb = self.classify(b, current_frame).unwrap_or(BufferActivity::Cold);
            // Cold (highest) should come first → reverse class ordering.
            cb.cmp(&ca).then_with(|| {
                let ea = self.buffers.get(a).and_then(|s| s.last_edit_frame).unwrap_or(0);
                let eb = self.buffers.get(b).and_then(|s| s.last_edit_frame).unwrap_or(0);
                ea.cmp(&eb)
            })
        });
        ids
    }

    /// Number of open documents.
    pub fn open_count(&self) -> usize {
        self.buffers.len()
    }

    /// Total line count across all open documents.
    pub fn total_lines(&self) -> usize {
        self.buffers.values().map(|b| b.line_count).sum()
    }

    /// Count of documents in each activity class at `current_frame`:
    /// `(hot, warm, cold)`.
    pub fn class_counts(&self, current_frame: u64) -> (usize, usize, usize) {
        let (mut hot, mut warm, mut cold) = (0, 0, 0);
        for id in self.buffers.keys() {
            match self.classify(id, current_frame) {
                Some(BufferActivity::Hot) => hot += 1,
                Some(BufferActivity::Warm) => warm += 1,
                _ => cold += 1,
            }
        }
        (hot, warm, cold)
    }

    /// Render the canonical `ZAROXI_BUF_TRACE` line.
    pub fn format_line(&self, current_frame: u64) -> String {
        let (hot, warm, cold) = self.class_counts(current_frame);
        format!(
            "ZAROXI_BUF_TRACE: open_docs={} total_lines={} hot={} warm={} cold={} active={}",
            self.open_count(),
            self.total_lines(),
            hot,
            warm,
            cold,
            self.active.as_deref().unwrap_or("<none>"),
        )
    }

    /// Emit the `ZAROXI_BUF_TRACE` line when tracing is enabled (reuses the
    /// memory-trace gate so one env var surfaces both observability streams).
    pub fn emit(&self, current_frame: u64) {
        if crate::memory::mem_trace_enabled() {
            eprintln!("{}", self.format_line(current_frame));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_hot_warm_cold() {
        let mut t = BufferActivityTracker::new().with_hot_window(60);
        t.note_open("a.rs", 100);
        t.note_open("b.rs", 200);
        t.note_open("c.rs", 300);
        t.set_visible(["a.rs", "b.rs"]);
        t.set_active("a.rs");
        t.note_edit("a.rs", 1000, 101);

        // active + recently edited at frame 1010 (<=60 window) => Hot
        assert_eq!(t.classify("a.rs", 1010), Some(BufferActivity::Hot));
        // visible, not edited => Warm
        assert_eq!(t.classify("b.rs", 1010), Some(BufferActivity::Warm));
        // not visible => Cold
        assert_eq!(t.classify("c.rs", 1010), Some(BufferActivity::Cold));
        // past hot window, active but stale => falls back to Warm (a is visible)
        assert_eq!(t.classify("a.rs", 2000), Some(BufferActivity::Warm));
    }

    #[test]
    fn totals_and_counts() {
        let mut t = BufferActivityTracker::new();
        t.note_open("a", 10);
        t.note_open("b", 20);
        t.note_open("c", 30);
        t.set_visible(["a"]);
        t.set_active("a");
        t.note_edit("a", 5, 11);
        assert_eq!(t.open_count(), 3);
        assert_eq!(t.total_lines(), 11 + 20 + 30);
        let (hot, warm, cold) = t.class_counts(10);
        assert_eq!((hot, warm, cold), (1, 0, 2));
    }

    #[test]
    fn eviction_is_cold_first() {
        let mut t = BufferActivityTracker::new();
        t.note_open("hot", 1);
        t.note_open("warm", 1);
        t.note_open("cold", 1);
        t.set_visible(["hot", "warm"]);
        t.set_active("hot");
        t.note_edit("hot", 100, 1);
        let order = t.eviction_order(110);
        assert_eq!(order.first().map(|s| s.as_str()), Some("cold"));
        assert_eq!(order.last().map(|s| s.as_str()), Some("hot"));
    }

    #[test]
    fn format_line_has_all_fields() {
        let mut t = BufferActivityTracker::new();
        t.note_open("main.rs", 42);
        t.set_active("main.rs");
        let line = t.format_line(0);
        assert!(line.starts_with("ZAROXI_BUF_TRACE: open_docs=1 total_lines=42"));
        assert!(line.contains("active=main.rs"));
    }
}
