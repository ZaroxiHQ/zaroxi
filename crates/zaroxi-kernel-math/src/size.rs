#![allow(dead_code)]
/// Size with width/height as f32.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    /// Create a new Size.
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Zero size.
    pub const fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}
