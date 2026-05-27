#![allow(clippy::unwrap_used)]
use zaroxi_interface_desktop::diagnostics::{
    collect_for_uri, diagnostics_snapshot_for_uri, DiagnosticsSummary, DiagnosticsSnapshot,
    ProviderState, register_mock_diagnostics, clear_mock_diagnostics, Diagnostic,
    DiagnosticSeverity,
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

/// Phase 9C tests: exercise the in-memory provider path so the composition can
/// surface a ready diagnostics snapshot with severity counts.

#[test]
fn ready_summary_reports_counts_for_active_buffer() {
    // Ensure a clean slate for mocks.
    clear_mock_diagnostics();

    // Register deterministic diagnostics for lib.rs: 1 error, 2 warnings, 1 info
    register_mock_diagnostics(
        "lib.rs",
        vec![
            Diagnostic { message: "E: something broke".to_string(), severity: DiagnosticSeverity::Error, uri: Some("lib.rs".to_string()) },
            Diagnostic { message: "W: minor issue".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("lib.rs".to_string()) },
            Diagnostic { message: "W: another warning".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("lib.rs".to_string()) },
            Diagnostic { message: "I: consider this".to_string(), severity: DiagnosticSeverity::Information, uri: Some("lib.rs".to_string()) },
        ],
    );

    let snap = diagnostics_snapshot_for_uri("lib.rs");
    match snap {
        Some(s) => {
            assert_eq!(s.provider, ProviderState::Ready);
            assert_eq!(s.errors, 1);
            assert_eq!(s.warnings, 2);
            assert_eq!(s.infos, 1);
        }
        None => panic!("expected Some snapshot for lib.rs"),
    }

    // cleanup
    clear_mock_diagnostics();
}

#[test]
fn switching_active_buffer_changes_diagnostics_summary() {
    clear_mock_diagnostics();

    register_mock_diagnostics(
        "main.rs",
        vec![
            Diagnostic { message: "W: main warning".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("main.rs".to_string()) },
        ],
    );

    register_mock_diagnostics(
        "lib.rs",
        vec![
            Diagnostic { message: "E: lib error".to_string(), severity: DiagnosticSeverity::Error, uri: Some("lib.rs".to_string()) },
            Diagnostic { message: "W: lib warning".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("lib.rs".to_string()) },
        ],
    );

    let snap_main = diagnostics_snapshot_for_uri("main.rs");
    let snap_lib = diagnostics_snapshot_for_uri("lib.rs");

    assert!(matches!(snap_main, Some(a) if a.warnings == 1 && a.errors == 0));
    assert!(matches!(snap_lib, Some(b) if b.errors == 1 && b.warnings == 1));

    clear_mock_diagnostics();
}

#[test]
fn missing_snapshot_for_active_buffer_is_handled_cleanly() {
    // Ensure no mock present and call with a non-empty uri; should return Some(snapshot)
    clear_mock_diagnostics();
    let snap = diagnostics_snapshot_for_uri("unknown.rs");
    assert!(snap.is_some(), "expected Some snapshot even when no provider entry present");
}
