/*!
Center editor area drawing logic.

GUI-8 refinements:
- Tab strip with one active tab + a few inactive tabs
- Inner split: gutter zone + editor content zone
- Repeated code-line placeholder bars in the editor body with varied lengths
- Geometry-only placeholders (no glyphs / text)
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
            // Slightly brighter header surface
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
            });

            // thin bottom separator
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
            // keep the left inner rail simple but slightly varied
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
            // Base canvas
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.10),
            });

            let top_sep_h = cmp::min(sep_h, r.height);
            if r.height > sep_h {
                // top thin separator
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: top_sep_h,
                    color: super::theme_adapter::parse_hex_color(theme.border_color),
                });
            }

            // Tab strip: one active, 2-3 inactive tabs
            let tab_strip_h: u32 = cmp::min(34, r.height / 10);
            if tab_strip_h > 0 && r.width > 40 {
                let tab_padding: u32 = 10;
                let tabs_total = 4u32;
                // compute a base width for tabs and allow active tab to be wider
                let base_tab_w = (r.width.saturating_sub(tab_padding * (tabs_total + 1))) / tabs_total;
                let active_extra = base_tab_w / 3;
                let mut tx = r.x.saturating_add(tab_padding);
                let tab_y = r.y.saturating_add(top_sep_h);

                for i in 0..tabs_total {
                    let is_active = i == 0;
                    let w = if is_active { base_tab_w.saturating_add(active_extra) } else { base_tab_w };
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: tx,
                        y: tab_y,
                        width: w,
                        height: tab_strip_h,
                        color: if is_active {
                            super::theme_adapter::adjust_brightness(theme.surface, 1.20)
                        } else {
                            super::theme_adapter::adjust_brightness(theme.surface, 1.08)
                        },
                    });

                    // small separation between tabs
                    tx = tx.saturating_add(w).saturating_add(tab_padding);
                }

                // separator under tabs
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

            // Inner split: gutter zone + editor content zone
            let gutter_w: u32 = 48;
            let content_x = r.x.saturating_add(gutter_w);
            let content_w = if r.width > gutter_w { r.width.saturating_sub(gutter_w) } else { 0 };
            let body_y = r.y.saturating_add(tab_strip_h.saturating_add(top_sep_h));
            let body_h = if r.height > (tab_strip_h.saturating_add(top_sep_h)) {
                r.height.saturating_sub(tab_strip_h.saturating_add(top_sep_h))
            } else {
                0
            };

            // gutter background
            if gutter_w > 0 && body_h > 0 {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: body_y,
                    width: gutter_w,
                    height: body_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 0.98),
                });

                // divider line between gutter and content
                if gutter_w > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: content_x.saturating_sub(sep_h),
                        y: body_y,
                        width: sep_h,
                        height: body_h,
                        color: super::theme_adapter::parse_hex_color(theme.border_color),
                    });
                }
            }

            // editor body: repeated code-line placeholder bars with varied lengths
            if content_w > 0 && body_h > 0 {
                // line metrics (deterministic)
                let line_h: u32 = 18;
                let lines = (body_h / (line_h + 6)).max(1);
                let mut ly = body_y.saturating_add(8);
                for i in 0..lines {
                    // vary width to imply code structure: alternate shorter/longer
                    let length_factor = match i % 5 {
                        0 => 0.92,
                        1 => 0.60,
                        2 => 0.78,
                        3 => 0.44,
                        _ => 0.70,
                    };
                    let bar_w = ((content_w as f64) * length_factor) as u32;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: content_x.saturating_add(12),
                        y: ly,
                        width: bar_w.saturating_sub(12),
                        height: line_h,
                        color: super::theme_adapter::adjust_brightness(theme.surface, 1.04 - (i as f64 * 0.002)),
                    });

                    ly = ly.saturating_add(line_h).saturating_add(6);
                    // subtle thin divider every few lines
                    if i > 0 && i % 6 == 0 && ly.saturating_add(sep_h) < r.y.saturating_add(r.height) {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: content_x,
                            y: ly,
                            width: content_w,
                            height: sep_h,
                            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.86),
                        });
                        ly = ly.saturating_add(sep_h).saturating_add(4);
                    }
                }
            }

            // right gutter/separator (visual balance)
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
            // keep bottom panel visuals but they will be enhanced in bottom_panel module
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

    // Tab labels (use Cosmic Text layout path) when enough horizontal room exists.
    if region.id == "center_editor" || region.id == "editor_header" {
        if r.width > 120 && r.height > 16 {
            let tabs = vec!["main.rs".to_string(), "lib.rs".to_string(), "mod.rs".to_string()];
            let inset_x = r.x.saturating_add(12);
            let inset_y = r.y.saturating_add(6);
            let mut text_rects =
                super::text_adapter::layout_and_publish_text(inset_x, inset_y, r.width.saturating_sub(24), r.height.saturating_sub(12), &tabs, theme);
            rects.append(&mut text_rects);
        }
    }

    rects
}
