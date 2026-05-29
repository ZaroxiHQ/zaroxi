/*!
Bottom dock drawing logic (full-width panel above status).

GUI-8 refinements:
- Clear header row with segmented placeholder tabs (terminal / problems / output)
- Body placeholder lines beneath the header to make the panel visually distinct
*/

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let sep_h: u32 = std::cmp::max(2, bt);
    let r = &region.rect;

    // Panel background
    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_brightness(theme.surface, 0.94),
    });

    // Header area (segmented tabs)
    let header_h: u32 = std::cmp::min(36, r.height / 4);
    if header_h > 0 && r.width > 40 {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: header_h,
            color: super::theme_adapter::adjust_brightness(theme.surface, 0.98),
        });

        // Segments (e.g., Terminal | Problems | Output)
        let segments: u32 = 3;
        let seg_pad: u32 = 12;
        let total_pad = seg_pad.saturating_mul(segments + 1);
        let seg_w = if r.width > total_pad {
            (r.width.saturating_sub(total_pad)) / segments
        } else {
            r.width / segments
        };
        let mut sx = r.x.saturating_add(seg_pad);
        let seg_y = r.y.saturating_add((header_h / 8).saturating_mul(1));
        let seg_h = header_h.saturating_sub((header_h / 8) * 2);

        for i in 0..segments {
            let active = i == 0;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: sx,
                y: seg_y,
                width: seg_w,
                height: seg_h,
                color: if active {
                    super::theme_adapter::adjust_brightness(theme.border_color, 0.92)
                } else {
                    super::theme_adapter::adjust_brightness(theme.border_color, 0.85)
                },
            });
            sx = sx.saturating_add(seg_w).saturating_add(seg_pad);
        }

        // header bottom separator
        if r.height > header_h.saturating_add(sep_h) {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y.saturating_add(header_h),
                width: r.width,
                height: sep_h,
                color: super::theme_adapter::parse_hex_color(theme.border_color),
            });
        }
    }

    // Body: placeholder horizontal bars to suggest logs/output lines
    let body_y = r.y.saturating_add(header_h).saturating_add(sep_h);
    if r.height > body_y.saturating_sub(r.y) && r.width > 40 {
        let available_h = r.height.saturating_sub(body_y.saturating_sub(r.y)).saturating_sub(8);
        let line_h = 14u32;
        let gap = 8u32;
        let lines = if available_h > (line_h + gap) { available_h / (line_h + gap) } else { 0 };
        let mut ly = body_y.saturating_add(8);
        for i in 0..lines {
            let factor = match i % 4 {
                0 => 0.95,
                1 => 0.62,
                2 => 0.82,
                _ => 0.46,
            };
            let w = ((r.width as f64) * factor) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(12),
                y: ly,
                width: w.saturating_sub(12),
                height: line_h,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.02 - (i as f64 * 0.002)),
            });
            ly = ly.saturating_add(line_h).saturating_add(gap);
        }
    }

    // top thin separator for the whole panel (if not drawn above)
    if r.height > sep_h {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: sep_h,
            color: super::theme_adapter::parse_hex_color(theme.border_color),
        });
    }

    // Segmented header labels (Terminal | Problems | Output) using the shared layout.
    if r.width > 80 {
        let labels = vec!["Terminal".to_string(), "Problems".to_string(), "Output".to_string()];
        let inset_x = r.x.saturating_add(12);
        let inset_y = r.y.saturating_add(4);
        let mut text_rects =
            super::text_adapter::layout_and_publish_text(inset_x, inset_y, r.width.saturating_sub(24), 32, &labels, theme);
        rects.append(&mut text_rects);
    }

    rects
}
