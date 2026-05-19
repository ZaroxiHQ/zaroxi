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

/// Kinds of logical regions present in the shell. Kept intentionally small
/// and explicit so the presenter can deterministically map kinds -> visuals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionKind {
    Chrome,
    Content,
    Status,
}

/// Simple rectangle region (pixel coordinates) augmented with a tiny semantic
/// `kind` field to enable deterministic presentational differences without
/// introducing a styling/theme system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub kind: RegionKind,
}

impl Region {
    /// Construct a region defaulting to `Content` kind for convenience.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Region { x, y, width, height, kind: RegionKind::Content }
    }

    /// Construct a region with an explicit semantic kind.
    pub fn with_kind(x: u32, y: u32, width: u32, height: u32, kind: RegionKind) -> Self {
        Region { x, y, width, height, kind }
    }
}

/// Collection of named regions for the shell.
///
/// An optional `marker` string is carried with the regions so the presenter
/// can paint a small deterministic visible cue (a colored bar in the chrome)
/// to reflect lightweight shell state (for example: active buffer name).
/// This keeps the visual change primitive and deterministic while avoiding
/// any heavy composition or text rendering logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRegions {
    pub chrome: Region,
    pub content: Region,
    pub status: Region,
    /// Optional marker string rendered into the chrome to reflect visible state
    /// (e.g. active buffer name). Kept optional and crate-local; presenter simply
    /// paints a deterministic colored marker when present.
    pub marker: Option<String>,
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

        let chrome = Region::with_kind(0, 0, width, chrome_h, RegionKind::Chrome);
        let content = Region::with_kind(0, chrome_h, width, content_h, RegionKind::Content);
        let status = Region::with_kind(0, chrome_h + content_h, width, status_h, RegionKind::Status);

        ShellRegions { chrome, content, status, marker: None }
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

        // Helper to draw an interior border of `thickness` pixels using `color`.
        fn draw_border(buffer: &mut [u8], width: u32, region: &Region, color: [u8; 4], thickness: u32) {
            if region.width == 0 || region.height == 0 || thickness == 0 {
                return;
            }
            let left = region.x;
            let top = region.y;
            let right = region.x + region.width;
            let bottom = region.y + region.height;
            for row in top..top.saturating_add(region.height) {
                for col in left..left.saturating_add(region.width) {
                    let in_left = col < left + thickness;
                    let in_right = col >= right.saturating_sub(thickness);
                    let in_top = row < top + thickness;
                    let in_bottom = row >= bottom.saturating_sub(thickness);
                    if in_left || in_right || in_top || in_bottom {
                        let idx = ((row * width + col) * 4) as usize;
                        buffer[idx..idx + 4].copy_from_slice(&color);
                    }
                }
            }
        }

        // Map semantic region kind -> deterministic border color.
        fn kind_border_color(kind: &RegionKind) -> [u8; 4] {
            match kind {
                RegionKind::Chrome => [200u8, 80u8, 80u8, 255u8],  // warm/red tint for chrome
                RegionKind::Content => [80u8, 140u8, 200u8, 255u8], // cool/blue tint for content
                RegionKind::Status => [80u8, 200u8, 120u8, 255u8],  // greenish tint for status
            }
        }

        // Base fills (kept as simple flat fills to preserve deterministic structure).
        fill_region(buffer, width, &regions.content, [220u8, 220u8, 225u8, 255u8]);
        fill_region(buffer, width, &regions.chrome, [32u8, 32u8, 40u8, 255u8]);
        fill_region(buffer, width, &regions.status, [48u8, 48u8, 56u8, 255u8]);

        // Draw a thin interior border for each region according to its semantic kind.
        let border_thickness = 2u32;
        draw_border(buffer, width, &regions.chrome, kind_border_color(&regions.chrome.kind), border_thickness);
        draw_border(buffer, width, &regions.content, kind_border_color(&regions.content.kind), border_thickness);
        draw_border(buffer, width, &regions.status, kind_border_color(&regions.status.kind), border_thickness);

        // If a marker string is present, draw a small deterministic colored bar in the chrome
        // region's right edge to make visible state changes observable by tests/runs.
        if let Some(ref m) = regions.marker {
            // Simple deterministic color from the first byte of the utf8 representation.
            let b0 = m.as_bytes().get(0).copied().unwrap_or(0);
            let r = b0;
            let g = 255u8.wrapping_sub(b0);
            let b = b0.wrapping_div(2);
            let color = [r, g, b, 255u8];

            // Draw an 8-pixel wide vertical bar anchored to the right edge of the chrome.
            let bar_width = 8u32.min(regions.chrome.width);
            let bar_x = regions.chrome.x + regions.chrome.width.saturating_sub(bar_width);
            let bar_region = Region::with_kind(bar_x, regions.chrome.y, bar_width, regions.chrome.height, RegionKind::Chrome);
            fill_region(buffer, width, &bar_region, color);
        }
    }

    /// Native window runner (no-op in the presenter).
    ///
    /// We intentionally avoid embedding winit/pixels usage in the presenter to
    /// keep the presenter free of platform API churn. The binary (src/bin/gpu_shell.rs)
    /// owns the native event loop and uses the presenter's pure functions
    /// (map_regions + paint_to_buffer) to render into a framebuffer.
    pub fn run_native(_initial_width: u32, _initial_height: u32) {
        // No-op: the native bootstrap lives in the gpu_shell binary to avoid
        // version/API coupling inside this presenter module.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that region mapping produces three ordered regions (chrome above
    /// content above status). This keeps the test crate-local and avoids
    /// depending on the binary-scoped adapter module.
    #[test]
    fn map_regions_preserves_order() {
        let width: u32 = 200;
        let height: u32 = 100;
        let chrome_h: u32 = 60;
        let status_h: u32 = 24;

        // Use the presenter's pure mapping function directly.
        let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);

        // Basic structural assertions: x origin, widths, and vertical ordering.
        assert_eq!(regions.chrome.x, 0);
        assert_eq!(regions.content.x, 0);
        assert_eq!(regions.status.x, 0);

        assert_eq!(regions.chrome.width, width);
        assert_eq!(regions.content.width, width);
        assert_eq!(regions.status.width, width);

        // Vertical ordering: chrome starts at 0, content starts after chrome,
        // status starts after content.
        assert!(regions.chrome.y < regions.content.y);
        assert!(regions.content.y < regions.status.y);
    }

    /// Focused test: ensure semantic region kinds produce deterministic visible
    /// differences (thin interior borders) while preserving ordering and marker.
    #[test]
    fn region_kind_borders_are_deterministic() {
        let width: u32 = 200;
        let height: u32 = 100;
        let chrome_h: u32 = 60;
        let status_h: u32 = 24;

        let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
        let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

        // Paint using the presenter's pure API.
        GpuShellPresenter::paint_to_buffer(width, height, &mut buf, &regions);

        // Helper to sample a pixel (x,y).
        let sample = |x: u32, y: u32| -> [u8; 4] {
            let idx = ((y * width + x) * 4) as usize;
            [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
        };

        // Coordinates chosen to fall inside the 2-pixel interior border for each region.
        let chrome_pixel = sample(1, 1);
        let content_pixel = sample(1, regions.content.y + 1);
        let status_pixel = sample(1, regions.status.y + 1);

        // Expect the deterministic border colors defined in kind_border_color above.
        assert_eq!(chrome_pixel, [200u8, 80u8, 80u8, 255u8]);
        assert_eq!(content_pixel, [80u8, 140u8, 200u8, 255u8]);
        assert_eq!(status_pixel, [80u8, 200u8, 120u8, 255u8]);

        // Sanity: borders must differ between region kinds.
        assert_ne!(chrome_pixel, content_pixel);
        assert_ne!(content_pixel, status_pixel);
    }
}
