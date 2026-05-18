/*!
A very small, dependency-light GPU/window-backed presenter.

This module provides:
- Region mapping utilities (chrome/content/status regions).
- GpuShellPresenter that can paint into an RGBA8 buffer.
- An optional `run_native` method (uses winit + pixels) guarded by the
  crate-level dependencies (winit + pixels). `run_native` is intentionally
  small and only demonstrates creating a window and filling three regions.

Design notes:
- Keeps the presenter additive to `zaroxi-interface-desktop` and does not
  introduce new crates or high-level UI frameworks.
- The core, testable logic is pure Rust (region mapping + buffer paint).
*/

use std::cmp::min;

/// Simple rectangle region (pixel coordinates).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Region { x, y, width, height }
    }
}

/// Collection of named regions for the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRegions {
    pub chrome: Region,
    pub content: Region,
    pub status: Region,
}

/// Thin GPU-backed presenter. It does not own any heavy application state;
/// it provides pure functions for region mapping and buffer painting.
///
/// The presenter is intentionally small so the core mapping logic is easily tested.
pub struct GpuShellPresenter;

impl GpuShellPresenter {
    /// Compute the chrome/content/status regions for a window of size (width x height).
    /// - chrome_height: default top chrome height in pixels (suggested: 60).
    /// - status_height: default bottom status bar height in pixels (suggested: 24).
    pub fn map_regions(width: u32, height: u32, chrome_height: u32, status_height: u32) -> ShellRegions {
        let chrome_h = min(chrome_height, height);
        let status_h = min(status_height, height.saturating_sub(chrome_h));
        let content_h = height.saturating_sub(chrome_h).saturating_sub(status_h);

        let chrome = Region::new(0, 0, width, chrome_h);
        let content = Region::new(0, chrome_h, width, content_h);
        let status = Region::new(0, chrome_h + content_h, width, status_h);

        ShellRegions { chrome, content, status }
    }

    /// Paint the three regions into the provided RGBA8 buffer.
    ///
    /// - `buffer` must be exactly width * height * 4 bytes long (RGBA8).
    /// - Colors are simple flat fills (no text rendering).
    ///
    /// Color choices (RGBA):
    /// - chrome: dark gray [32, 32, 40, 255]
    /// - content: light gray [220, 220, 225, 255]
    /// - status: medium gray [48, 48, 56, 255]
    pub fn paint_to_buffer(width: u32, height: u32, buffer: &mut [u8], regions: &ShellRegions) {
        let expected = (width as usize) * (height as usize) * 4;
        if buffer.len() != expected {
            // Silence: do nothing if buffer size mismatches. Callers/tests should ensure size.
            return;
        }

        // helper to fill region with color
        let mut fill_region = |region: &Region, color: [u8; 4]| {
            for row in region.y..region.y.saturating_add(region.height) {
                for col in region.x..region.x.saturating_add(region.width) {
                    let idx = ((row * width + col) * 4) as usize;
                    buffer[idx..idx+4].copy_from_slice(&color);
                }
            }
        };

        // Clear to a baseline (transparent black) first.
        for b in buffer.iter_mut() {
            *b = 0;
        }

        fill_region(&regions.content, [220u8, 220u8, 225u8, 255u8]);
        fill_region(&regions.chrome, [32u8, 32u8, 40u8, 255u8]);
        fill_region(&regions.status, [48u8, 48u8, 56u8, 255u8]);
    }

    /// Optional native window runner. Creates a simple window and displays the three regions.
    /// This function blocks the current thread and uses winit + pixels. It's intentionally
    /// small: no event-to-action plumbing is added here, but the window demonstrates the
    /// native path and can be extended to hook into the existing EventBridge.
    ///
    /// Note: This will compile only if `winit` and `pixels` are available (we added them
    /// to Cargo.toml). Errors use `panic!` for brevity in this thin adapter.
    pub fn run_native(initial_width: u32, initial_height: u32) {
        use pixels::{Pixels, SurfaceTexture};
        use winit::dpi::LogicalSize;
        use winit::event::{Event, WindowEvent};
        use winit::event_loop::{ControlFlow, EventLoop};
        use winit::window::WindowBuilder;

        let event_loop = EventLoop::new();
        let window = {
            let size = LogicalSize::new(initial_width as f64, initial_height as f64);
            WindowBuilder::new()
                .with_title("Zaroxi GPU Shell (wireframe)")
                .with_inner_size(size)
                .build(&event_loop)
                .expect("failed to create window")
        };

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let mut pixels = Pixels::new(window_size.width, window_size.height, surface_texture)
            .expect("failed to create pixel surface");

        // Default chrome/status sizes
        let chrome_h = 60u32;
        let status_h = 24u32;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    pixels.resize(size.width, size.height);
                }
                Event::RedrawRequested(_) | Event::MainEventsCleared => {
                    // Acquire frame buffer and paint
                    let frame = pixels.get_frame();
                    let regions = GpuShellPresenter::map_regions(pixels.width(), pixels.height(), chrome_h, status_h);
                    GpuShellPresenter::paint_to_buffer(pixels.width(), pixels.height(), frame, &regions);

                    if pixels
                        .render()
                        .is_err()
                    {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            }

            // Request a redraw to keep the window visible/updated.
            window.request_redraw();
        });
    }
}
