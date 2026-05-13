use thiserror::Error;

/// Errors surfaced by the renderer crate.
///
/// Added explicit surface state variants so the runtime can react to
/// Lost/Outdated/Timeout/Occluded conditions reported by the renderer.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("wgpu error: {0}")]
    Wgpu(#[from] wgpu::Error),

    #[error("surface lost (needs reconfigure)")]
    SurfaceLost,

    #[error("surface outdated (needs reconfigure)")]
    SurfaceOutdated,

    #[error("surface timeout (skip frame)")]
    SurfaceTimeout,

    #[error("surface occluded (skip frame)")]
    SurfaceOccluded,

    #[error("surface validation error: {0}")]
    SurfaceValidation(String),

    #[error("surface unsupported")]
    SurfaceUnsupported,

    #[error("other: {0}")]
    Other(String),
}
