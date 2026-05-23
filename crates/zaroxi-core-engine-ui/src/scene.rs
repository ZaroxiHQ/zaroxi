/// Small, deterministic scene primitive produced by the engine UI composer.
///
/// This minimal primitive is intended for use by engine renderers that accept
/// simple rect-based draw commands. It keeps the API intentionally tiny for
/// Phase 3.
#[derive(Clone, Debug)]
pub struct RectPrimitive {
    /// Top-left X (pixels)
    pub x: f32,
    /// Top-left Y (pixels)
    pub y: f32,
    /// Width (pixels)
    pub width: f32,
    /// Height (pixels)
    pub height: f32,
    /// RGBA color with floats 0.0..1.0
    pub color: [f32; 4],
}

/// Lightweight constructor helper.
impl RectPrimitive {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) -> Self {
        Self { x, y, width, height, color }
    }
}
