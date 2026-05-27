/*!
Small diagnostics helper for Phase 10.

Provides a stable, testable projection of LSP diagnostics for the active/open buffer.
This module intentionally hides the feature-flagged adapter detail and exposes a
compact API used by the harness and presenter.
*/

use crate::DesktopComposition;

/// Local severity model used by the presenter and tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Information => "info",
            DiagnosticSeverity::Hint => "hint",
        }
    }
}

/// Local diagnostic model consumed by the presenter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub uri: Option<String>,
}

/// Stable summary enum returned to callers so caller code can distinguish
/// disabled / none / some states explicitly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticsSummary {
    Disabled,
    None,
    Some(Vec<Diagnostic>),
}

/// Collect diagnostics for the given resource URI in a way that respects the
/// crate feature flag. When `use_core_lsp` is enabled we forward to the real
/// core-platform-lsp adapter; otherwise we return Disabled to make the UI path
/// explicitly observable when LSP is not available.
pub fn collect_for_uri(uri: &str) -> DiagnosticsSummary {
    let u = uri.trim();
    if u.is_empty() {
        return DiagnosticsSummary::None;
    }

    // When the feature is not enabled, report explicit disabled state.
    #[cfg(not(feature = "use_core_lsp"))]
    {
        return DiagnosticsSummary::Disabled;
    }

    // When feature enabled, forward to the core platform adapter and map types.
    #[cfg(feature = "use_core_lsp")]
    {
        use zaroxi_core_platform_lsp::session::{request_diagnostics, Diagnostic as CoreDiag};
        let raw: Vec<CoreDiag> = request_diagnostics(u);
        if raw.is_empty() {
            DiagnosticsSummary::None
        } else {
            let mapped = raw
                .into_iter()
                .map(|d| Diagnostic {
                    message: d.message,
                    severity: match d.severity {
                        zaroxi_core_platform_lsp::session::DiagnosticSeverity::Error => {
                            DiagnosticSeverity::Error
                        }
                        zaroxi_core_platform_lsp::session::DiagnosticSeverity::Warning => {
                            DiagnosticSeverity::Warning
                        }
                        zaroxi_core_platform_lsp::session::DiagnosticSeverity::Information => {
                            DiagnosticSeverity::Information
                        }
                        zaroxi_core_platform_lsp::session::DiagnosticSeverity::Hint => {
                            DiagnosticSeverity::Hint
                        }
                    },
                    uri: d.uri,
                })
                .collect();
            DiagnosticsSummary::Some(mapped)
        }
    }
}

/// Compose a compact, stable human-readable summary for the given DesktopComposition.
///
/// This prints a one-line summary intended for harness output like:
///   "diagnostics for main.rs -> errors=0 warnings=1 hints=0"
pub fn summarize_for_composition(composition: &DesktopComposition) -> String {
    // Derive uri from active buffer details when available.
    let maybe_active = composition.latest_active_buffer_details();
    let uri = if let Some(ref abd) = maybe_active {
        // Prefer explicit display name, fall back to buffer_id string representation.
        abd.display.clone().unwrap_or_else(|| abd.buffer_id.to_string())
    } else {
        "<none>".to_string()
    };

    match collect_for_uri(&uri) {
        DiagnosticsSummary::Disabled => format!("lsp=disabled active_buffer={}", uri),
        DiagnosticsSummary::None => format!("diagnostics active_buffer={} -> none", uri),
        DiagnosticsSummary::Some(v) => {
            let mut errors = 0usize;
            let mut warnings = 0usize;
            let mut infos = 0usize;
            let mut hints = 0usize;
            for d in &v {
                match d.severity {
                    DiagnosticSeverity::Error => errors += 1,
                    DiagnosticSeverity::Warning => warnings += 1,
                    DiagnosticSeverity::Information => infos += 1,
                    DiagnosticSeverity::Hint => hints += 1,
                }
            }
            format!(
                "diagnostics active_buffer={} -> errors={} warnings={} infos={} hints={}",
                uri, errors, warnings, infos, hints
            )
        }
    }
}
