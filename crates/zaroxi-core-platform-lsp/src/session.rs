/*!
Minimal LSP session surface (Phase-10 baseline).

This module provides a tiny, honest diagnostics baseline used by the
presenter for observable diagnostics during Phase 10. It intentionally
implements a small mock adapter inline to avoid any heavy JSON-RPC
plumbing while keeping the API shape ready for a real adapter later.
*/

#[allow(dead_code)]
pub struct Session;

/// Severity of a diagnostic message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    /// Stable, short string form used by presenters/tests.
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Information => "info",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

/// Small diagnostic model sufficient for presenter visibility and tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
    /// Optional source uri that produced the diagnostic.
    pub uri: Option<String>,
}

/// Simple trait representing an LSP-like adapter able to return diagnostics
/// for a given resource/URI. This is intentionally tiny and synchronous for
/// the Phase-10 baseline; a real adapter can implement the same trait and
/// be wired in later.
pub trait LspAdapter: Send + Sync {
    fn diagnostics_for_uri(&self, uri: &str) -> Vec<Diagnostic>;
}

/// A tiny builtin mock adapter used as the Phase-10 default. It emits a
/// deterministic diagnostic for Rust-like files and otherwise returns none.
/// Tests and harnesses may rely on this behaviour for predictable assertions.
struct MockAdapter;

impl LspAdapter for MockAdapter {
    fn diagnostics_for_uri(&self, uri: &str) -> Vec<Diagnostic> {
        if uri.is_empty() {
            return vec![];
        }

        // Deterministic, visible mock condition:
        // - If the uri ends with ".rs" we return a single example diagnostic.
        // - Otherwise, return no diagnostics.
        if uri.trim().ends_with(".rs") {
            vec![Diagnostic {
                message: format!("mock: example diagnostic for {}", uri),
                severity: DiagnosticSeverity::Warning,
                uri: Some(uri.to_string()),
            }]
        } else {
            vec![]
        }
    }
}

/// Request diagnostics for a given resource URI using the Phase-10 baseline
/// adapter implementation. For Phase-10 we use the builtin MockAdapter so
/// callers observe deterministic behaviour without additional wiring.
pub fn request_diagnostics(uri: &str) -> Vec<Diagnostic> {
    // Direct call into the builtin mock adapter for minimal baseline.
    MockAdapter.diagnostics_for_uri(uri)
}
