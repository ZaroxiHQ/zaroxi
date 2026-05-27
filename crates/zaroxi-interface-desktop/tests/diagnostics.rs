#![allow(clippy::unwrap_used)]
use zaroxi_interface_desktop::diagnostics::{collect_for_uri, DiagnosticsSummary};

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
