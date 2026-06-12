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
use std::sync::Arc;

use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::Window;

/// A thin handle to the native window used by the engine.
///
/// Stores the winit `Window` inside an `Arc` so that `wgpu::Surface<'static>`
/// can be created safely from a clone without unsafe transmute.
pub struct ZaroxiWindow {
    window: Arc<Window>,
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

        let window = Arc::new(event_loop.create_window(attrs).expect("failed to create window"));

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        Self { window, width, height }
    }

    /// Construct a ZaroxiWindow from an existing winit Window.
    /// This is useful when the window was created via ActiveEventLoop::create_window.
    pub fn from_window(window: Window) -> Self {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        Self { window: Arc::new(window), width, height }
    }

    /// Make the window visible and perform a small "warm-up" sequence of calls
    /// intended to nudge compositors (especially Wayland) to map the surface
    /// and schedule an initial frame. These are best-effort and safe public calls.
    pub fn show_and_warmup(&self) {
        // Try to make the window visible and request a frame.
        let _ = self.window.set_visible(true);
        let _ = self.window.pre_present_notify();
        let _ = self.window.request_redraw();

        // Small sleep + second request helps some compositors surface the window.
        std::thread::sleep(std::time::Duration::from_millis(40));
        let _ = self.window.request_redraw();
    }

    /// Return the cached size (width, height) in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Borrow the underlying winit Window for cases where the caller needs
    /// direct access (for example, calling winit window methods).
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Return a clone of the `Arc<Window>` for creating a `wgpu::Surface<'static>`
    /// without unsafe transmute.
    pub fn window_arc(&self) -> Arc<Window> {
        Arc::clone(&self.window)
    }

    /// Update the cached window size (driver code should call this on resize).
    /// Zero sizes are clamped to 1 to avoid wgpu / render panics on minimized windows.
    pub fn update_size(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
    }
}
