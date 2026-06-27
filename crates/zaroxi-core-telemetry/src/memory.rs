//! Process memory-pressure monitoring (`ZAROXI_MEM_TRACE`).
//!
//! Zaroxi runs on memory-constrained hosts (e.g. 8 GB RAM + swap), so this
//! module provides a tiny, dependency-free monitor that:
//!
//! - samples process RSS from `/proc/self/statm` on Linux (no `libc`, no
//!   `unsafe`),
//! - classifies a [`MemoryPressureLevel`] (Normal / Elevated / Critical)
//!   against a configurable budget,
//! - carries the per-subsystem footprint the caller supplies (shaped-glyph
//!   cache, rope buffers, GPU buffers) in a [`MemorySample`], and
//! - formats a `ZAROXI_MEM_TRACE` line compatible with the existing trace
//!   format.
//!
//! The *response* to pressure (evicting cold cache entries at Elevated, an
//! emergency glyph-cache flush at Critical) is performed by the renderer; this
//! module only decides the level and reports it, keeping the core layer free of
//! render dependencies.

use std::fmt;

/// Default per-process memory budget in mebibytes, used to derive pressure
/// thresholds. Override with `ZAROXI_MEM_BUDGET_MB`.
pub const DEFAULT_BUDGET_MB: u64 = 4096;

/// Default fraction of the budget at which pressure becomes
/// [`MemoryPressureLevel::Elevated`]. Override with `ZAROXI_MEM_ELEVATED_PCT`.
pub const DEFAULT_ELEVATED_PCT: f32 = 0.70;

/// Default fraction of the budget at which pressure becomes
/// [`MemoryPressureLevel::Critical`]. Override with `ZAROXI_MEM_CRITICAL_PCT`.
pub const DEFAULT_CRITICAL_PCT: f32 = 0.90;

/// Default number of frames between RSS samples (~5s at 60 fps). Override with
/// `ZAROXI_MEM_SAMPLE_FRAMES`.
pub const DEFAULT_SAMPLE_FRAMES: u64 = 300;

/// Linux reports `statm` resident size in pages; standard page size on the
/// supported targets is 4 KiB. Reading `sysconf(_SC_PAGESIZE)` would require
/// `libc`/`unsafe`, which the workspace forbids, so this constant is assumed.
const PAGE_SIZE_BYTES: u64 = 4096;

/// Coarse memory-pressure classification. Ordered `Normal < Elevated < Critical`
/// so callers can compare with `>=` (e.g. `level >= MemoryPressureLevel::Elevated`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum MemoryPressureLevel {
    /// Below the elevated threshold; no action required.
    #[default]
    Normal,
    /// At/above the elevated threshold; cold caches should be evicted.
    Elevated,
    /// At/above the critical threshold; perform an emergency cache flush.
    Critical,
}

impl MemoryPressureLevel {
    /// Stable lowercase tag for trace lines.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryPressureLevel::Normal => "normal",
            MemoryPressureLevel::Elevated => "elevated",
            MemoryPressureLevel::Critical => "critical",
        }
    }
}

impl fmt::Display for MemoryPressureLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Read current process resident set size (RSS) in bytes.
///
/// Returns `None` on non-Linux targets or if `/proc/self/statm` is unreadable.
#[cfg(target_os = "linux")]
pub fn read_rss_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    // Fields: size resident shared text lib data dt — we want `resident`.
    let resident_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages.saturating_mul(PAGE_SIZE_BYTES))
}

/// Read current process resident set size (RSS) in bytes.
///
/// Returns `None` on non-Linux targets or if `/proc/self/statm` is unreadable.
#[cfg(not(target_os = "linux"))]
pub fn read_rss_bytes() -> Option<u64> {
    None
}

