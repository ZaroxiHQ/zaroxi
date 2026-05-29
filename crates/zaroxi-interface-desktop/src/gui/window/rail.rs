/*!
Rail drawing logic (left app rail and outer sidebar).

Refined GUI-8 behavior:
- Grouped clusters for the outer sidebar and app rail to imply hierarchy.
- Indentation rhythm on some rows to show nested items.
- Subtle section headers and spacer rhythm.
- Rectangle-only placeholders (no text).
*/

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let sep_h: u32 = std::cmp::max(2, bt);

    let r = &region.rect;

    match region.id {
        // Narrow app rail: keep a vertical list feel but add grouped separators
        "app_rail" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.92),
            });

            // Top group (primary controls)
            let padding: u32 = 8;
            let mut y_off = r.y.saturating_add(padding);
            let top_group_count: u32 = 3;
            let btn_h = 32u32;
            for i in 0..top_group_count {
                let x_inset = 8u32;
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(x_inset),
                    y: y_off,
                    width: r.width.saturating_sub(x_inset.saturating_mul(2)),
                    height: btn_h,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.78 + (i as f64 * 0.03)),
                });
                y_off = y_off.saturating_add(btn_h).saturating_add(padding / 2);
            }

            // Section spacer
            if r.height > y_off.saturating_add(20) {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: y_off,
                    width: r.width,
                    height: 12,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 0.94),
                });
                y_off = y_off.saturating_add(12).saturating_add(padding);
            }

            // Secondary group: stacked rows with small indentation to imply tree
            let rows = 5u32;
            let row_h = 20u32;
            for i in 0..rows {
                let indent = if i % 3 == 0 { 6 } else { 14 };
                let width = r.width.saturating_sub(indent).saturating_sub(8);
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(indent),
                    y: y_off,
                    width,
                    height: row_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 0.96 - (i as f64 * 0.01)),
                });
                y_off = y_off.saturating_add(row_h).saturating_add(padding / 3);
                // subtle dividing line after some rows
                if i == 2 {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: y_off,
                        width: r.width,
                        height: sep_h,
                        color: super::theme_adapter::adjust_brightness(theme.border_color, 0.86),
                    });
                    y_off = y_off.saturating_add(sep_h).saturating_add(padding / 2);
                }
            }

            // Right separator
            if r.width > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(sep_h),
                    y: r.y,
                    width: sep_h,
                    height: r.height,
                    color: super::theme_adapter::parse_hex_color(theme.border_color),
                });
            }
        }

        // Wider sidebar: grouped "panels" with headers and nested rows to imply tree
        "sidebar" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.96),
            });

            let mut y_off = r.y.saturating_add(10);
            let header_h = 22u32;
            let _group_gap = 12u32;

            // First section header
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(8),
                y: y_off,
                width: r.width.saturating_sub(16),
                height: header_h,
                color: super::theme_adapter::adjust_brightness(theme.border_color, 0.88),
            });
            y_off = y_off.saturating_add(header_h).saturating_add(8);

            // Section rows with indentation rhythm
            let section_rows = 6u32;
            let row_h = 18u32;
            for i in 0..section_rows {
                let indent = if i % 2 == 0 { 10 } else { 20 };
                let row_w = r.width.saturating_sub(indent).saturating_sub(16);
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(indent),
                    y: y_off,
                    width: row_w,
                    height: row_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 0.98 - (i as f64 * 0.01)),
                });
                y_off = y_off.saturating_add(row_h).saturating_add(6);
                // occasional thin divider
                if i == 2 {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(6),
                        y: y_off,
                        width: r.width.saturating_sub(12),
                        height: sep_h,
                        color: super::theme_adapter::adjust_brightness(theme.border_color, 0.84),
                    });
                    y_off = y_off.saturating_add(sep_h).saturating_add(6);
                }
            }

            // Second section header
            if y_off.saturating_add(header_h).saturating_add(8) < r.y.saturating_add(r.height) {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(8),
                    y: y_off,
                    width: r.width.saturating_sub(16),
                    height: header_h,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.86),
                });
                y_off = y_off.saturating_add(header_h).saturating_add(8);
            }

            // compact leaf rows
            let more_rows = 4u32;
            for i in 0..more_rows {
                let indent = 14u32;
                let row_w = r.width.saturating_sub(indent).saturating_sub(16);
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(indent),
                    y: y_off,
                    width: row_w,
                    height: row_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 0.95 - (i as f64 * 0.01)),
                });
                y_off = y_off.saturating_add(row_h).saturating_add(6);
            }

            // Right separator
            if r.width > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(sep_h),
                    y: r.y,
                    width: sep_h,
                    height: r.height,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.88),
                });
            }
        }

        _ => {}
    }

    rects
}
