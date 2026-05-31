/*!
Left column drawing logic (app rail + outer sidebar).

Phase 2: sidebar structured as Zaroxi Studio header, search field,
PROJECT tree, GIT section, OUTLINE section, and tool dock icons.
App rail kept minimal with vertical icon strip.
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
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.88),
            });

            let padding: u32 = 6;
            let mut y_off = r.y.saturating_add(padding);
            let btn_w = r.width.saturating_sub(12);
            let btn_h = 28u32;

            // Top group: primary activity icons (Explorer, Search, Git, Extensions)
            let top_icons: u32 = 4;
            for i in 0..top_icons {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(6),
                    y: y_off,
                    width: btn_w,
                    height: btn_h,
                    color: if i == 0 {
                        super::theme_adapter::parse_hex_color(theme.border_color)
                    } else {
                        super::theme_adapter::adjust_brightness(
                            theme.surface,
                            0.98 + (i as f64 * 0.01),
                        )
                    },
                });
                y_off = y_off.saturating_add(btn_h).saturating_add(padding);
            }

            // Bottom group: settings, account
            let bottom_icons: u32 = 2;
            let bottom_start =
                r.y.saturating_add(r.height)
                    .saturating_sub((bottom_icons * (btn_h + padding)).saturating_add(padding));
            y_off = bottom_start;
            for i in 0..bottom_icons {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(6),
                    y: y_off,
                    width: btn_w,
                    height: btn_h,
                    color: super::theme_adapter::adjust_brightness(
                        theme.surface,
                        0.98 + (i as f64 * 0.01),
                    ),
                });
                y_off = y_off.saturating_add(btn_h).saturating_add(padding);
            }

            // Right separator
            if r.width > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(sep_h),
                    y: r.y,
                    width: sep_h,
                    height: r.height,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.75),
                });
            }
        }

        "sidebar" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.95),
            });

            let pad: u32 = 10;
            let mut y_off = r.y.saturating_add(pad);

            // --- Zaroxi Studio header ---
            let header_h: u32 = 28;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad),
                y: y_off,
                width: r.width.saturating_sub(pad * 2),
                height: header_h,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.04),
            });
            y_off = y_off.saturating_add(header_h).saturating_add(6);

            // --- Search field ---
            let search_h: u32 = 26;
            if y_off.saturating_add(search_h) < r.y.saturating_add(r.height) {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: search_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
                });
                // Search icon placeholder (left edge of search bar)
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad + 4),
                    y: y_off.saturating_add(4),
                    width: 18,
                    height: search_h.saturating_sub(8),
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.92),
                });
                y_off = y_off.saturating_add(search_h).saturating_add(8);
            }

            // Thin separator
            if y_off.saturating_add(sep_h) < r.y.saturating_add(r.height) {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: sep_h,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.82),
                });
                y_off = y_off.saturating_add(sep_h).saturating_add(8);
            }

            // --- PROJECT section ---
            let section_gap = 6u32;
            let y_limit = r.y.saturating_add(r.height).saturating_sub(120); // reserve space for GIT/OUTLINE

            // PROJECT header
            if y_off.saturating_add(20) < y_limit {
                let proj_header_y = y_off;
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: proj_header_y,
                    width: r.width.saturating_sub(pad * 2),
                    height: 20,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 1.00),
                });
                y_off = y_off.saturating_add(20).saturating_add(section_gap);

                // PROJECT tree items
                let tree_items = [
                    (6u32, "src/"),
                    (16, "main.rs"),
                    (16, "lib.rs"),
                    (6, "Cargo.toml"),
                    (16, "mod.rs"),
                    (6, "tests/"),
                    (16, "integration.rs"),
                ];
                let row_h = 17u32;
                for &(indent, _name) in &tree_items {
                    if y_off.saturating_add(row_h) > y_limit {
                        break;
                    }
                    let row_w = r.width.saturating_sub(pad * 2).saturating_sub(indent);
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(pad).saturating_add(indent),
                        y: y_off.saturating_add(2),
                        width: row_w,
                        height: row_h.saturating_sub(4),
                        color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
                    });
                    y_off = y_off.saturating_add(row_h);
                }
                y_off = y_off.saturating_add(4);
            }

            // --- GIT section ---
            if y_off.saturating_add(20) < y_limit {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: 20,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 1.00),
                });
                y_off = y_off.saturating_add(20).saturating_add(2);
                // GIT status row
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad + 16),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2).saturating_sub(16),
                    height: 14,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
                });
                y_off = y_off.saturating_add(14).saturating_add(6);
            }

            // --- OUTLINE section ---
            if y_off.saturating_add(20) < y_limit {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: 20,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 1.00),
                });
                y_off = y_off.saturating_add(20).saturating_add(2);

                // Outline symbols (indented)
                let outline_items = ["fn main()", "struct App", "impl App", "fn run()"];
                for &_name in &outline_items {
                    if y_off.saturating_add(16) > r.y.saturating_add(r.height).saturating_sub(48) {
                        break;
                    }
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(pad + 14),
                        y: y_off,
                        width: r.width.saturating_sub(pad * 2).saturating_sub(14),
                        height: 14,
                        color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
                    });
                    y_off = y_off.saturating_add(16);
                }
            }

            // --- Bottom tool dock icons ---
            let dock_y = r.y.saturating_add(r.height).saturating_sub(38);
            let dock_h: u32 = 28;
            let icon_w: u32 = 26;
            let icon_pad: u32 = 8;
            let icon_count: u32 = 4;
            let total_icons_w = icon_count * icon_w + (icon_count.saturating_sub(1)) * icon_pad;
            let dock_start_x = r.x.saturating_add((r.width.saturating_sub(total_icons_w)) / 2);
            for i in 0..icon_count {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: dock_start_x.saturating_add(i * (icon_w + icon_pad)),
                    y: dock_y,
                    width: icon_w,
                    height: dock_h,
                    color: super::theme_adapter::adjust_brightness(
                        theme.border_color,
                        0.82 + (i as f64 * 0.03),
                    ),
                });
            }

            // Right separator
            if r.width > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(sep_h),
                    y: r.y,
                    width: sep_h,
                    height: r.height,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.80),
                });
            }
        }

        _ => {}
    }

    // Add text labels via the Cosmic Text layout path
    if r.width > 80 && r.height > 40 {
        let labels: Vec<String> = if region.id == "app_rail" {
            vec![]
        } else {
            vec![
                "Zaroxi Studio".to_string(),
                "Filter files...".to_string(),
                "PROJECT".to_string(),
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "GIT".to_string(),
                "OUTLINE".to_string(),
            ]
        };
        if !labels.is_empty() {
            let inset_x = r.x.saturating_add(12);
            let inset_y = r.y.saturating_add(12);
            let color =
                if region.id == "app_rail" { theme.text_primary } else { theme.text_secondary };
            let mut text_rects = super::text_adapter::layout_and_publish_text(
                inset_x,
                inset_y,
                r.width.saturating_sub(24),
                r.height.saturating_sub(24),
                &labels,
                theme,
                color,
            );
            rects.append(&mut text_rects);
        }
    }

    rects
}