/// Read current process virtual memory size (VSZ) in bytes from
/// `/proc/self/status`. Returns `None` on non-Linux or if unreadable.
#[cfg(target_os = "linux")]
pub fn read_vsz_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(val) = line.strip_prefix("VmSize:") {
            // Format: "VmSize:  123456 kB"
            let kb: u64 = val.trim().split_whitespace().next()?.parse().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn read_vsz_bytes() -> Option<u64> {
    None
}

/// Classify RSS against a budget and the two threshold fractions.
pub fn classify(
    rss_bytes: u64,
    budget_bytes: u64,
    elevated_pct: f32,
    critical_pct: f32,
) -> MemoryPressureLevel {
    if budget_bytes == 0 {
        return MemoryPressureLevel::Normal;
    }
    let frac = rss_bytes as f64 / budget_bytes as f64;
    if frac >= critical_pct as f64 {
        MemoryPressureLevel::Critical
    } else if frac >= elevated_pct as f64 {
        MemoryPressureLevel::Elevated
    } else {
        MemoryPressureLevel::Normal
    }
}

/// Whether `ZAROXI_MEM_TRACE=1` is set, or the umbrella `ZAROXI_PERF_TRACE=1`.
pub fn mem_trace_enabled() -> bool {
    matches!(std::env::var("ZAROXI_MEM_TRACE").as_deref(), Ok("1"))
        || matches!(std::env::var("ZAROXI_PERF_TRACE").as_deref(), Ok("1"))
}

/// Frame-paced RSS sampler + pressure classifier.
///
/// The render loop calls [`MemoryMonitor::tick`] every frame; it returns `true`
/// only once per `sample_interval_frames`, at which point the caller reads RSS,
/// calls [`MemoryMonitor::evaluate`], assembles a [`MemorySample`] with the
/// per-subsystem footprints, reacts to the level, and emits the trace.
#[derive(Debug, Clone)]
pub struct MemoryMonitor {
    budget_bytes: u64,
    elevated_pct: f32,
    critical_pct: f32,
    sample_interval_frames: u64,
    frame: u64,
    last_level: MemoryPressureLevel,
}

impl MemoryMonitor {
    /// Build a monitor, honouring `ZAROXI_MEM_BUDGET_MB`,
    /// `ZAROXI_MEM_ELEVATED_PCT`, `ZAROXI_MEM_CRITICAL_PCT`, and
    /// `ZAROXI_MEM_SAMPLE_FRAMES` (each falling back to the `DEFAULT_*` consts).
    pub fn from_env() -> Self {
        let budget_mb = env_u64("ZAROXI_MEM_BUDGET_MB").unwrap_or(DEFAULT_BUDGET_MB).max(1);
        let elevated = env_f32("ZAROXI_MEM_ELEVATED_PCT").unwrap_or(DEFAULT_ELEVATED_PCT);
        let critical = env_f32("ZAROXI_MEM_CRITICAL_PCT").unwrap_or(DEFAULT_CRITICAL_PCT);
        let frames = env_u64("ZAROXI_MEM_SAMPLE_FRAMES").unwrap_or(DEFAULT_SAMPLE_FRAMES).max(1);
        Self {
            budget_bytes: budget_mb.saturating_mul(1024 * 1024),
            elevated_pct: elevated,
            critical_pct: critical,
            sample_interval_frames: frames,
            frame: 0,
            last_level: MemoryPressureLevel::Normal,
        }
    }

    /// Advance the frame counter; returns `true` when a sample is due this frame.
    pub fn tick(&mut self) -> bool {
        self.frame = self.frame.wrapping_add(1);
        self.frame % self.sample_interval_frames == 0
    }

    /// Classify `rss_bytes` against the configured budget/thresholds, caching
    /// and returning the level.
    pub fn evaluate(&mut self, rss_bytes: u64) -> MemoryPressureLevel {
        let level = classify(rss_bytes, self.budget_bytes, self.elevated_pct, self.critical_pct);
        self.last_level = level;
        level
    }

    /// The most recently evaluated pressure level.
    pub fn last_level(&self) -> MemoryPressureLevel {
        self.last_level
    }

    /// Configured budget in bytes.
    pub fn budget_bytes(&self) -> u64 {
        self.budget_bytes
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::from_env()
    }
}

/// A single memory observation, assembled by the caller from the monitor's RSS
/// reading plus per-subsystem footprints, ready to emit as `ZAROXI_MEM_TRACE`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemorySample {
    /// Process resident set size (bytes).
    pub rss_bytes: u64,
    /// Process virtual memory size (bytes).
    pub vsz_bytes: u64,
    /// Shaped-glyph (line) cache footprint (bytes).
    pub shape_cache_bytes: u64,
    /// Number of entries in the shaped-glyph line cache.
    pub shape_cache_entries: usize,
    /// Number of entries in the persistent atlas glyph cache.
    pub atlas_entries: usize,
    /// Combined rope-buffer footprint across all open documents (bytes).
    pub rope_bytes: u64,
    /// GPU buffer footprint we manage (atlas + instance buffers, bytes).
    pub gpu_bytes: u64,
    /// Number of open documents.
    pub open_docs: usize,
    /// Total line count across all open documents.
    pub total_lines: usize,
    /// Classified pressure level for this sample.
    pub pressure: MemoryPressureLevel,
}

