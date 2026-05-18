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

        // Clear to a baseline (transparent black) first.
        buffer.fill(0);

        // helper function to fill a region with an RGBA color without capturing a mutable borrow
        fn fill_region(buffer: &mut [u8], width: u32, region: &Region, color: [u8; 4]) {
            for row in region.y..region.y.saturating_add(region.height) {
                for col in region.x..region.x.saturating_add(region.width) {
                    let idx = ((row * width + col) * 4) as usize;
                    buffer[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }

        fill_region(buffer, width, &regions.content, [220u8, 220u8, 225u8, 255u8]);
        fill_region(buffer, width, &regions.chrome, [32u8, 32u8, 40u8, 255u8]);
        fill_region(buffer, width, &regions.status, [48u8, 48u8, 56u8, 255u8]);
    }

    /// Optional native window runner (stub).
    ///
    /// The previous implementation attempted to call into winit/pixels directly,
    /// but the winit event-loop API changed between 0.28 and 0.30 which makes a
    /// tiny cross-crate example fragile. To keep this crate stable and focused on
    /// pure, testable logic (region mapping + buffer painting) we provide a no-op
    /// stub here. Consumers (the desktop harness or an integration module) should
    /// own the event loop and integrate with Pixels/winit directly to avoid
    /// tight coupling to a specific winit API version.
    pub fn run_native(_initial_width: u32, _initial_height: u32) {
        // Intentionally a no-op. Use the harness or an integration layer that
        // depends on the exact winit/pixels versions to create a window and feed
        // frames into GpuShellPresenter::paint_to_buffer.
    }
}
