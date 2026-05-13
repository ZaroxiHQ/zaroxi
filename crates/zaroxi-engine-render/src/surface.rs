/// Utility helpers for common surface error handling.
/// Kept small to avoid over-abstraction for v1.
pub fn should_recreate_surface(err: &wgpu::SurfaceError) -> bool {
    matches!(err, wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated)
}
