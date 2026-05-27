/*!
Small diagnostics helper for Phase 10.

Provides a stable, testable projection of LSP diagnostics for the active/open buffer.
This module intentionally hides the feature-flagged adapter detail and exposes a
compact API used by the harness and presenter.
*/

use crate::DesktopComposition;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// In-memory mock provider map used by harness and tests to exercise a
/// ready-state diagnostics path without a full LSP client. Keys are normalized
/// resource URIs (strings) and values are presenter-local Diagnostic vectors.
static MOCK_PROVIDER: Lazy<Mutex<HashMap<String, Vec<Diagnostic>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Whether an in-memory provider has been installed (registered) by the harness
/// or tests. This separates provider availability from the presence of an
/// entry for a specific URI: an installed provider may have zero diagnostics
/// for a given URI and should still be considered "ready".
static MOCK_PROVIDER_INSTALLED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

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

/// Provider state for diagnostics at the composition / snapshot level.
///
/// This is intentionally small: Disabled => feature flag off; Unavailable =>
/// no active buffer or other transient absence; Ready => provider present
/// (may still have zero counts).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderState {
    Disabled,
    Unavailable,
    Ready,
}

/// Compact diagnostics snapshot surfaced on DesktopComposition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticsSnapshot {
    pub provider: ProviderState,
    pub active_buffer: String,
    pub errors: u32,
    pub warnings: u32,
    pub infos: u32,
    pub hints: u32,
}

/// Register deterministic mock diagnostics for a URI. Tests and the harness
/// may call this to simulate a provider producing diagnostics for a given
/// active buffer. Passing an empty vector will clear any existing mock entry.
///
/// Behavior:
/// - non-empty `diags` inserts the entry and marks the in-memory provider as installed.
/// - empty `diags` removes the entry for the URI but does NOT mark the provider
///   as uninstalled (so provider availability is preserved until explicitly cleared).
pub fn register_mock_diagnostics(uri: &str, diags: Vec<Diagnostic>) {
    let mut map = MOCK_PROVIDER.lock().unwrap();
    if diags.is_empty() {
        map.remove(uri);
    } else {
        map.insert(uri.to_string(), diags);
        // Mark provider installed when a non-empty registration occurs.
        *MOCK_PROVIDER_INSTALLED.lock().unwrap() = true;
    }
}

/// Public ingestion API for externally-supplied diagnostics payloads.
///
/// This is the narrow provider boundary used by Phase 9E. Callers may provide
/// diagnostics entries for a given URI; the stored snapshot for that URI is
/// replaced atomically. Passing an empty vector clears any stored diagnostics
/// for the URI (removes the entry).
///
/// The current implementation writes into the same in-memory provider used by
/// tests/harness; later this function can be adapted to forward to a real
/// adapter or queue without affecting the presenter surface.
pub fn ingest_diagnostics_payload(uri: &str, diags: Vec<Diagnostic>) {
    // Reuse the existing mock registrar to mutate the in-memory provider map.
    register_mock_diagnostics(uri, diags)
}

/// Clear all registered mock diagnostics (test convenience).
///
/// Also clear the "installed" marker so tests can reset provider availability
/// to the initial state.
pub fn clear_mock_diagnostics() {
    MOCK_PROVIDER.lock().unwrap().clear();
    *MOCK_PROVIDER_INSTALLED.lock().unwrap() = false;
}

