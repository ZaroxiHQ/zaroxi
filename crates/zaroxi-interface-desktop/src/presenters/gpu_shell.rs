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

    /// Tiny deterministic semantic payloads (kept primitive and optional):
    /// - chrome_label: a short label for the chrome/header (e.g. active buffer name)
    /// - status_text: a short status string for the status bar
    /// - content_preview: a single-line preview or hint for the content region
    ///
    /// These are purely presenter-facing labels (no text shaping). The presenter
    /// may render them as small deterministic colored boxes / bars so they are
    /// observable in the GPU-backed shell while remaining backend-light.
    pub chrome_label: Option<String>,
    pub status_text: Option<String>,
    pub content_preview: Option<String>,
}

/// Presenter-visible explicit, tiny output contract.
///
/// This struct is intentionally minimal and mirrors the stable concepts the
/// presenter already used: ordering, region kind, bounds, marker, borders and
/// the small semantic payloads. Having this explicit type makes future
/// rendering layers consume a stable model rather than ad-hoc region structs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionView {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub kind: RegionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuShellView {
    pub chrome: RegionView,
    pub content: RegionView,
    pub status: RegionView,

    /// Carries the same lightweight marker as ShellRegions (active buffer hint).
    pub marker: Option<String>,

    /// The tiny semantic payloads (kept optional and primitive).
    pub chrome_label: Option<String>,
    pub status_text: Option<String>,
    pub content_preview: Option<String>,
}

impl GpuShellView {
    /// Build a stable presenter view from the adapter's ShellRegions.
    pub fn from_shell_regions(s: &ShellRegions) -> Self {
        let chrome = RegionView {
            x: s.chrome.x,
            y: s.chrome.y,
            width: s.chrome.width,
            height: s.chrome.height,
            kind: s.chrome.kind.clone(),
        };
        let content = RegionView {
            x: s.content.x,
            y: s.content.y,
            width: s.content.width,
            height: s.content.height,
            kind: s.content.kind.clone(),
        };
        let status = RegionView {
            x: s.status.x,
            y: s.status.y,
            width: s.status.width,
            height: s.status.height,
            kind: s.status.kind.clone(),
        };

        GpuShellView {
            chrome,
            content,
            status,
            marker: s.marker.clone(),
            chrome_label: s.chrome_label.clone(),
            status_text: s.status_text.clone(),
            content_preview: s.content_preview.clone(),
        }
    }
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

        ShellRegions { chrome, content, status, marker: None, chrome_label: None, status_text: None, content_preview: None }
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

        // Build the explicit presenter output contract and convert into a paint plan.
        let view = GpuShellView::from_shell_regions(regions);
        let plan = GpuPaintPlan::from_view(&view);

        // helper function to fill a rect with an RGBA color without capturing a mutable borrow
        fn fill_rect(buffer: &mut [u8], width: u32, rect: &GpuPaintRect) {
            for row in rect.y..rect.y.saturating_add(rect.height) {
                for col in rect.x..rect.x.saturating_add(rect.width) {
                    let idx = ((row * width + col) * 4) as usize;
                    buffer[idx..idx + 4].copy_from_slice(&rect.color);
                }
            }
        }

        // Helper to draw an interior border of `thickness` pixels using `color`.
        fn draw_border_rect(buffer: &mut [u8], width: u32, rect: &GpuPaintRect, thickness: u32) {
            if rect.width == 0 || rect.height == 0 || thickness == 0 {
                return;
            }
            let left = rect.x;
            let top = rect.y;
            let right = rect.x + rect.width;
            let bottom = rect.y + rect.height;
            for row in top..top.saturating_add(rect.height) {
                for col in left..left.saturating_add(rect.width) {
                    let in_left = col < left + thickness;
                    let in_right = col >= right.saturating_sub(thickness);
                    let in_top = row < top + thickness;
                    let in_bottom = row >= bottom.saturating_sub(thickness);
                    if in_left || in_right || in_top || in_bottom {
                        let idx = ((row * width + col) * 4) as usize;
                        buffer[idx..idx + 4].copy_from_slice(&rect.color);
                    }
                }
            }
        }

        // Execute plan in deterministic order.
        for op in plan.ops.iter() {
            match op {
                GpuPaintOp::FillRect(r) => fill_rect(buffer, width, r),
                GpuPaintOp::BorderRect { rect, thickness } => draw_border_rect(buffer, width, rect, *thickness),
            }
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

        // Coordinates chosen to fall inside the 1-pixel interior border for each region.
        // Sample at the top-left edge of each region (x=0) which is the border row when
        // border_thickness == 1. This preserves prior interior-fill samples (e.g. (1,1))
        // while still proving that region-kind borders are rendered deterministically.
        let chrome_pixel = sample(0, 0);
        let content_pixel = sample(0, regions.content.y);
        let status_pixel = sample(0, regions.status.y);

        // Expect the deterministic border colors defined in kind_border_color above.
        assert_eq!(chrome_pixel, [200u8, 80u8, 80u8, 255u8]);
        assert_eq!(content_pixel, [80u8, 140u8, 200u8, 255u8]);
        assert_eq!(status_pixel, [80u8, 200u8, 120u8, 255u8]);

        // Sanity: borders must differ between region kinds.
        assert_ne!(chrome_pixel, content_pixel);
        assert_ne!(content_pixel, status_pixel);
    }

