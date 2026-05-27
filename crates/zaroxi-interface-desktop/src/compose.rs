// Desktop UI surface: pure presenter/compositor for Phase 0.
//
// IMPORTANT: this module must not construct concrete adapters or import infrastructure.
// It accepts application-level traits and drives the UI-level scenario only.
//
// NOTE: The composition root (apps/zaroxi-desktop-harness) is the place that wires
// concrete adapters to application services. This compose module remains a pure
// interface surface and intentionally does not reference infrastructure crates.

/// Pure interface entrypoint used by an outer composition binary to exercise the first slice.
/// For Phase 0 this is intentionally a minimal, no-op presenter (keeps interface crate free of infra).
pub async fn run_desktop_flow() -> Result<(), String> {
    // The real composition and orchestration are performed in apps/zaroxi-desktop-harness.
    // Interface remains a pure presenter; this function is a stable, no-op entrypoint
    // that can be extended later to accept abstract presenters bound to application contracts.
    println!("Interface: run_desktop_flow invoked (no-op presenter for Phase 0)");
    Ok(())
}
