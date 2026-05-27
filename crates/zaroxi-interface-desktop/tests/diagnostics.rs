#![allow(clippy::unwrap_used)]
use zaroxi_interface_desktop::diagnostics::{
    collect_for_uri, diagnostics_snapshot_for_uri, DiagnosticsSnapshot, DiagnosticsSummary,
    ProviderState,
};
use zaroxi_interface_desktop::DesktopComposition;

#[test]
#[cfg(feature = "use_core_lsp")]
fn diagnostics_present_for_rs_when_feature_enabled() {
    match collect_for_uri("main.rs") {
        DiagnosticsSummary::Some(v) => {
            assert!(!v.is_empty(), "expected diagnostics for .rs file when feature enabled");
        }
        other => panic!("expected Some diagnostics, got {:?}", other),
    }
}

#[test]
#[cfg(feature = "use_core_lsp")]
fn diagnostics_absent_for_txt_when_feature_enabled() {
    match collect_for_uri("README.md") {
        DiagnosticsSummary::None => {}
        other => panic!("expected None for non-.rs file, got {:?}", other),
    }
}

#[test]
#[cfg(not(feature = "use_core_lsp"))]
fn diagnostics_disabled_when_feature_off() {
    match collect_for_uri("main.rs") {
        DiagnosticsSummary::Disabled => {}
        other => panic!("expected Disabled when use_core_lsp feature is off, got {:?}", other),
    }
}

#[test]
#[cfg(feature = "use_core_lsp")]
fn switching_active_buffers_updates_diagnostics() {
    // main.rs should produce diagnostics under the mock adapter
    let a = collect_for_uri("main.rs");
    assert!(matches!(a, DiagnosticsSummary::Some(_)));

    // lib.txt should not produce diagnostics
    let b = collect_for_uri("lib.txt");
    assert!(matches!(b, DiagnosticsSummary::None));
}

/// New tests for Phase 9B: composition-level snapshot and uri->snapshot helpers.

#[test]
fn composition_no_active_buffer_reports_none() {
    // Fresh composition has no active buffer details by default.
    let comp = DesktopComposition::new();
    assert!(comp.latest_diagnostics_snapshot().is_none(), "expected no diagnostics snapshot when no active buffer present");
}

#[test]
#[cfg(feature = "use_core_lsp")]
fn diagnostics_snapshot_for_uri_ready_and_counts() {
    // main.rs should produce a Ready snapshot with at least one warning from the mock adapter.
    let snap: Option<DiagnosticsSnapshot> = diagnostics_snapshot_for_uri("main.rs");
    match snap {
        Some(s) => {
            assert_eq!(s.provider, ProviderState::Ready);
            assert!(s.warnings >= 1, "expected at least one warning in mock diagnostics for main.rs");
        }
        None => panic!("expected Some snapshot for main.rs"),
    }
}

#[test]
#[cfg(not(feature = "use_core_lsp"))]
fn diagnostics_snapshot_for_uri_disabled_when_feature_off() {
    let snap = diagnostics_snapshot_for_uri("main.rs");
    match snap {
        Some(s) => {
            assert_eq!(s.provider, ProviderState::Disabled);
            assert_eq!(s.errors, 0);
            assert_eq!(s.warnings, 0);
        }
        None => panic!("expected Some snapshot carrying Disabled provider"),
    }
}

#[test]
#[cfg(feature = "use_core_lsp")]
fn switching_uri_snapshots_reflect_counts() {
    let snap_a = diagnostics_snapshot_for_uri("main.rs");
    let snap_b = diagnostics_snapshot_for_uri("README.md");

    assert!(matches!(snap_a, Some(a) if a.warnings >= 1));
    assert!(matches!(snap_b, Some(b) if b.errors == 0 && b.warnings == 0));
}