    /// Focused contract test: ensure the presenter can produce the explicit
    /// GpuShellView from ShellRegions and that tiny semantic payloads survive
    /// the conversion. This proves the new output contract is available to
    /// downstream consumers while preserving prior invariants.
    #[test]
    fn produce_gpu_shell_view_contract() {
        let width: u32 = 200;
        let height: u32 = 100;
        let chrome_h: u32 = 60;
        let status_h: u32 = 24;

        let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
        // Default mapping produces no semantic payloads.
        let view = GpuShellView::from_shell_regions(&regions);
        assert_eq!(view.chrome.x, regions.chrome.x);
        assert_eq!(view.content.y, regions.content.y);
        assert_eq!(view.marker, regions.marker);
        assert!(view.chrome_label.is_none() && view.status_text.is_none() && view.content_preview.is_none());

        // Ensure payloads propagate through the conversion.
        let mut r2 = regions.clone();
        r2.chrome_label = Some("buf".to_string());
        r2.status_text = Some("status".to_string());
        let view2 = GpuShellView::from_shell_regions(&r2);
        assert_eq!(view2.chrome_label, Some("buf".to_string()));
        assert_eq!(view2.status_text, Some("status".to_string()));
    }

    /// Focused test: ensure converting a GpuShellView -> GpuPaintPlan produces
    /// the expected leading operations (base fills and borders) and preserves
    /// rects/colors deterministically.
    #[test]
    fn paint_plan_from_view_sequence() {
        let width: u32 = 200;
        let height: u32 = 100;
        let chrome_h: u32 = 60;
        let status_h: u32 = 24;

        let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
        let view = GpuShellView::from_shell_regions(&regions);

        let plan = GpuPaintPlan::from_view(&view);

        // Expect at least: content fill, chrome fill, status fill, then three borders.
        assert!(plan.ops.len() >= 6);

        // First three ops should be FillRect for content, chrome, status respectively.
        match &plan.ops[0] {
            GpuPaintOp::FillRect(r) => {
                assert_eq!(r.x, regions.content.x);
                assert_eq!(r.y, regions.content.y);
                assert_eq!(r.width, regions.content.width);
                assert_eq!(r.height, regions.content.height);
                assert_eq!(r.color, [220u8, 220u8, 225u8, 255u8]);
            }
            _ => panic!("expected content FillRect as first op"),
        }

        match &plan.ops[1] {
            GpuPaintOp::FillRect(r) => {
                assert_eq!(r.x, regions.chrome.x);
                assert_eq!(r.y, regions.chrome.y);
                assert_eq!(r.width, regions.chrome.width);
                assert_eq!(r.height, regions.chrome.height);
                assert_eq!(r.color, [32u8, 32u8, 40u8, 255u8]);
            }
            _ => panic!("expected chrome FillRect as second op"),
        }

        match &plan.ops[2] {
            GpuPaintOp::FillRect(r) => {
                assert_eq!(r.x, regions.status.x);
                assert_eq!(r.y, regions.status.y);
                assert_eq!(r.width, regions.status.width);
                assert_eq!(r.height, regions.status.height);
                assert_eq!(r.color, [48u8, 48u8, 56u8, 255u8]);
            }
            _ => panic!("expected status FillRect as third op"),
        }

        // Next three should be BorderRect entries (chrome, content, status).
        match &plan.ops[3] {
            GpuPaintOp::BorderRect { rect, thickness } => {
                assert_eq!(thickness, &1u32);
                assert_eq!(rect.x, regions.chrome.x);
            }
            _ => panic!("expected chrome BorderRect as fourth op"),
        }
        match &plan.ops[4] {
            GpuPaintOp::BorderRect { rect, thickness } => {
                assert_eq!(thickness, &1u32);
                assert_eq!(rect.x, regions.content.x);
            }
            _ => panic!("expected content BorderRect as fifth op"),
        }
        match &plan.ops[5] {
            GpuPaintOp::BorderRect { rect, thickness } => {
                assert_eq!(thickness, &1u32);
                assert_eq!(rect.x, regions.status.x);
            }
            _ => panic!("expected status BorderRect as sixth op"),
        }
    }
}
