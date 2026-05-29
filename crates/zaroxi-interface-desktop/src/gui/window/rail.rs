/*!
Rail drawing logic (left app rail and outer sidebar).
This module accepts a ShellRegion which may be either the narrow app_rail
or the wider sidebar. It inspects the region id to decide which placeholder
decorations to emit. Receives only the region and theme.
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
        "app_rail" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.92),
            });

            // Render a small set of horizontal placeholder rows (grouped list hint).
            let rows: u32 = 4;
            let padding: u32 = 8;
            if r.height > padding && r.width > padding {
                let available_h = r.height.saturating_sub(padding.saturating_mul(rows + 1));
                let row_h = if available_h > 0 { available_h / rows } else { 0 };
                let mut y_off = r.y.saturating_add(padding);
                for i in 0..rows {
                    let inset: u32 = if i == 0 { 6 } else { 10 };
                    let row_w = r.width.saturating_sub(inset.saturating_add(padding));
                    let factor = (1.02_f64 - (i as f64) * 0.01).clamp(0.0, 2.0);
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(inset),
                        y: y_off,
                        width: row_w,
                        height: row_h,
                        color: super::theme_adapter::adjust_brightness(theme.surface, factor),
                    });

                    if row_h > sep_h {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: r.x,
                            y: y_off.saturating_add(row_h),
                            width: r.width,
                            height: sep_h,
                            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.85),
                        });
                    }

                    y_off = y_off.saturating_add(row_h.saturating_add(padding));
                }
            }

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

        "sidebar" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.96),
            });

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
