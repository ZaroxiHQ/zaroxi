/*!
In-app operational log capture for the bottom panel's **Output** tab.

The Zaroxi app already emits `log::info!/warn!/error!` from real lifecycle
points (terminal spawn/resize, file open/read, clipboard, folder picker, parse
pipeline, etc.). Nothing was capturing those records, so this module installs a
lightweight global [`log::Log`] implementation that appends each record to a
bounded ring buffer. The Output tab renders that buffer — a genuine event/log
stream distinct from the interactive shell in the Terminal tab.

The buffer handle ([`OutputLog`]) is cheap to clone (`Arc`), so the logger and
the GUI share the same ring. Installation is idempotent (guarded by a
`OnceLock`) because a process may only register one global logger.
*/

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use log::{Level, LevelFilter, Log, Metadata, Record};

/// Default number of retained log entries.
pub const DEFAULT_CAPACITY: usize = 1000;

/// A single captured log record.
#[derive(Debug, Clone)]
pub struct OutputEntry {
    /// Monotonic sequence number (stable ordering, dedupe-friendly).
    pub seq: u64,
    /// Milliseconds since the log was installed (app start).
    pub millis: u128,
    pub level: Level,
    pub target: String,
    pub message: String,
}

struct Ring {
    entries: VecDeque<OutputEntry>,
    cap: usize,
    seq: u64,
    start: Instant,
}

/// Cheap, cloneable handle to the shared Output ring buffer.
#[derive(Clone)]
pub struct OutputLog {
    inner: Arc<Mutex<Ring>>,
}

impl OutputLog {
    /// Create a fresh, unattached buffer (used by tests and headless builds).
    pub fn new(cap: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Ring {
                entries: VecDeque::with_capacity(cap.min(256)),
                cap: cap.max(1),
                seq: 0,
                start: Instant::now(),
            })),
        }
    }

    /// Append a record, evicting the oldest entry when at capacity.
    pub fn push(&self, level: Level, target: &str, message: String) {
        if let Ok(mut ring) = self.inner.lock() {
            let seq = ring.seq;
            ring.seq += 1;
            let millis = ring.start.elapsed().as_millis();
            if ring.entries.len() == ring.cap {
                ring.entries.pop_front();
            }
            ring.entries.push_back(OutputEntry {
                seq,
                millis,
                level,
                target: target.to_string(),
                message,
            });
        }
    }

    /// Number of retained entries.
    pub fn len(&self) -> usize {
        self.inner.lock().map(|r| r.entries.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total records ever recorded (including evicted ones).
    pub fn total(&self) -> u64 {
        self.inner.lock().map(|r| r.seq).unwrap_or(0)
    }

    /// Invoke `f` with the retained entries oldest→newest without cloning them.
    pub fn with_entries<R>(&self, f: impl FnOnce(&VecDeque<OutputEntry>) -> R) -> R {
        let ring = self.inner.lock().unwrap();
        f(&ring.entries)
    }

    /// Snapshot the retained entries (oldest→newest).
    pub fn snapshot(&self) -> Vec<OutputEntry> {
        self.inner.lock().map(|r| r.entries.iter().cloned().collect()).unwrap_or_default()
    }
}

impl Default for OutputLog {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

struct CaptureLogger {
    sink: OutputLog,
}

impl Log for CaptureLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Capture Info and above; Debug/Trace stay out of the user-facing pane.
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        self.sink.push(record.level(), record.target(), format!("{}", record.args()));
    }

    fn flush(&self) {}
}

static GLOBAL: OnceLock<OutputLog> = OnceLock::new();

/// Install the global capture logger (idempotent) and return the shared handle.
///
/// Safe to call from every process entry point: the first call registers the
/// logger and sets the max level; later calls return the same handle. If some
/// other logger was already installed, capture still works via the returned
/// handle for anything routed through it (the GUI reads this handle directly).
pub fn install() -> OutputLog {
    GLOBAL
        .get_or_init(|| {
            let sink = OutputLog::new(DEFAULT_CAPACITY);
            let logger: &'static CaptureLogger =
                Box::leak(Box::new(CaptureLogger { sink: sink.clone() }));
            if log::set_logger(logger).is_ok() {
                log::set_max_level(LevelFilter::Info);
            }
            sink
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_evicts_oldest_at_capacity() {
        let log = OutputLog::new(3);
        for i in 0..5 {
            log.push(Level::Info, "test", format!("msg{i}"));
        }
        assert_eq!(log.len(), 3);
        assert_eq!(log.total(), 5);
        let snap = log.snapshot();
        assert_eq!(snap.first().unwrap().message, "msg2");
        assert_eq!(snap.last().unwrap().message, "msg4");
        // Sequence numbers keep counting past evictions.
        assert_eq!(snap.last().unwrap().seq, 4);
    }

    #[test]
    fn empty_by_default() {
        let log = OutputLog::new(10);
        assert!(log.is_empty());
        assert_eq!(log.total(), 0);
    }
}
