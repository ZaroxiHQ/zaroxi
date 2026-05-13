use winit::dpi::PhysicalSize;

/// Simple window state used by the runtime.
///
/// For v1 we store only the current size. This is a small separation
/// to make the runtime easier to expand later.
#[derive(Debug, Clone)]
pub struct WindowState {
    pub size: PhysicalSize<u32>,
}

impl WindowState {
    pub fn new(size: PhysicalSize<u32>) -> Self {
        Self { size }
    }
}
