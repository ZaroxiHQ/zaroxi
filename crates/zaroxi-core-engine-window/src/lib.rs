/*!
A minimal wrapper around winit's Window providing a small, explicit API
surface for the render backend.

This crate intentionally keeps its API small (create, query size, update size,
access the underlying Window) so the render backend can create surfaces and
drive presentation without depending on winit from many places.
*/

#![deny(missing_docs)]

// Use a module alias for the winit window module to avoid
// ambiguous import resolution across different winit/raw-window-handle
// versions and to match the actual public API surface in 0.30.13.
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::Window;

/// A thin handle to the native window used by the engine.
/// Thin handle storing the concrete winit Window type from the aliased module.
pub struct ZaroxiWindow {
    window: Window,
    width: u32,
    height: u32,
}

impl ZaroxiWindow {
    /// Create a new ZaroxiWindow bound to the supplied EventLoop.
    ///
    /// Window is configured:
    /// - title: "Zaroxi Studio"
    /// - initial size: 1400x900
    /// - resizable: true
    /// - transparent: false
    #[allow(deprecated)]
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        // winit 0.30.x removed WindowBuilder in favor of creating windows via the EventLoop
        // using WindowAttributes. Build attributes here and ask the EventLoop to create the window.
        let attrs = Window::default_attributes()
            .with_title("Zaroxi Studio".to_string())
            .with_inner_size(PhysicalSize::new(1400u32, 900u32))
            .with_resizable(true)
            .with_transparent(false);

        let window = event_loop.create_window(attrs).expect("failed to create window");

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        Self { window, width, height }
    }

    /// Return the cached size (width, height) in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Borrow the underlying winit Window for cases where the caller needs
    /// direct access (for example, creating a wgpu surface).
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Update the cached window size (driver code should call this on resize).
    /// Zero sizes are clamped to 1 to avoid wgpu / render panics on minimized windows.
    pub fn update_size(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
    }
}
