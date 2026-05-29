/*!
Center editor area drawing logic.

Handles:
- editor_header
- content_left_sidebar
- center_editor (tabs + canvas)
- minimap_lane
- center_bottom_panel

Receives a ShellRegion and Theme and returns DrawRect overlays for that region.
*/

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    use std::cmp;
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let sep_h: u32 = cmp::max(2, bt);
    let r = &region.rect;

    match region.id {
        "editor_header" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
            });
            if r.height > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y.saturating_add(r.height.saturating_sub(sep_h)),
                    width: r.width,
                    height: sep_h,
                    color: super::theme_adapter::parse_hex_color(theme.border_color),
                });
            }
        }

        "content_left_sidebar" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.90),
            });

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

        "center_editor" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.10),
            });

            let top_sep_h = cmp::min(sep_h, r.height);
            if r.height > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: top_sep_h,
                    color: super::theme_adapter::parse_hex_color(theme.border_color),
                });
            }

            // Tab-strip placeholders
            let tab_strip_h: u32 = cmp::min(28, r.height / 8);
            if tab_strip_h > 0 && r.width > 16 {
                let tabs: u32 = 5;
                let tab_padding: u32 = 8;
                let total_padding = tab_padding.saturating_mul(tabs + 1);
                let tab_w = if r.width > total_padding {
                    (r.width.saturating_sub(total_padding)) / tabs
                } else {
                    r.width / tabs
                };
                let mut tx = r.x.saturating_add(tab_padding);
                let tab_y = r.y.saturating_add(top_sep_h);
                for i in 0..tabs {
                    let is_active = i == 0;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: tx,
                        y: tab_y,
                        width: tab_w,
                        height: tab_strip_h,
                        color: if is_active {
                            super::theme_adapter::adjust_brightness(theme.surface, 1.16)
                        } else {
                            super::theme_adapter::adjust_brightness(theme.surface, 1.06)
                        },
                    });
                    tx = tx.saturating_add(tab_w).saturating_add(tab_padding);
                }

                if tab_strip_h > top_sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: tab_y.saturating_add(tab_strip_h),
                        width: r.width,
                        height: top_sep_h,
                        color: super::theme_adapter::parse_hex_color(theme.border_color),
                    });
                }
            }

            if r.width > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(sep_h),
                    y: r.y,
                    width: sep_h,
                    height: r.height,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.90),
                });
            }
        }

        "minimap_lane" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.97),
            });

            if r.width > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: sep_h,
                    height: r.height,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.92),
                });
            }
        }

        "center_bottom_panel" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.88),
            });

            let header_h: u32 = cmp::min(28, r.height / 4);
            if header_h > 0 && r.width > 40 {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: header_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 0.95),
                });

                let segments: u32 = 3;
                let seg_pad: u32 = 10;
                let total_pad = seg_pad.saturating_mul(segments + 1);
                let seg_w = if r.width > total_pad {
                    (r.width.saturating_sub(total_pad)) / segments
                } else {
                    r.width / segments
                };
                let mut sx = r.x.saturating_add(seg_pad);
                let seg_y = r.y.saturating_add((header_h / 6).saturating_mul(1));
                let seg_h = header_h.saturating_sub((header_h / 6) * 2);
                for i in 0..segments {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: sx,
                        y: seg_y,
                        width: seg_w,
                        height: seg_h,
                        color: super::theme_adapter::adjust_brightness(theme.border_color, 0.88 + (i as f64 * 0.02)),
                    });
                    sx = sx.saturating_add(seg_w).saturating_add(seg_pad);
                }

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

            if r.height > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: sep_h,
                    color: super::theme_adapter::parse_hex_color(theme.border_color),
                });
            }
        }

        _ => {}
    }

    rects
}
