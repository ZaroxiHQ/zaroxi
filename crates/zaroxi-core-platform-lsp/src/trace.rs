//! LSP round-trip latency tracing (`ZAROXI_LSP_TRACE`).
//!
//! Each LSP operation (completion, hover, go-to-definition, diagnostics) is
//! traced separately with four phases:
//! - `lsp_request_sent_ms`: user trigger → request dispatched on the wire,
//! - `lsp_response_recv_ms`: dispatch → response received,
//! - `lsp_apply_ms`: applying the result (diagnostics/completions) to state,
//! - `lsp_total_round_trip_ms`: end-to-end (sum of the three).
//!
//! When the total exceeds a configurable threshold (default 500 ms,
//! `ZAROXI_LSP_SLOW_MS`) a `WARNING` variant of the line is emitted.

/// Default slow-LSP threshold in milliseconds.
pub const DEFAULT_SLOW_MS: f32 = 500.0;

/// LSP operation kind, traced separately so per-operation regressions show up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspMethod {
    /// `textDocument/completion`.
    Completion,
    /// `textDocument/hover`.
    Hover,
    /// `textDocument/definition`.
    Definition,
    /// `textDocument/publishDiagnostics` (server push).
    Diagnostics,
}

impl LspMethod {
    /// Stable lowercase tag for trace lines.
    pub fn as_str(&self) -> &'static str {
        match self {
            LspMethod::Completion => "completion",
            LspMethod::Hover => "hover",
            LspMethod::Definition => "definition",
            LspMethod::Diagnostics => "diagnostics",
        }
    }

    /// The JSON-RPC method string this operation dispatches.
    pub fn rpc_method(&self) -> &'static str {
        match self {
            LspMethod::Completion => "textDocument/completion",
            LspMethod::Hover => "textDocument/hover",
            LspMethod::Definition => "textDocument/definition",
            LspMethod::Diagnostics => "textDocument/publishDiagnostics",
        }
    }
}

/// Slow-LSP threshold (ms), honouring `ZAROXI_LSP_SLOW_MS`.
pub fn lsp_slow_threshold_ms() -> f32 {
    std::env::var("ZAROXI_LSP_SLOW_MS")
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .filter(|v| *v > 0.0)
        .unwrap_or(DEFAULT_SLOW_MS)
}

/// Whether `ZAROXI_LSP_TRACE=1` is set, or the umbrella `ZAROXI_PERF_TRACE=1`.
pub fn lsp_trace_enabled() -> bool {
    matches!(std::env::var("ZAROXI_LSP_TRACE").as_deref(), Ok("1"))
        || matches!(std::env::var("ZAROXI_PERF_TRACE").as_deref(), Ok("1"))
}

/// Per-operation LSP latency breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct LspTiming {
    /// Which operation this records.
    pub method: LspMethod,
    /// User trigger → request dispatch (ms).
    pub request_sent_ms: f32,
    /// Dispatch → response received (ms).
    pub response_recv_ms: f32,
    /// Applying the result to buffer/editor state (ms).
    pub apply_ms: f32,
}

impl LspTiming {
    /// Create a zeroed record for `method`.
    pub fn new(method: LspMethod) -> Self {
        Self { method, request_sent_ms: 0.0, response_recv_ms: 0.0, apply_ms: 0.0 }
    }

    /// End-to-end round-trip (sum of the three phases), in ms.
    pub fn total_round_trip_ms(&self) -> f32 {
        self.request_sent_ms + self.response_recv_ms + self.apply_ms
    }

    /// Whether the round-trip exceeded the (configurable) slow threshold.
    pub fn is_slow(&self) -> bool {
        self.total_round_trip_ms() >= lsp_slow_threshold_ms()
    }

    /// Render the canonical `ZAROXI_LSP_TRACE` line.
    pub fn format_line(&self) -> String {
        format!(
            "ZAROXI_LSP_TRACE: method={} lsp_request_sent_ms={:.2} lsp_response_recv_ms={:.2} lsp_apply_ms={:.2} lsp_total_round_trip_ms={:.2}",
            self.method.as_str(),
            self.request_sent_ms,
            self.response_recv_ms,
            self.apply_ms,
            self.total_round_trip_ms(),
        )
    }

    /// Render the `WARNING` line emitted when the round-trip is slow.
    pub fn format_warning_line(&self) -> String {
        format!(
            "ZAROXI_LSP_TRACE: WARNING slow_lsp method={} lsp_total_round_trip_ms={:.2} threshold_ms={:.1}",
            self.method.as_str(),
            self.total_round_trip_ms(),
            lsp_slow_threshold_ms(),
        )
    }

    /// Emit the trace line (and a WARNING line if slow) when tracing is enabled.
    pub fn emit(&self) {
        if lsp_trace_enabled() {
            eprintln!("{}", self.format_line());
            if self.is_slow() {
                eprintln!("{}", self.format_warning_line());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_is_sum() {
        let t = LspTiming {
            method: LspMethod::Hover,
            request_sent_ms: 1.0,
            response_recv_ms: 10.0,
            apply_ms: 2.0,
        };
        assert!((t.total_round_trip_ms() - 13.0).abs() < 1e-6);
    }

    #[test]
    fn slow_detection_and_warning() {
        let fast = LspTiming {
            method: LspMethod::Completion,
            request_sent_ms: 1.0,
            response_recv_ms: 5.0,
            apply_ms: 1.0,
        };
        assert!(!fast.is_slow());

        let slow = LspTiming {
            method: LspMethod::Completion,
            request_sent_ms: 1.0,
            response_recv_ms: 600.0,
            apply_ms: 1.0,
        };
        assert!(slow.is_slow());
        assert!(slow.format_warning_line().contains("WARNING slow_lsp method=completion"));
    }

    #[test]
    fn format_line_fields() {
        let line = LspTiming::new(LspMethod::Definition).format_line();
        assert!(line.starts_with("ZAROXI_LSP_TRACE: method=definition"));
        assert!(line.contains("lsp_request_sent_ms="));
        assert!(line.contains("lsp_total_round_trip_ms="));
    }
}
