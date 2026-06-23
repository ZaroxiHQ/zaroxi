//! Granular Tree-sitter timing instrumentation (`ZAROXI_TS_TRACE`).
//!
//! The top-level `ZAROXI_PERF_TRACE` line collapses all syntax work into a
//! single `syntax_ms`. This module breaks that number down per edit cycle and
//! per language so regressions in a specific phase (edit-apply, reparse, or
//! highlight) or a specific grammar (Rust / Nix / Markdown) become visible.
//!
//! Durations are produced at the source by the timed method variants on
//! [`crate::parser::SyntaxTree`] and [`crate::highlight::HighlightEngine`];
//! callers accumulate them into a [`TsTiming`] and call [`TsTiming::emit`] once
//! per edit cycle.

/// Per-edit-cycle Tree-sitter timing breakdown for a single document.
///
/// All `*_ms` fields are wall-clock milliseconds for the current edit cycle.
/// A high [`changed_ranges`](TsTiming::changed_ranges) count means Tree-sitter
/// considered a large region invalidated — a candidate for edit-scope
/// optimization.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TsTiming {
    /// Canonical language id of the document (e.g. `"rust"`, `"nix"`, `"markdown"`).
    pub language: String,
    /// Milliseconds to apply the `InputEdit` to the rope and the existing tree.
    pub edit_apply_ms: f32,
    /// Milliseconds for the actual Tree-sitter incremental reparse.
    pub reparse_ms: f32,
    /// Milliseconds for highlight-query execution.
    pub highlight_ms: f32,
    /// Number of changed ranges returned by `Tree::changed_ranges` after the
    /// reparse. High values indicate a large invalidation surface.
    pub changed_ranges: usize,
}

impl TsTiming {
    /// Create an empty timing record tagged with `language`.
    pub fn new(language: impl Into<String>) -> Self {
        Self { language: language.into(), ..Self::default() }
    }

    /// Total Tree-sitter CPU time across all phases (ms).
    pub fn total_ms(&self) -> f32 {
        self.edit_apply_ms + self.reparse_ms + self.highlight_ms
    }

    /// Render the canonical `ZAROXI_TS_TRACE` line for this record.
    pub fn format_line(&self) -> String {
        format!(
            "ZAROXI_TS_TRACE: lang={} ts_edit_apply_ms={:.2} ts_reparse_ms={:.2} ts_highlight_ms={:.2} ts_changed_ranges={} ts_total_ms={:.2}",
            self.language,
            self.edit_apply_ms,
            self.reparse_ms,
            self.highlight_ms,
            self.changed_ranges,
            self.total_ms(),
        )
    }

    /// Emit the `ZAROXI_TS_TRACE` line to stderr when tracing is enabled.
    ///
    /// No-op unless [`ts_trace_enabled`] returns `true`, so the formatting cost
    /// is only paid when tracing is requested.
    pub fn emit(&self) {
        if ts_trace_enabled() {
            eprintln!("{}", self.format_line());
        }
    }
}

/// Whether `ZAROXI_TS_TRACE=1` is set, or the umbrella `ZAROXI_PERF_TRACE=1` is
/// active (so enabling the consolidated perf trace also surfaces the per-language
/// Tree-sitter breakdown). Cheap env read.
pub fn ts_trace_enabled() -> bool {
    matches!(std::env::var("ZAROXI_TS_TRACE").as_deref(), Ok("1"))
        || matches!(std::env::var("ZAROXI_PERF_TRACE").as_deref(), Ok("1"))
}

#[cfg(test)]
mod tests {
    use super::TsTiming;

    #[test]
    fn total_is_sum_of_phases() {
        let t = TsTiming {
            language: "rust".to_string(),
            edit_apply_ms: 0.5,
            reparse_ms: 2.0,
            highlight_ms: 1.5,
            changed_ranges: 3,
        };
        assert!((t.total_ms() - 4.0).abs() < 1e-6);
    }

    #[test]
    fn format_line_has_all_fields() {
        let t = TsTiming::new("nix");
        let line = t.format_line();
        assert!(line.starts_with("ZAROXI_TS_TRACE: lang=nix"));
        assert!(line.contains("ts_edit_apply_ms="));
        assert!(line.contains("ts_reparse_ms="));
        assert!(line.contains("ts_highlight_ms="));
        assert!(line.contains("ts_changed_ranges=0"));
        assert!(line.contains("ts_total_ms="));
    }
}
