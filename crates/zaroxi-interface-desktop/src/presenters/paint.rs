use crate::presenters::model::{GpuShellView, RegionKind};

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
    /// Semantic text op: carries the textual label and a color.
    /// The executor renders a small deterministic label rectangle for visibility
    /// and transcript generation includes the actual text string.
    Text { x: u32, y: u32, text: String, color: [u8; 4] },
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

                let padding = v.chrome.height.saturating_sub(2).min(4);
                let box_y = v.chrome.y + padding;

                let box_h = 12u32.min(v.chrome.height.saturating_sub(2));
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
                    // Compute a text origin with a left inset.
                    let text_x = box_x + 6;
                    let text_y = box_y + (box_h.saturating_sub(8) / 2); // vertical center accounting for glyph height
                    ops.push(GpuPaintOp::Text {
                        x: text_x,
                        y: text_y,
                        text: label.clone(),
                        color: [10u8, 10u8, 10u8, 255u8],
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
            ops.push(GpuPaintOp::Text {
                x: text_x,
                y: text_y,
                text: preview.clone(),
                color: [10u8, 10u8, 10u8, 255u8],
            });
        }

        // Tab strip rendering (additive, purely presentational)
        //
        // Enhancement: render deterministic small label indicators for each tab.
        // We push a semantic `Text` op that carries the display string; the
        // executor will render a small label rectangle (deterministic size)
        // so the GPU-backed presenter visually shows where labels are and the
        // transcript will include the actual text string for testability/logs.
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
                let focused = t.focused;
                let fill_color = if active {
                    [255u8, 200u8, 0u8, 255u8] // active tab color (distinct)
                } else if focused {
                    [120u8, 160u8, 255u8, 255u8] // focused-but-not-active color (distinct)
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

                // Textual label (semantic): compute a small deterministic label box
                // width based on character budget (6px per char including spacing).
                let display = t.display.clone();
                let max_label_chars = if w > 8 { ((w - 8) / 6) as usize } else { 0 };
                let label_text = if max_label_chars == 0 {
                    String::new()
                } else if display.chars().count() > max_label_chars {
                    // truncate deterministically, replace last char with '.' when truncated
                    let mut s: String = display.chars().take(max_label_chars).collect();
                    if s.len() > 0 {
                        s.replace_range((s.len() - 1).., ".");
                    }
                    s
                } else {
                    display
                };

                if !label_text.is_empty() {
                    let label_w = (label_text.chars().count() as u32).saturating_mul(6);
                    // center label horizontally inside tab
                    let label_x = x + ((w.saturating_sub(label_w)) / 2);
                    // vertically center inside tab; label height fixed to 6px
                    let label_y = tab_bar_y + (tab_bar_h.saturating_sub(6) / 2);

                    // Push a semantic Text op; the executor will render a small
                    // filled rect at this position (visual cue) and transcripts
                    // will include the text value itself for testability.
                    ops.push(GpuPaintOp::Text {
                        x: label_x,
                        y: label_y,
                        text: label_text,
                        color: [10u8, 10u8, 10u8, 255u8],
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
    // For this phase we add a tiny, built-in bitmap glyph renderer so the GPU
    // shell can display readable text without pulling in full font stacks.
    // The renderer is intentionally minimal:
    // - fixed monospace glyphs (6x8) with an integer scale
    // - supports basic ASCII (lowercase, digits, common punctuation) used by the demo
    // - clipped safely against the framebuffer
    //
    // This keeps the presenter self-contained and provides the readable UI
    // required for Phase 14 (first real GUI editor shell).
    fn draw_text_rect(buffer: &mut [u8], width: u32, height: u32, x: u32, y: u32, text: &str, color: [u8; 4]) {
        if text.is_empty() {
            return;
        }

        // Glyph metrics (monospace)
        const GLYPH_W: u32 = 6;
        const GLYPH_H: u32 = 8;
        // Scale glyphs up slightly so text is legible in typical desktop windows.
        // This can be tuned later.
        const SCALE: u32 = 2;

        // Helper: set a pixel in the RGBA buffer with bounds checking.
        fn set_pixel(buf: &mut [u8], fb_width: u32, fb_height: u32, px: i32, py: i32, col: [u8; 4]) {
            if px < 0 || py < 0 {
                return;
            }
            let px = px as u32;
            let py = py as u32;
            if px >= fb_width || py >= fb_height {
                return;
            }
            let idx = ((py * fb_width + px) * 4) as usize;
            if idx + 4 <= buf.len() {
                buf[idx..idx + 4].copy_from_slice(&col);
            }
        }

        // Very small built-in 6x8 mono font. Each glyph is 8 rows of up to 6 bits.
        // The font intentionally covers the common, demo-focused subset:
        // - space, lowercase a-z, digits 0-9, colon, period, dash, underscore
        // For characters not present we draw an empty box placeholder.
        fn glyph_for(ch: char) -> [u8; GLYPH_H as usize] {
            match ch {
                ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                'a' => [0x00, 0x00, 0x1E, 0x01, 0x1F, 0x13, 0x1F, 0x00],
                'b' => [0x10, 0x10, 0x1E, 0x13, 0x13, 0x13, 0x1E, 0x00],
                'c' => [0x00, 0x00, 0x1E, 0x11, 0x10, 0x11, 0x1E, 0x00],
                'd' => [0x01, 0x01, 0x0F, 0x13, 0x13, 0x13, 0x0F, 0x00],
                'e' => [0x00, 0x00, 0x1E, 0x11, 0x1F, 0x10, 0x0F, 0x00],
                'f' => [0x06, 0x09, 0x08, 0x1E, 0x08, 0x08, 0x08, 0x00],
                'g' => [0x00, 0x00, 0x0F, 0x13, 0x13, 0x0F, 0x01, 0x1E],
                'h' => [0x10, 0x10, 0x1E, 0x13, 0x13, 0x13, 0x13, 0x00],
                'i' => [0x04, 0x00, 0x0C, 0x04, 0x04, 0x04, 0x0E, 0x00],
                'j' => [0x02, 0x00, 0x06, 0x02, 0x02, 0x12, 0x12, 0x0C],
                'k' => [0x10, 0x10, 0x12, 0x14, 0x18, 0x14, 0x12, 0x00],
                'l' => [0x0C, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E, 0x00],
                'm' => [0x00, 0x00, 0x1B, 0x1F, 0x15, 0x15, 0x15, 0x00],
                'n' => [0x00, 0x00, 0x1E, 0x13, 0x13, 0x13, 0x13, 0x00],
                'o' => [0x00, 0x00, 0x0E, 0x11, 0x11, 0x11, 0x0E, 0x00],
                'p' => [0x00, 0x00, 0x1E, 0x13, 0x13, 0x1E, 0x10, 0x10],
                'q' => [0x00, 0x00, 0x0F, 0x13, 0x13, 0x0F, 0x01, 0x01],
                'r' => [0x00, 0x00, 0x1A, 0x0C, 0x08, 0x08, 0x08, 0x00],
                's' => [0x00, 0x00, 0x0F, 0x10, 0x0E, 0x01, 0x1E, 0x00],
                't' => [0x08, 0x08, 0x1E, 0x08, 0x08, 0x09, 0x06, 0x00],
                'u' => [0x00, 0x00, 0x13, 0x13, 0x13, 0x13, 0x0F, 0x00],
                'v' => [0x00, 0x00, 0x11, 0x11, 0x0A, 0x0A, 0x04, 0x00],
                'w' => [0x00, 0x00, 0x11, 0x15, 0x15, 0x15, 0x0A, 0x00],
                'x' => [0x00, 0x00, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x00],
                'y' => [0x00, 0x00, 0x11, 0x11, 0x0F, 0x01, 0x1E, 0x00],
                'z' => [0x00, 0x00, 0x1F, 0x02, 0x04, 0x08, 0x1F, 0x00],
                '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E, 0x00],
                '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E, 0x00],
                '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F, 0x00],
                '3' => [0x0E, 0x11, 0x01, 0x06, 0x01, 0x11, 0x0E, 0x00],
                '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02, 0x00],
                '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E, 0x00],
                '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E, 0x00],
                '7' => [0x1F, 0x01, 0x02, 0x04, 0x04, 0x04, 0x04, 0x00],
                '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E, 0x00],
                '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x1C, 0x00],
                ':' => [0x00, 0x00, 0x00, 0x04, 0x00, 0x04, 0x00, 0x00],
                '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00],
                '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00, 0x00],
                '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F, 0x00],
                _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            }
        }

        // Draw a single glyph at (ox, oy) scaled by SCALE. Clipped to framebuffer.
        fn draw_glyph(
            buf: &mut [u8],
            fb_w: u32,
            fb_h: u32,
            ox: i32,
            oy: i32,
            glyph: [u8; GLYPH_H as usize],
            color: [u8; 4],
            scale: u32,
        ) {
            for (row_idx, row) in glyph.iter().enumerate() {
                for bit in 0..GLYPH_W {
                    // bits stored in low-to-high within the byte (we used values as small bitmaps)
                    let mask = 1 << (GLYPH_W - 1 - bit);
                    if (row & mask as u8) != 0 {
                        // plot scaled pixel block
                        let sx = ox + (bit as i32) * (scale as i32);
                        let sy = oy + (row_idx as i32) * (scale as i32);
                        for dy in 0..(scale as i32) {
                            for dx in 0..(scale as i32) {
                                set_pixel(buf, fb_w, fb_h, sx + dx, sy + dy, color);
                            }
                        }
                    }
                }
            }
            // Optional: draw 1px underline for lowercase 'i' or small glyph baseline if needed.
        }

        // Iterate chars and render glyphs left-to-right.
        let mut cursor_x = x as i32;
        let fb_w = width;
        let fb_h = height;
        for ch in text.chars() {
            let glyph = glyph_for(ch);
            draw_glyph(buffer, fb_w, fb_h, cursor_x, y as i32, glyph, color, SCALE);
            cursor_x += (GLYPH_W * SCALE) as i32;
        }
    }

    for op in plan.ops.iter() {
        match op {
            GpuPaintOp::FillRect(r) => fill_rect(buffer, width, r),
            GpuPaintOp::BorderRect { rect, thickness } => draw_border_rect(buffer, width, rect, *thickness),
            GpuPaintOp::Text { x, y, text, color } => draw_text_rect(buffer, width, height, *x, *y, text, *color),
        }
    }
}
