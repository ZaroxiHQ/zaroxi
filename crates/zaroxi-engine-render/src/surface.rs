use wgpu::SurfaceError;

/// Utility helpers for common surface error handling.
/// Kept small to avoid over-abstraction for v1.
pub fn should_recreate_surface(err: &SurfaceError) -> bool {
    matches!(err, SurfaceError::Lost | SurfaceError::Outdated)
}
