use thiserror::Error;

/// Errors surfaced by the renderer crate.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("wgpu error: {0}")]
    Wgpu(#[from] wgpu::Error),
    #[error("surface unsupported")]
    SurfaceUnsupported,
    #[error("other: {0}")]
    Other(String),
}