/// Return detailed diagnostics for a URI when the provider is ready.
///
/// - Returns Some(Vec<Diagnostic>) when a provider (in-memory mock or real adapter)
///   can supply diagnostics (may be empty to indicate zero-count ready state).
/// - Returns None when diagnostics are not available (feature disabled and no mock provider).
pub fn diagnostics_details_for_uri(uri: &str) -> Option<Vec<Diagnostic>> {
    let u = uri.trim();
    if u.is_empty() {
        // Treat empty uri as "no active buffer" -> return an empty ready set so callers
        // can render a coherent "none" state without confusing it with disabled.
        return Some(Vec::new());
    }

    // First, consult the in-memory mock provider.
    if let Some(v) = MOCK_PROVIDER.lock().unwrap().get(u) {
        return Some(v.clone());
    }

    // If an in-memory provider was installed (but has no entry for this URI),
    // report an empty ready set (provider present, zero diagnostics).
    if *MOCK_PROVIDER_INSTALLED.lock().unwrap() {
        return Some(Vec::new());
    }

    // If feature is disabled and no mock provider, report no provider available.
    #[cfg(not(feature = "use_core_lsp"))]
    {
        return None;
    }

    // When feature enabled, forward to collect_for_uri mapping.
    #[cfg(feature = "use_core_lsp")]
    {
        match collect_for_uri(u) {
            DiagnosticsSummary::Some(v) => Some(v),
            DiagnosticsSummary::None => Some(Vec::new()),
            DiagnosticsSummary::Disabled => None,
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

    // No active buffer case -> explicit none.
    if uri.trim().is_empty() || uri == "<none>" {
        return format!("diagnostics active_buffer={} -> none", uri);
    }

    // Use the compact diagnostics snapshot/provider boundary to determine visible state.
    match diagnostics_snapshot_for_uri(&uri) {
        Some(snap) => match snap.provider {
            ProviderState::Disabled => format!("lsp=disabled active_buffer={}", uri),
            ProviderState::Ready => format!(
                "lsp=ready active_buffer={} errors={} warnings={} infos={} hints={}",
                uri, snap.errors, snap.warnings, snap.infos, snap.hints
            ),
            ProviderState::Unavailable => format!("lsp=unavailable active_buffer={}", uri),
        },
        None => format!("diagnostics active_buffer={} -> none", uri),
    }
}

/// Compose a DiagnosticsSnapshot for a given resource URI.
///
/// Returns None when the provided URI is empty (caller should handle active-buffer absent).
pub fn diagnostics_snapshot_for_uri(uri: &str) -> Option<DiagnosticsSnapshot> {
    let u = uri.trim();
    if u.is_empty() {
        return None;
    }

    // First, consult the in-memory mock provider. This allows the harness and
    // tests to exercise a ready-state diagnostics path without a full LSP.
    if let Some(v) = MOCK_PROVIDER.lock().unwrap().get(u) {
        let mut errors = 0u32;
        let mut warnings = 0u32;
        let mut infos = 0u32;
        let mut hints = 0u32;
        for d in v.iter() {
            match d.severity {
                DiagnosticSeverity::Error => errors += 1,
                DiagnosticSeverity::Warning => warnings += 1,
                DiagnosticSeverity::Information => infos += 1,
                DiagnosticSeverity::Hint => hints += 1,
            }
        }
        return Some(DiagnosticsSnapshot {
            provider: ProviderState::Ready,
            active_buffer: u.to_string(),
            errors,
            warnings,
            infos,
            hints,
        });
    }

    // If an in-memory provider was installed but no entry exists for this URI,
    // treat the provider as present and return a zero-count Ready snapshot.
    if *MOCK_PROVIDER_INSTALLED.lock().unwrap() {
        return Some(DiagnosticsSnapshot {
            provider: ProviderState::Ready,
            active_buffer: u.to_string(),
            errors: 0,
            warnings: 0,
            infos: 0,
            hints: 0,
        });
    }

    // When the feature is not enabled and no mock provider has an entry,
    // report explicit Disabled provider state so the UI remains honest.
    #[cfg(not(feature = "use_core_lsp"))]
    {
        return Some(DiagnosticsSnapshot {
            provider: ProviderState::Disabled,
            active_buffer: u.to_string(),
            errors: 0,
            warnings: 0,
            infos: 0,
            hints: 0,
        });
    }

    // When feature enabled, forward to the core platform adapter and map types.
    #[cfg(feature = "use_core_lsp")]
    {
        match collect_for_uri(u) {
            DiagnosticsSummary::Disabled => Some(DiagnosticsSnapshot {
                provider: ProviderState::Disabled,
                active_buffer: u.to_string(),
                errors: 0,
                warnings: 0,
                infos: 0,
                hints: 0,
            }),
            DiagnosticsSummary::None => Some(DiagnosticsSnapshot {
                provider: ProviderState::Ready,
                active_buffer: u.to_string(),
                errors: 0,
                warnings: 0,
                infos: 0,
                hints: 0,
            }),
            DiagnosticsSummary::Some(v) => {
                let mut errors = 0u32;
                let mut warnings = 0u32;
                let mut infos = 0u32;
                let mut hints = 0u32;
                for d in v.iter() {
                    match d.severity {
                        DiagnosticSeverity::Error => errors += 1,
                        DiagnosticSeverity::Warning => warnings += 1,
                        DiagnosticSeverity::Information => infos += 1,
                        DiagnosticSeverity::Hint => hints += 1,
                    }
                }
                Some(DiagnosticsSnapshot {
                    provider: ProviderState::Ready,
                    active_buffer: u.to_string(),
                    errors,
                    warnings,
                    infos,
                    hints,
                })
            }
        }
    }
}

/// Expose a DesktopComposition-level accessor so consumers (harness/presenter/tests)
/// can read a compact DiagnosticsSnapshot directly from the composition.
///
/// Returns None when there is no active buffer to summarize.
impl DesktopComposition {
    pub fn latest_diagnostics_snapshot(&self) -> Option<DiagnosticsSnapshot> {
        let maybe_active = self.latest_active_buffer_details();
        let uri = if let Some(ref abd) = maybe_active {
            abd.display.clone().unwrap_or_else(|| abd.buffer_id.to_string())
        } else {
            return None;
        };

        diagnostics_snapshot_for_uri(&uri)
    }
}
