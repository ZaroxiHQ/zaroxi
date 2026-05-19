use crate::presenters::model::{GpuShellView, TabStrip, RegionKind};

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
        let total_height = v
            .chrome
            .height
            .saturating_add(v.content.height)
            .saturating_add(v.status.height);

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
            let color = [255u8.wrapping_sub(b0), b0, 120u8 as u8, 255u8];

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

        // Tab strip rendering (additive, purely presentational)
        if !v.tabs.tabs.is_empty() && v.chrome.height > 2 {
            let num = v.tabs.tabs.len() as u32;
            // small tab bar inset/padding and height
            let tab_bar_h = 12u32.min(v.chrome.height.saturating_sub(2));
            let tab_bar_y = v.chrome.y + 1;
            // allocate equal widths deterministically
            let base_w = if num > 0 { v.chrome.width / num } else { 0 };
            let mut x = v.chrome.x;
            for (i, t) in v.tabs.tabs.iter().enumerate() {
                let mut w = base_w;
                // last tab takes remainder to avoid gaps
                if (i as u32) + 1 == num {
                    let consumed = base_w.saturating_mul(num.saturating_sub(1));
                    w = v.chrome.width.saturating_sub(consumed);
                }
                if w == 0 {
                    continue;
                }
                let active = t.active;
                let fill_color = if active {
                    [255u8, 200u8, 0u8, 255u8] // active tab color (distinct)
                } else {
                    [180u8, 180u8, 180u8, 255u8] // inactive tab color
                };
                // Draw tab body
                ops.push(GpuPaintOp::FillRect(GpuPaintRect {
                    x,
                    y: tab_bar_y,
                    width: w,
                    height: tab_bar_h,
                    color: fill_color,
                }));
                // Draw a thin border around tab for separation
                ops.push(GpuPaintOp::BorderRect {
                    rect: GpuPaintRect {
                        x,
                        y: tab_bar_y,
                        width: w,
                        height: tab_bar_h,
                        color: [0u8, 0u8, 0u8, 255u8],
                    },
                    thickness: 1u32,
                });
                x = x.saturating_add(w);
            }
        }

        GpuPaintPlan { ops }
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
