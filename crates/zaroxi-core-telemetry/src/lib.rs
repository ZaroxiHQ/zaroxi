#![doc = "zaroxi-core-telemetry: low-overhead telemetry primitives for core systems.\n\nThis crate provides tiny, allocation-conscious helpers for emitting metrics and traces from core loops. It intentionally avoids IO and heavy dependencies; adapters to real backends belong to infrastructure-* crates."]
#![deny(missing_docs)]

pub mod buffer_activity;
pub mod memory;

pub use buffer_activity::{BufferActivity, BufferActivityTracker};
pub use memory::{
    DEFAULT_BUDGET_MB, DEFAULT_CRITICAL_PCT, DEFAULT_ELEVATED_PCT, DEFAULT_SAMPLE_FRAMES,
    MemoryMonitor, MemoryPressureLevel, MemorySample, classify, mem_trace_enabled, read_rss_bytes,
    read_vsz_bytes,
};

/// Minimal telemetry API exposed to core layers.
pub mod api {
    /// A tiny metric counter (very small API surface).
    #[derive(Default, Debug)]
    pub struct Counter(u64);

    impl Counter {
        /// Increment the counter by `n`.
        pub fn incr(&mut self, n: u64) {
            self.0 = self.0.saturating_add(n);
        }

        /// Read the current value.
        pub fn value(&self) -> u64 {
            self.0
        }
    }
}