impl MemorySample {
    /// Render the canonical `ZAROXI_MEM_TRACE` line.
    pub fn format_line(&self) -> String {
        format!(
            "ZAROXI_MEM_TRACE: rss_mb={:.1} vsz_mb={:.1} pressure={} shape_cache_kb={} shape_cache_entries={} atlas_entries={} rope_kb={} gpu_kb={} open_docs={} total_lines={}",
            self.rss_bytes as f64 / (1024.0 * 1024.0),
            self.vsz_bytes as f64 / (1024.0 * 1024.0),
            self.pressure,
            self.shape_cache_bytes / 1024,
            self.shape_cache_entries,
            self.atlas_entries,
            self.rope_bytes / 1024,
            self.gpu_bytes / 1024,
            self.open_docs,
            self.total_lines,
        )
    }

    /// Emit the `ZAROXI_MEM_TRACE` line when tracing is enabled.
    pub fn emit(&self) {
        if mem_trace_enabled() {
            eprintln!("{}", self.format_line());
        }
    }
}

/// Convenience: RSS in mebibytes, for MEM_STARTUP checkpoints.
pub fn rss_mb() -> f64 {
    read_rss_bytes().unwrap_or(0) as f64 / (1024.0 * 1024.0)
}

/// Whether `ZAROXI_MEM_STARTUP=1` is set.
pub fn startup_trace_enabled() -> bool {
    matches!(std::env::var("ZAROXI_MEM_STARTUP").as_deref(), Ok("1"))
}

fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|s| s.trim().parse::<u64>().ok())
}

fn env_f32(key: &str) -> Option<f32> {
    std::env::var(key).ok().and_then(|s| s.trim().parse::<f32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_thresholds() {
        let budget = 1000;
        assert_eq!(classify(500, budget, 0.7, 0.9), MemoryPressureLevel::Normal);
        assert_eq!(classify(700, budget, 0.7, 0.9), MemoryPressureLevel::Elevated);
        assert_eq!(classify(899, budget, 0.7, 0.9), MemoryPressureLevel::Elevated);
        assert_eq!(classify(900, budget, 0.7, 0.9), MemoryPressureLevel::Critical);
        assert_eq!(classify(5000, 0, 0.7, 0.9), MemoryPressureLevel::Normal);
    }

    #[test]
    fn level_ordering_allows_ge_compare() {
        assert!(MemoryPressureLevel::Critical >= MemoryPressureLevel::Elevated);
        assert!(MemoryPressureLevel::Elevated >= MemoryPressureLevel::Normal);
        assert!(MemoryPressureLevel::Normal < MemoryPressureLevel::Critical);
    }

    #[test]
    fn tick_fires_on_interval() {
        let mut m = MemoryMonitor::from_env();
        m.sample_interval_frames = 3;
        m.frame = 0;
        assert!(!m.tick()); // 1
        assert!(!m.tick()); // 2
        assert!(m.tick()); // 3
        assert!(!m.tick()); // 4
    }

    #[test]
    fn format_line_has_all_fields() {
        let s = MemorySample {
            rss_bytes: 2 * 1024 * 1024,
            vsz_bytes: 5 * 1024 * 1024,
            shape_cache_bytes: 4096,
            shape_cache_entries: 128,
            atlas_entries: 256,
            rope_bytes: 8192,
            gpu_bytes: 16384,
            open_docs: 3,
            total_lines: 12345,
            pressure: MemoryPressureLevel::Elevated,
        };
        let line = s.format_line();
        assert!(line.starts_with("ZAROXI_MEM_TRACE: rss_mb=2.0"));
        assert!(line.contains("vsz_mb=5.0"));
        assert!(line.contains("shape_cache_entries=128"));
        assert!(line.contains("atlas_entries=256"));
        assert!(line.contains("shape_cache_kb=4"));
        assert!(line.contains("rope_kb=8"));
        assert!(line.contains("gpu_kb=16"));
        assert!(line.contains("open_docs=3"));
        assert!(line.contains("total_lines=12345"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rss_is_readable_on_linux() {
        let rss = read_rss_bytes();
        assert!(rss.is_some(), "RSS should be readable on Linux");
        assert!(rss.unwrap() > 0);
    }
}
