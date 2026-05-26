use crate::presenters::model::{GpuShellView, RegionKind};
use crate::text::cosmic_text_renderer;

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
    /// Semantic text op: carries the textual label, a color, and an explicit
    /// clipping box (max width/height in pixels) that the executor should honor.
    ///
    /// Rationale: ensure small label/text regions are rendered into a bounded
    /// rect (avoid passing full-frame dims as the label bounds) and allow the
    /// executor to clip and avoid wide full-frame debug fills.
    Text { x: u32, y: u32, text: String, color: [u8; 4], max_w: u32, max_h: u32 },
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

        // chrome_label small centered rect + readable text
        if let Some(ref label) = v.chrome_label {
            let b0 = label.as_bytes().get(0).copied().unwrap_or(1);
            let color = [b0, 200u8.wrapping_sub(b0), b0.wrapping_add(40), 255u8];

            let max_w = v.chrome.width.saturating_sub(16);
            let box_w = max_w.min(240); // increase width budget for longer buffer names
            if box_w > 0 {
                let avail = v.chrome.width.saturating_sub(box_w);
                let box_x = v.chrome.x + (avail / 2);

                // Smarter chrome label sizing: ensure a minimum readable height and
                // vertically center the label box inside the chrome. This tightens the
                // chrome layout and provides balanced vertical placement for the label.
                let box_h = std::cmp::max(8u32, std::cmp::min(v.chrome.height.saturating_sub(4), 18u32));
                let box_y = v.chrome.y + (v.chrome.height.saturating_sub(box_h) / 2);

                if box_h > 0 && box_x.saturating_add(box_w) <= v.chrome.x.saturating_add(v.chrome.width) {
                    // Decorative background for chrome label for better contrast
                    ops.push(GpuPaintOp::FillRect(GpuPaintRect {
                        x: box_x,
                        y: box_y,
                        width: box_w,
                        height: box_h,
                        color,
                    }));

                    // Push readable text centered inside the box.
                    // Compute a text origin with a left inset and vertically center it.
                    let text_x = box_x + 6;
                    let text_y = box_y + (box_h.saturating_sub(8) / 2); // vertical center accounting for glyph height
                    // Clip to the chrome label box to avoid using full-frame bounds.
                    let clip_w = box_w;
                    let clip_h = box_h;
                    ops.push(GpuPaintOp::Text {
                        x: text_x,
                        y: text_y,
                        text: label.clone(),
                        color: [10u8, 10u8, 10u8, 255u8],
                        max_w: clip_w,
                        max_h: clip_h,
                    });
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

        // content_preview: render the textual preview inside the content region.
        if let Some(ref preview) = v.content_preview {
            let b0 = preview.as_bytes().get(0).copied().unwrap_or(3);
            let _color = [100u8, 100u8.wrapping_add(b0), 200u8.wrapping_sub(b0), 255u8];

            let text_x = v.content.x + 10;
            // Place preview near the top of the content region with small inset.
            let text_y = v.content.y + 8;
            // Clip the preview to the content region (inset of 10px).
            let clip_w = v.content.width.saturating_sub(10);
            let clip_h = 16u32; // conservative single-line height
            ops.push(GpuPaintOp::Text {
                x: text_x,
                y: text_y,
                text: preview.clone(),
                color: [10u8, 10u8, 10u8, 255u8],
                max_w: clip_w,
                max_h: clip_h,
            });
        }

        // Tab strip rendering (additive, purely presentational)
        //
        // Enhancement: render deterministic small label indicators for each tab.
        // We compute a single canonical label box per tab and use it for layout,
        // clipping, and instrumentation. This prevents mismatches between the
        // presenter's estimate and the executor's glyph metrics.
        if !v.tabs.tabs.is_empty() && v.chrome.height > 2 {
            let num = v.tabs.tabs.len() as u32;
            // Small tab bar: prefer a slightly larger tab bar when chrome allows it,
            // and vertically center the bar inside the chrome to produce balanced spacing.
            let tab_bar_h = std::cmp::min(14u32, v.chrome.height.saturating_sub(4));
            let tab_bar_y = v.chrome.y + (v.chrome.height.saturating_sub(tab_bar_h) / 2);
            // allocate equal widths deterministically
            let base_w = if num > 0 { v.chrome.width / num } else { 0 };
            let mut x = v.chrome.x;

            // Load monospace metrics once so presenter and executor agree.
            let fm = zaroxi_core_engine_font::load_bundled_monospace();
            let glyph_w: u32 = fm.char_width;
            let glyph_h: u32 = fm.line_height;
            // Conservative padding in pixels around text within the label box.
            let pad_x: u32 = (glyph_w / 4).max(2);
            let pad_y: u32 = (glyph_h / 6).max(1);

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
                let focused = t.focused;
                let fill_color = if active {
                    [255u8, 200u8, 0u8, 255u8] // active tab color (distinct)
                } else if focused {
                    [120u8, 160u8, 255u8, 255u8] // focused-but-not-active color (distinct)
                } else {
                    [180u8, 180u8, 180u8, 255u8] // inactive tab color
                };

                // Draw tab body (background)
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

                // Textual label (semantic): compute a canonical label box (text+padding)
                let display = t.display.clone();

                // Compute max chars that fit considering glyph width and padding.
                let available_for_text = if w > pad_x.saturating_mul(2) { w.saturating_sub(pad_x.saturating_mul(2)) } else { 0 };
                let max_label_chars = if glyph_w > 0 { (available_for_text / glyph_w) as usize } else { 0 };

                let label_text = if max_label_chars == 0 {
                    String::new()
                } else if display.chars().count() > max_label_chars {
                    let mut s: String = display.chars().take(max_label_chars).collect();
                    if s.len() > 0 {
                        s.replace_range((s.len() - 1).., ".");
                    }
                    s
                } else {
                    display
                };

                if !label_text.is_empty() {
                    // Compute text pixel bounds and label box including padding.
                    let text_pixel_w = (label_text.chars().count() as u32).saturating_mul(glyph_w);
                    let label_box_w = text_pixel_w.saturating_add(pad_x.saturating_mul(2)).min(w);
                    let label_box_h = glyph_h.saturating_add(pad_y.saturating_mul(2)).min(tab_bar_h);

                    // Center label box inside tab body.
                    let label_box_x = x + ((w.saturating_sub(label_box_w)) / 2);
                    let label_box_y = tab_bar_y + ((tab_bar_h.saturating_sub(label_box_h)) / 2);

                    // Text origin (top-left) inside label box (respecting padding).
                    let text_x = label_box_x.saturating_add(pad_x);
                    let text_y = label_box_y.saturating_add(pad_y);

                    // Instrumentation: log exact rects / origins and draw order (quiet by default).
                    if std::env::var("ZAROXI_DEBUG_TEXT").is_ok() {
                        eprintln!(
                            "TAB DEBUG: tab_body x={} y={} w={} h={} | label_box x={} y={} w={} h={} | text_origin x={} y={} | text=\"{}\" | draw_order=body,border,text",
                            x, tab_bar_y, w, tab_bar_h, label_box_x, label_box_y, label_box_w, label_box_h, text_x, text_y, label_text
                        );
                    }

                    // Push a semantic Text op using the canonical text origin and clip the shaping to the label box.
                    let clip_w = label_box_w.saturating_sub(pad_x.saturating_mul(2));
                    let clip_h = label_box_h.saturating_sub(pad_y.saturating_mul(2));
                    ops.push(GpuPaintOp::Text {
                        x: text_x,
                        y: text_y,
                        text: label_text,
                        color: [10u8, 10u8, 10u8, 255u8],
                        max_w: clip_w,
                        max_h: clip_h,
                    });
                }

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
                // Bounds-check to be safe in case of slightly out-of-range rects.
                if row >= (u32::MAX) || col >= (u32::MAX) {
                    continue;
                }
                let idx = ((row * width + col) * 4) as usize;
                if idx + 4 <= buffer.len() {
                    buffer[idx..idx + 4].copy_from_slice(&rect.color);
                }
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
                    if idx + 4 <= buffer.len() {
                        buffer[idx..idx + 4].copy_from_slice(&rect.color);
                    }
                }
            }
        }
    }

    // Semantic "Text" op executor (renders a small deterministic label rect).
    // This version accepts explicit clip dims (max_w/max_h) so labels are
    // rasterized and clipped to their intended boxes rather than relying on
    // the full-frame dims as the label area.
    fn draw_text_rect(
        buffer: &mut [u8],
        fb_w: u32,
        fb_h: u32,
        x: u32,
        y: u32,
        text: &str,
        color: [u8; 4],
        clip_w: u32,
        clip_h: u32,
    ) {
        if text.is_empty() {
            return;
        }

        // Compute conservative monospace metrics to reason about glyph bounds.
        let fm = zaroxi_core_engine_font::load_bundled_monospace();
        let glyph_w: u32 = fm.char_width;
        let glyph_h: u32 = fm.line_height;
        let glyph_count: u32 = text.chars().count() as u32;
        let text_w = glyph_count.saturating_mul(glyph_w);
        let text_h = glyph_h;

        // Use explicit clip box to determine label extents (avoid full-frame sizes).
        let label_w = std::cmp::min(text_w, clip_w);
        let label_h = std::cmp::min(text_h, clip_h);

        // Compute the safe drawing extents clamped to framebuffer bounds.
        let max_rows = fb_h.saturating_sub(y);
        let max_cols = fb_w.saturating_sub(x);
        let draw_h = label_h.min(max_rows);
        let draw_w = label_w.min(max_cols);

        // Derive glyph-run bounds (in framebuffer coordinates).
        let glyph_bounds_x0 = x;
        let glyph_bounds_y0 = y;
        let glyph_bounds_x1 = x.saturating_add(draw_w);
        let glyph_bounds_y1 = y.saturating_add(draw_h);

        // Instrumentation: report before drawing. Important: we do NOT perform any
        // opaque background fill here. The cosmic renderer must only write glyph pixels.
        if std::env::var("ZAROXI_DEBUG_TEXT").is_ok() {
            eprintln!(
                "DRAW_TEXT_DEBUG: text=\"{}\" origin=({}, {}) clip=({}, {}) fb=({}, {}) glyph_bounds=({}-{}, {}-{}) background_fill_performed={} blend=\"src_over(out = src*alpha + dst*(1-alpha))\"",
                text,
                x,
                y,
                clip_w,
                clip_h,
                fb_w,
                fb_h,
                glyph_bounds_x0,
                glyph_bounds_x1,
                glyph_bounds_y0,
                glyph_bounds_y1,
                /* background_fill_performed */ false
            );
        }

        // Snapshot the destination region so we can count how many destination pixels change.
        // If the computed draw area is empty, skip snapshot.
        let mut pre_snapshot: Vec<u8> = Vec::new();
        let bbox_w = draw_w as usize;
        let bbox_h = draw_h as usize;
        if bbox_w > 0 && bbox_h > 0 {
            pre_snapshot.reserve(bbox_w * bbox_h * 4);
            for row in 0..(bbox_h as u32) {
                let py = y.saturating_add(row) as usize;
                for col in 0..(bbox_w as u32) {
                    let px = x.saturating_add(col) as usize;
                    let idx = ((py as u32 * fb_w + px as u32) * 4) as usize;
                    if idx + 4 <= buffer.len() {
                        pre_snapshot.extend_from_slice(&buffer[idx..idx + 4]);
                    } else {
                        pre_snapshot.extend_from_slice(&[0u8, 0u8, 0u8, 0u8]);
                    }
                }
            }
        }

        // Ensure a global cosmic renderer exists; try to initialize it if absent (tests may run without explicit init).
        let renderer = if let Some(r) = crate::text::COSMIC_RENDERER.get() {
            r.clone()
        } else {
            // Attempt idempotent initialization; tests and binaries can rely on this convenience.
            crate::text::init_cosmic_renderer().expect("failed to initialize COSMIC_RENDERER");
            crate::text::COSMIC_RENDERER
                .get()
                .expect("COSMIC_RENDERER not set after init")
                .clone()
        };

        // Delegate shaping/layout/rasterization to the canonical CosmicTextRenderer.
        // Pass the framebuffer dims for bounds checking and the clip width as the shaping constraint.
        // Note: the Cosmic renderer is expected to only write pixels where glyph coverage > 0.
        cosmic_text_renderer::CosmicTextRenderer::draw_text(
            &renderer,
            buffer,
            fb_w,
            fb_h,
            x as i32,
            y as i32,
            text,
            color,
            Some(clip_w),
        )
        .unwrap_or_else(|e| panic!("CosmicTextRenderer::draw_text failed: {}", e));

        // Count how many destination pixels in the bbox changed.
        let mut touched_pixels: usize = 0;
        if bbox_w > 0 && bbox_h > 0 {
            let mut idx_snapshot = 0usize;
            for row in 0..(bbox_h as u32) {
                let py = y.saturating_add(row) as usize;
                for col in 0..(bbox_w as u32) {
                    let px = x.saturating_add(col) as usize;
                    let idx = ((py as u32 * fb_w + px as u32) * 4) as usize;
                    let before = &pre_snapshot[idx_snapshot..idx_snapshot + 4];
                    idx_snapshot += 4;
                    let after = if idx + 4 <= buffer.len() {
                        &buffer[idx..idx + 4]
                    } else {
                        &[0u8, 0u8, 0u8, 0u8]
                    };
                    if before != after {
                        touched_pixels += 1;
                    }
                }
            }
        }

        if std::env::var("ZAROXI_DEBUG_TEXT").is_ok() {
            eprintln!(
                "DRAW_TEXT_DEBUG_SUMMARY: text=\"{}\" glyph_bbox_w={} glyph_bbox_h={} touched_pixels={} total_bbox_pixels={}",
                text,
                bbox_w,
                bbox_h,
                touched_pixels,
                bbox_w.saturating_mul(bbox_h)
            );
        }
    } // close draw_text_rect

    // Iterate paint ops and execute them into the framebuffer.
    for op in plan.ops.iter() {
        match op {
            GpuPaintOp::FillRect(r) => fill_rect(buffer, width, r),
            GpuPaintOp::BorderRect { rect, thickness } => draw_border_rect(buffer, width, rect, *thickness),
            GpuPaintOp::Text { x, y, text, color, max_w, max_h } => {
                // Use framebuffer dims for bounds but pass the explicit clip box
                // (max_w/max_h) to avoid treating the full frame as the text region.
                draw_text_rect(buffer, width, height, *x, *y, text, *color, *max_w, *max_h);
            }
        }
    }
}
