#![allow(clippy::unwrap_used)]
use zaroxi_interface_desktop::diagnostics::{
    collect_for_uri, diagnostics_snapshot_for_uri, DiagnosticsSummary,
    ProviderState, register_mock_diagnostics, clear_mock_diagnostics, ingest_diagnostics_payload,
    Diagnostic, DiagnosticSeverity,
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

/// Phase 9E tests: ingestion API exercises small, honest external-update semantics.

#[test]
fn ingest_payload_updates_summary_for_active_buffer() {
    clear_mock_diagnostics();

    // Ingest a deterministic payload for `ingest.rs`: 1 error, 2 warnings.
    ingest_diagnostics_payload(
        "ingest.rs",
        vec![
            Diagnostic { message: "E: failure".to_string(), severity: DiagnosticSeverity::Error, uri: Some("ingest.rs".to_string()) },
            Diagnostic { message: "W: warn1".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("ingest.rs".to_string()) },
            Diagnostic { message: "W: warn2".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("ingest.rs".to_string()) },
        ],
    );

    let snap = diagnostics_snapshot_for_uri("ingest.rs").expect("expected snapshot after ingest");
    assert_eq!(snap.provider, ProviderState::Ready);
    assert_eq!(snap.errors, 1);
    assert_eq!(snap.warnings, 2);

    // cleanup
    clear_mock_diagnostics();
}

#[test]
#[cfg(not(feature = "use_core_lsp"))]
fn ingest_empty_payload_clears_summary_counts_feature_off() {
    clear_mock_diagnostics();

    register_mock_diagnostics(
        "clear_test.xyz",
        vec![
            Diagnostic { message: "W: w".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("clear_test.xyz".to_string()) },
        ],
    );

    // Ingest empty payload -> clears entry
    ingest_diagnostics_payload("clear_test.xyz", Vec::new());

    let snap = diagnostics_snapshot_for_uri("clear_test.xyz").expect("expected snapshot after clear");
    // When feature is off and no mock entry, provider should be Disabled per contract.
    assert_eq!(snap.provider, ProviderState::Disabled);

    clear_mock_diagnostics();
}

#[test]
#[cfg(feature = "use_core_lsp")]
fn ingest_empty_payload_clears_summary_counts_feature_on() {
    clear_mock_diagnostics();

    register_mock_diagnostics(
        "clear_test.txt",
        vec![
            Diagnostic { message: "W: w".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("clear_test.txt".to_string()) },
        ],
    );

    // Ingest empty payload -> clears entry
    ingest_diagnostics_payload("clear_test.txt", Vec::new());

    let snap = diagnostics_snapshot_for_uri("clear_test.txt").expect("expected snapshot after clear");
    // With feature on, absent mock falls back to adapter; if adapter reports none,
    // we map to Ready with zero counts; assert counts are zero.
    assert_eq!(snap.errors + snap.warnings + snap.infos + snap.hints, 0);

    clear_mock_diagnostics();
}

#[test]
fn switching_active_buffer_reads_correct_uri_snapshot_after_updates() {
    clear_mock_diagnostics();

    ingest_diagnostics_payload(
        "a.rs",
        vec![
            Diagnostic { message: "W: a".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("a.rs".to_string()) },
        ],
    );

    ingest_diagnostics_payload(
        "b.rs",
        vec![
            Diagnostic { message: "E: b".to_string(), severity: DiagnosticSeverity::Error, uri: Some("b.rs".to_string()) },
            Diagnostic { message: "W: b".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("b.rs".to_string()) },
        ],
    );

    let snap_a = diagnostics_snapshot_for_uri("a.rs").expect("a.rs snapshot");
    let snap_b = diagnostics_snapshot_for_uri("b.rs").expect("b.rs snapshot");

    assert_eq!(snap_a.warnings, 1);
    assert_eq!(snap_a.errors, 0);

    assert_eq!(snap_b.errors, 1);
    assert_eq!(snap_b.warnings, 1);

    clear_mock_diagnostics();
}

#[test]
fn updating_one_uri_does_not_mutate_other_uri_snapshots() {
    clear_mock_diagnostics();

    ingest_diagnostics_payload(
        "one.rs",
        vec![
            Diagnostic { message: "W: one".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("one.rs".to_string()) },
        ],
    );

    ingest_diagnostics_payload(
        "two.rs",
        vec![
            Diagnostic { message: "W: two".to_string(), severity: DiagnosticSeverity::Warning, uri: Some("two.rs".to_string()) },
        ],
    );

    // Update only `one.rs`
    ingest_diagnostics_payload(
        "one.rs",
        vec![
            Diagnostic { message: "E: one now error".to_string(), severity: DiagnosticSeverity::Error, uri: Some("one.rs".to_string()) },
        ],
    );

    let snap_one = diagnostics_snapshot_for_uri("one.rs").expect("one.rs snapshot");
    let snap_two = diagnostics_snapshot_for_uri("two.rs").expect("two.rs snapshot");

    // one.rs should reflect updated counts
    assert_eq!(snap_one.errors, 1);
    assert_eq!(snap_one.warnings, 0);

    // two.rs should be unchanged
    assert_eq!(snap_two.warnings, 1);
    assert_eq!(snap_two.errors, 0);

    clear_mock_diagnostics();
}
