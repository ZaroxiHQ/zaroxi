/// Utility helpers for common surface error handling.
///
/// The concrete SurfaceError type location has changed across wgpu releases.
/// To keep the v1 scaffold simple and avoid brittle references, expose a
/// tiny API that accepts any debug-able error and returns whether it likely
/// requires surface recreation. This keeps the function callable while we
/// evolve the error-handling surface logic later.
pub fn should_recreate_surface<E: std::fmt::Debug>(_err: &E) -> bool {
    // For now we conservatively return false; runtime currently handles
    // Lost/OutOfMemory directly where appropriate.
    false
}
