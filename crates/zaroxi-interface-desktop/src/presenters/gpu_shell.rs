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

    /// Tiny deterministic semantic payloads (kept primitive and optional).
    ///
    /// The intent here is to project lightweight, testable semantic labels and
    /// small telemetry indicators into the presenter/transcript path without
    /// introducing any text rendering. These fields are purely semantic (not
    /// visual) and may be surfaced in the debug transcript or used to drive
    /// small deterministic paint tokens (already present as marker/chrome_label).
    ///
    /// - chrome_label: a short label for the chrome/header (e.g. active buffer name)
    /// - status_text: a short status string for the status bar
    /// - content_preview: an optional single-line preview or hint for the content region
    /// - active_buffer_label: explicitly named active buffer (preferred over ad-hoc marker)
    /// - content_preview_count: optional numeric summary of preview lines (semantic)
    /// - ai_indicator: optional tiny AI status summary (e.g. "ai:available" or "ai:off")
    pub chrome_label: Option<String>,
    pub status_text: Option<String>,
    pub content_preview: Option<String>,

    /// New explicit semantic fields (additive; do not affect painting).
    pub active_buffer_label: Option<String>,
    pub content_preview_count: Option<usize>,
    pub ai_indicator: Option<String>,
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

    /// Explicit, additive semantic projection fields:
    /// - active_buffer_label: explicit active buffer name (for transcript/observability)
    /// - content_preview_count: numeric summary of content preview lines
    /// - ai_indicator: tiny AI status summary (semantic only)
    pub active_buffer_label: Option<String>,
    pub content_preview_count: Option<usize>,
    pub ai_indicator: Option<String>,
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
            active_buffer_label: s.active_buffer_label.clone(),
            content_preview_count: s.content_preview_count,
            ai_indicator: s.ai_indicator.clone(),
        }
    }

}
 // ---------------------------------------------------------------------
 // Tiny paint-plan layer (presenter-local, deterministic)
 //
 // This minimal model converts the stable GpuShellView into an ordered,
 // deterministic list of paint operations. The paint execution loop below
 // consumes the plan to produce the exact same pixel output as before.
 //
 // The contract is intentionally small:
 // - FillRect: axis-aligned rectangle + color
 // - BorderRect: interior border rectangle + color + thickness
 //
 // This stays additive to the presenter and preserves visible behavior.
 // ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuPaintRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub color: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuPaintOp {
    FillRect(GpuPaintRect),
    BorderRect { rect: GpuPaintRect, thickness: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuPaintPlan {
    pub ops: Vec<GpuPaintOp>,
}

impl GpuPaintPlan {
    /// Deterministically produce a paint plan from the stable presenter view.
    /// The order of operations mirrors the original presenter's painting order:
    /// 1) base fills: content, chrome, status
    /// 2) interior borders for chrome/content/status
    /// 3) optional marker bar in chrome
    /// 4) optional chrome_label box
    /// 5) optional status_text bar
    /// 6) optional content_preview line
    pub fn from_view(v: &GpuShellView) -> Self {
        let mut ops: Vec<GpuPaintOp> = Vec::new();

        // Ensure the full viewport is deterministically covered first. This
        // inserts a conservative full-viewport background fill using the same
        // base content color. Doing this at the plan stage guarantees that the
        // presenter will produce a clearly partitioned viewport (chrome /
        // content / status) even on minimal binary paths.
        //
        // Compute viewport extents from the aggregated region sizes.
        let total_width = v.chrome.width.max(v.content.width).max(v.status.width);
        let total_height = v.chrome.height.saturating_add(v.content.height).saturating_add(v.status.height);

        // Full-viewport background (same tone as content to keep visuals coherent).
        ops.push(GpuPaintOp::FillRect(GpuPaintRect {
            x: 0,
            y: 0,
            width: total_width,
            height: total_height,
            color: [220u8, 220u8, 225u8, 255u8],
        }));

        // Base fills (content, chrome, status) preserve previous ordering so
        // existing consumers and tests that inspect the content/chrome/status
        // rectangles continue to rely on these deterministic rects overlaying
        // the background.
        ops.push(GpuPaintOp::FillRect(GpuPaintRect {
            x: v.content.x,
            y: v.content.y,
            width: v.content.width,
            height: v.content.height,
            color: [220u8, 220u8, 225u8, 255u8],
        }));
        ops.push(GpuPaintOp::FillRect(GpuPaintRect {
            x: v.chrome.x,
            y: v.chrome.y,
            width: v.chrome.width,
            height: v.chrome.height,
            color: [32u8, 32u8, 40u8, 255u8],
        }));
        ops.push(GpuPaintOp::FillRect(GpuPaintRect {
            x: v.status.x,
            y: v.status.y,
            width: v.status.width,
            height: v.status.height,
            color: [48u8, 48u8, 56u8, 255u8],
        }));

        // Map semantic region kind -> deterministic border color.
        let kind_border_color = |kind: &RegionKind| -> [u8; 4] {
            match kind {
                RegionKind::Chrome => [200u8, 80u8, 80u8, 255u8],
                RegionKind::Content => [80u8, 140u8, 200u8, 255u8],
                RegionKind::Status => [80u8, 200u8, 120u8, 255u8],
            }
        };

        // Interior borders (1px)
        let border_thickness = 1u32;
        ops.push(GpuPaintOp::BorderRect {
            rect: GpuPaintRect {
                x: v.chrome.x,
                y: v.chrome.y,
                width: v.chrome.width,
                height: v.chrome.height,
                color: kind_border_color(&v.chrome.kind),
            },
            thickness: border_thickness,
        });
        ops.push(GpuPaintOp::BorderRect {
            rect: GpuPaintRect {
                x: v.content.x,
                y: v.content.y,
                width: v.content.width,
                height: v.content.height,
                color: kind_border_color(&v.content.kind),
            },
            thickness: border_thickness,
        });
        ops.push(GpuPaintOp::BorderRect {
            rect: GpuPaintRect {
                x: v.status.x,
                y: v.status.y,
                width: v.status.width,
                height: v.status.height,
                color: kind_border_color(&v.status.kind),
            },
            thickness: border_thickness,
        });

        // Marker bar in chrome (right edge)
        if let Some(ref m) = v.marker {
            let b0 = m.as_bytes().get(0).copied().unwrap_or(0);
            let r = b0;
            let g = 255u8.wrapping_sub(b0);
            let b = b0.wrapping_div(2);
            let color = [r, g, b, 255u8];

            let bar_width = 8u32.min(v.chrome.width);
            let bar_x = v.chrome.x + v.chrome.width.saturating_sub(bar_width);
            ops.push(GpuPaintOp::FillRect(GpuPaintRect {
                x: bar_x,
                y: v.chrome.y,
                width: bar_width,
                height: v.chrome.height,
                color,
            }));
        }

        // chrome_label small centered rect
        if let Some(ref label) = v.chrome_label {
            let b0 = label.as_bytes().get(0).copied().unwrap_or(1);
            let color = [b0, 200u8.wrapping_sub(b0), b0.wrapping_add(40), 255u8];

            let max_w = v.chrome.width.saturating_sub(16);
            let box_w = max_w.min(80);
            if box_w > 0 {
                // Safely center the box horizontally. Compute available space first
                // then divide by 2 (division by constant 2 cannot panic).
                let avail = v.chrome.width.saturating_sub(box_w);
                let box_x = v.chrome.x + (avail / 2);

                // Vertical inset: clamp to small padding (no panics on tiny heights).
                let padding = v.chrome.height.saturating_sub(2).min(2);
                let box_y = v.chrome.y + padding;

                let box_h = 6u32.min(v.chrome.height.saturating_sub(2));
                if box_h > 0 && box_x.saturating_add(box_w) <= v.chrome.x.saturating_add(v.chrome.width) {
                    ops.push(GpuPaintOp::FillRect(GpuPaintRect {
                        x: box_x,
                        y: box_y,
                        width: box_w,
                        height: box_h,
                        color,
                    }));
                }
            }
        }

        // status_text small right-aligned rect
        if let Some(ref status) = v.status_text {
            let b0 = status.as_bytes().get(0).copied().unwrap_or(2);
            let color = [255u8.wrapping_sub(b0), b0, 120u8, 255u8];

            let bar_w = 18u32.min(v.status.width);
            let bar_x = v.status.x + v.status.width.saturating_sub(bar_w + 2);
            let bar_y = v.status.y + 1u32.min(v.status.height.saturating_sub(1));
            let bar_h = 6u32.min(v.status.height.saturating_sub(1));
            if bar_h > 0 && bar_w > 0 {
                ops.push(GpuPaintOp::FillRect(GpuPaintRect {
                    x: bar_x,
                    y: bar_y,
                    width: bar_w,
                    height: bar_h,
                    color,
                }));
            }
        }

        // content_preview thin centered line
        if let Some(ref preview) = v.content_preview {
            let b0 = preview.as_bytes().get(0).copied().unwrap_or(3);
            let color = [100u8, 100u8.wrapping_add(b0), 200u8.wrapping_sub(b0), 255u8];

            let line_w = v.content.width.saturating_sub(20);
            if line_w > 0 {
                let line_x = v.content.x + 10;
                // Determine a safe line height and center it vertically inside the content region.
                let line_h = 2u32.min(v.content.height);
                if line_h > 0 {
                    let avail_h = v.content.height.saturating_sub(line_h);
                    let line_y = v.content.y + (avail_h / 2);
                    ops.push(GpuPaintOp::FillRect(GpuPaintRect {
                        x: line_x,
                        y: line_y,
                        width: line_w,
                        height: line_h,
                        color,
                    }));
                }
            }
        }

        GpuPaintPlan { ops }
    }
}

//
// Tiny deterministic debug transcript seam
//
// This additive, read-only representation captures the effective viewport,
// the stable presenter view and the ordered paint-plan entries as deterministic
// textual lines. It is intentionally minimal and purely additive so binaries
// and tests can inspect the final paint intent without relying on pixel checks.
//
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRenderTranscript {
    pub width: u32,
    pub height: u32,
    pub view: GpuShellView,
    pub plan_lines: Vec<String>,
}

impl ShellRenderTranscript {
    /// Construct a transcript from the stable presenter view + paint plan.
    /// The produced plan_lines mirror the exact order of GpuPaintPlan.ops
    /// and contain concise, deterministic descriptions of each op.
    pub fn from_view_and_plan(width: u32, height: u32, view: &GpuShellView, plan: &GpuPaintPlan) -> Self {
        let mut plan_lines = Vec::with_capacity(plan.ops.len());
        for op in plan.ops.iter() {
            match op {
                GpuPaintOp::FillRect(r) => {
                    plan_lines.push(format!(
                        "FillRect x={} y={} w={} h={} color={:?}",
                        r.x, r.y, r.width, r.height, r.color
                    ));
                }
                GpuPaintOp::BorderRect { rect, thickness } => {
                    plan_lines.push(format!(
                        "BorderRect x={} y={} w={} h={} color={:?} thickness={}",
                        rect.x, rect.y, rect.width, rect.height, rect.color, thickness
                    ));
                }
            }
        }

        ShellRenderTranscript {
            width,
            height,
            view: view.clone(),
            plan_lines,
        }
    }

    /// Produce a compact deterministic multi-line textual snapshot suitable for
    /// test assertions or logging by the native binary. The format is stable
    /// and intentionally small.
    pub fn to_string(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("viewport: {}x{}", self.width, self.height));
        lines.push("regions:".to_string());
        lines.push(format!(
            "  chrome: x={} y={} w={} h={} kind={:?}",
            self.view.chrome.x,
            self.view.chrome.y,
            self.view.chrome.width,
            self.view.chrome.height,
            self.view.chrome.kind
        ));
        lines.push(format!(
            "  content: x={} y={} w={} h={} kind={:?}",
            self.view.content.x,
            self.view.content.y,
            self.view.content.width,
            self.view.content.height,
            self.view.content.kind
        ));
        lines.push(format!(
            "  status: x={} y={} w={} h={} kind={:?}",
            self.view.status.x,
            self.view.status.y,
            self.view.status.width,
            self.view.status.height,
            self.view.status.kind
        ));
        if let Some(ref m) = self.view.marker {
            lines.push(format!("marker: {}", m));
        }
        if let Some(ref label) = self.view.chrome_label {
            lines.push(format!("chrome_label: {}", label));
        }
        if let Some(ref status) = self.view.status_text {
            lines.push(format!("status_text: {}", status));
        }
        // Additive semantic projection lines for richer observability.
        if let Some(ref active) = self.view.active_buffer_label {
            lines.push(format!("active_buffer: {}", active));
        }
        if let Some(count) = self.view.content_preview_count {
            lines.push(format!("content_preview_count: {}", count));
        }
        if let Some(ref ai) = self.view.ai_indicator {
            lines.push(format!("ai_indicator: {}", ai));
        }
        // Retain content_preview textual hint if present (semantic only; not rendered).
        if let Some(ref preview) = self.view.content_preview {
            lines.push(format!("content_preview: {}", preview));
        }
        lines.push("plan:".to_string());
        for l in &self.plan_lines {
            lines.push(format!("  {}", l));
        }
        lines.join("\n")
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

        ShellRegions {
            chrome,
            content,
            status,
            marker: None,
            chrome_label: None,
            status_text: None,
            content_preview: None,
            active_buffer_label: None,
            content_preview_count: None,
            ai_indicator: None,
        }
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

        // Delegate execution to the explicit paint-plan executor (pure, dumb).
        execute_paint_plan(&plan, buffer, width, height);
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

/// Execute a paint plan into an RGBA8 buffer.
///
/// This executor is intentionally dumb: it follows the GpuPaintPlan operations
/// exactly and writes pixels into the provided buffer. It performs a size check
/// and returns early when the buffer size does not match width*height*4.
pub fn execute_paint_plan(plan: &GpuPaintPlan, buffer: &mut [u8], width: u32, height: u32) {
    let expected = (width as usize) * (height as usize) * 4;
    if buffer.len() != expected {
        // Silence: do nothing on size mismatch.
        return;
    }

    // helper function to fill a rect with an RGBA color
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

    for op in plan.ops.iter() {
        match op {
            GpuPaintOp::FillRect(r) => fill_rect(buffer, width, r),
            GpuPaintOp::BorderRect { rect, thickness } => draw_border_rect(buffer, width, rect, *thickness),
        }
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

        // Expect at least: background, content fill, chrome fill, status fill, then three borders.
        assert!(plan.ops.len() >= 7);

        // First op should be the full-viewport background FillRect.
        match &plan.ops[0] {
            GpuPaintOp::FillRect(r) => {
                let total_h = regions.chrome.height.saturating_add(regions.content.height).saturating_add(regions.status.height);
                assert_eq!(r.x, 0);
                assert_eq!(r.y, 0);
                assert_eq!(r.width, regions.chrome.width);
                assert_eq!(r.height, total_h);
                assert_eq!(r.color, [220u8, 220u8, 225u8, 255u8]);
            }
            _ => panic!("expected full-viewport background FillRect as first op"),
        }

        // Next three ops should be FillRect for content, chrome, status respectively.
        match &plan.ops[1] {
            GpuPaintOp::FillRect(r) => {
                assert_eq!(r.x, regions.content.x);
                assert_eq!(r.y, regions.content.y);
                assert_eq!(r.width, regions.content.width);
                assert_eq!(r.height, regions.content.height);
                assert_eq!(r.color, [220u8, 220u8, 225u8, 255u8]);
            }
            _ => panic!("expected content FillRect as second op"),
        }

        match &plan.ops[2] {
            GpuPaintOp::FillRect(r) => {
                assert_eq!(r.x, regions.chrome.x);
                assert_eq!(r.y, regions.chrome.y);
                assert_eq!(r.width, regions.chrome.width);
                assert_eq!(r.height, regions.chrome.height);
                assert_eq!(r.color, [32u8, 32u8, 40u8, 255u8]);
            }
            _ => panic!("expected chrome FillRect as third op"),
        }

        match &plan.ops[3] {
            GpuPaintOp::FillRect(r) => {
                assert_eq!(r.x, regions.status.x);
                assert_eq!(r.y, regions.status.y);
                assert_eq!(r.width, regions.status.width);
                assert_eq!(r.height, regions.status.height);
                assert_eq!(r.color, [48u8, 48u8, 56u8, 255u8]);
            }
            _ => panic!("expected status FillRect as fourth op"),
        }

        // Next three should be BorderRect entries (chrome, content, status).
        match &plan.ops[4] {
            GpuPaintOp::BorderRect { rect, thickness } => {
                assert_eq!(thickness, &1u32);
                assert_eq!(rect.x, regions.chrome.x);
            }
            _ => panic!("expected chrome BorderRect as fifth op"),
        }
        match &plan.ops[5] {
            GpuPaintOp::BorderRect { rect, thickness } => {
                assert_eq!(thickness, &1u32);
                assert_eq!(rect.x, regions.content.x);
            }
            _ => panic!("expected content BorderRect as sixth op"),
        }
        match &plan.ops[6] {
            GpuPaintOp::BorderRect { rect, thickness } => {
                assert_eq!(thickness, &1u32);
                assert_eq!(rect.x, regions.status.x);
            }
            _ => panic!("expected status BorderRect as seventh op"),
        }
    }

    /// Focused test: ensure executing a small, explicit GpuPaintPlan writes the
    /// expected pixels into the buffer.
    #[test]
    fn execute_paint_plan_writes_pixels() {
        let width: u32 = 10;
        let height: u32 = 5;
        let mut buf = vec![0u8; (width as usize) * (height as usize) * 4];

        // Single small rect: at (1,1) size 2x2 with a distinctive color.
        let rect = GpuPaintRect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
            color: [11u8, 22u8, 33u8, 44u8],
        };
        let plan = GpuPaintPlan {
            ops: vec![GpuPaintOp::FillRect(rect.clone())],
        };

        // Execute the plan directly (executor should be dumb and follow ops).
        execute_paint_plan(&plan, &mut buf, width, height);

        // Helper to read RGBA at (x,y)
        let read_pixel = |x: u32, y: u32| -> [u8; 4] {
            let idx = ((y * width + x) * 4) as usize;
            [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
        };

        // Pixels inside rect should match color.
        assert_eq!(read_pixel(1, 1), rect.color);
        assert_eq!(read_pixel(2, 1), rect.color);
        assert_eq!(read_pixel(1, 2), rect.color);
        assert_eq!(read_pixel(2, 2), rect.color);

        // Pixel outside should remain zero.
        assert_eq!(read_pixel(0, 0), [0u8, 0u8, 0u8, 0u8]);
    }

    /// Executor size-mismatch remains a no-op (does not panic and does not mutate).
    #[test]
    fn execute_paint_plan_size_mismatch_is_noop() {
        let width: u32 = 8;
        let height: u32 = 4;
        // Wrong sized buffer intentionally.
        let mut buf = vec![7u8; (width as usize) * (height as usize) * 4 - 4];

        let rect = GpuPaintRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            color: [9u8, 9u8, 9u8, 9u8],
        };
        let plan = GpuPaintPlan {
            ops: vec![GpuPaintOp::FillRect(rect)],
        };

        // Should silently return without modifying `buf`.
        execute_paint_plan(&plan, &mut buf, width, height);

        // Ensure buffer unchanged (all bytes still 7).
        assert!(buf.iter().all(|&b| b == 7u8));
    }

    /// Focused test: ensure the debug transcript reflects the final view + plan
    /// in deterministic order and contains essential viewport information.
    #[test]
    fn shell_render_transcript_reflects_plan_order_and_viewport() {
        let width: u32 = 120;
        let height: u32 = 80;
        let chrome_h: u32 = 10;
        let status_h: u32 = 6;

        let regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
        let view = GpuShellView::from_shell_regions(&regions);
        let plan = GpuPaintPlan::from_view(&view);

        // Construct transcript via the new seam.
        let transcript = ShellRenderTranscript::from_view_and_plan(width, height, &view, &plan);
        let txt = transcript.to_string();

        // Sanity: viewport line present.
        assert!(txt.contains(&format!("viewport: {}x{}", width, height)));

        // The transcript.plan_lines must match the plan operation count and order.
        assert_eq!(transcript.plan_lines.len(), plan.ops.len());

        // First op in the plan should be the full-viewport background FillRect.
        assert!(transcript.plan_lines[0].starts_with("FillRect"));

        // Ensure the sequence preserves order: the first three non-background fills
        // are content, chrome, status in that sequence within the plan_lines slice.
        // Find the first occurrence of the content fill (it should exist).
        let mut found_content = false;
        let mut found_status_after = false;
        for (i, line) in transcript.plan_lines.iter().enumerate() {
            if line.contains("FillRect") && !line.contains("content:") {
                // no-op: placeholder to keep logic explicit and readable.
            }
            // Identify chrome fill by matching the chrome region coordinates.
            if line.contains(&format!("x={} y={} w={} h={}", regions.chrome.x, regions.chrome.y, regions.chrome.width, regions.chrome.height)) {
                found_content = true;
                // ensure subsequent lines still contain status later
                for later in transcript.plan_lines.iter().skip(i+1) {
                    if later.contains(&format!("x={} y={} w={} h={}", regions.status.x, regions.status.y, regions.status.width, regions.status.height)) {
                        found_status_after = true;
                        break;
                    }
                }
                break;
            }
        }
        // At minimum, ensure we detected chrome and found status after chrome.
        assert!(found_content);
        assert!(found_status_after);
    }

    /// New focused test: ensure the transcript includes richer semantic payloads
    /// projected from ShellRegions -> GpuShellView -> ShellRenderTranscript.
    #[test]
    fn transcript_includes_semantic_payloads() {
        let width: u32 = 120;
        let height: u32 = 80;
        let chrome_h: u32 = 10;
        let status_h: u32 = 6;

        let mut regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
        // Populate explicit semantic payloads (additive; doesn't affect painting).
        regions.chrome_label = Some("chromeX".to_string());
        regions.status_text = Some("ok".to_string());
        regions.content_preview = Some("preview-line".to_string());
        regions.active_buffer_label = Some("active_buf".to_string());
        regions.content_preview_count = Some(1usize);
        regions.ai_indicator = Some("ai:available".to_string());

        let view = GpuShellView::from_shell_regions(&regions);
        let plan = GpuPaintPlan::from_view(&view);
        let transcript = ShellRenderTranscript::from_view_and_plan(width, height, &view, &plan);
        let txt = transcript.to_string();

        assert!(txt.contains("chrome_label: chromeX"));
        assert!(txt.contains("status_text: ok"));
        assert!(txt.contains("marker:"));
        assert!(txt.contains("active_buffer: active_buf"));
        assert!(txt.contains("content_preview_count: 1"));
        assert!(txt.contains("ai_indicator: ai:available"));
    }

    /// Regression: ensure minimal / zero viewport sizes and zero-count previews
    /// do not panic and still produce a deterministic transcript.
    #[test]
    fn handle_zero_and_minimal_viewport_safely() {
        // Zero-sized viewport
        let width: u32 = 0;
        let height: u32 = 0;
        let chrome_h: u32 = 0;
        let status_h: u32 = 0;

        let mut regions = GpuShellPresenter::map_regions(width, height, chrome_h, status_h);
        // Expose semantic payload edge-cases: empty preview and zero count.
        regions.content_preview = Some(String::new());
        regions.content_preview_count = Some(0usize);
        regions.active_buffer_label = Some(String::new());
        regions.ai_indicator = Some(String::new());

        let view = GpuShellView::from_shell_regions(&regions);
        let plan = GpuPaintPlan::from_view(&view);

        // Construct transcript; this should not panic even with zero dims and
        // odd semantic payload values.
        let transcript = ShellRenderTranscript::from_view_and_plan(width, height, &view, &plan);
        let txt = transcript.to_string();

        assert!(txt.contains("viewport: 0x0"));
        // The optional fields should be represented (even if empty or zero).
        assert!(txt.contains("content_preview_count: 0"));
        assert!(txt.contains("marker:") || txt.contains("marker: "));
    }
}
