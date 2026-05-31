/*!
Left column drawing logic (app rail + outer sidebar).

Phase 3: semantic theme colours, 1 px separators, IDE-grade visual structure.
*/
use zaroxi_interface_theme::theme::ZaroxiTheme;

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let r = &region.rect;
    let sem = ZaroxiTheme::Dark.colors(false);

    match region.id {
        "app_rail" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.activity_rail_background, 1.0),
            });

            let padding: u32 = 6;
            let mut y_off = r.y.saturating_add(padding + 4);
            let btn_w = r.width.saturating_sub(16);
            let btn_h = 26u32;

            // Top group: primary activity icons
            let top_icons: u32 = 4;
            for i in 0..top_icons {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(8),
                    y: y_off,
                    width: btn_w,
                    height: btn_h,
                    color: if i == 0 {
                        super::theme_adapter::adjust_color(sem.accent_soft_background, 1.8)
                    } else {
                        super::theme_adapter::adjust_color(sem.text_faint, 0.32)
                    },
                });
                y_off = y_off.saturating_add(btn_h).saturating_add(padding);
            }

            // Bottom group: settings, account
            let bottom_icons: u32 = 2;
            let bottom_start =
                r.y.saturating_add(r.height)
                    .saturating_sub((bottom_icons * (btn_h + padding)).saturating_add(padding + 8));
            y_off = bottom_start;
            for _i in 0..bottom_icons {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(8),
                    y: y_off,
                    width: btn_w,
                    height: btn_h,
                    color: super::theme_adapter::adjust_color(sem.text_faint, 0.28),
                });
                y_off = y_off.saturating_add(btn_h).saturating_add(padding);
            }

            // Right separator (1 px)
            if r.width > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(bt),
                    y: r.y,
                    width: bt,
                    height: r.height,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.90),
                });
            }
        }

        "sidebar" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.sidebar_background, 1.0),
            });

            let pad: u32 = 10;
            let mut y_off = r.y.saturating_add(pad);

            // Header row
            let header_h: u32 = 24;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad),
                y: y_off,
                width: r.width.saturating_sub(pad * 2),
                height: header_h,
                color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
            });
            y_off = y_off.saturating_add(header_h).saturating_add(6);

            // Search field
            let search_h: u32 = 24;
            if y_off.saturating_add(search_h) < r.y.saturating_add(r.height) {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: search_h,
                    color: super::theme_adapter::adjust_color(sem.input_background, 1.0),
                });
                y_off = y_off.saturating_add(search_h).saturating_add(8);
            }

            // Thin separator after search
            if y_off.saturating_add(bt) < r.y.saturating_add(r.height) {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: bt,
                    color: super::theme_adapter::adjust_color(sem.divider_subtle, 1.0),
                });
                y_off = y_off.saturating_add(bt).saturating_add(8);
            }

            // PROJECT section
            let y_limit = r.y.saturating_add(r.height).saturating_sub(100);
            if y_off.saturating_add(18) < y_limit {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: 18,
                    color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
                });
                y_off = y_off.saturating_add(18).saturating_add(4);

                // PROJECT tree items (reduced)
                let tree_items = [(6u32, "src/"), (16, "main.rs"), (16, "lib.rs"), (6, "tests/")];
                let row_h = 16u32;
                for &(indent, _name) in &tree_items {
                    if y_off.saturating_add(row_h) > y_limit {
                        break;
                    }
                    let row_w = r.width.saturating_sub(pad * 2).saturating_sub(indent);
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(pad).saturating_add(indent),
                        y: y_off.saturating_add(1),
                        width: row_w,
                        height: row_h.saturating_sub(4),
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.30),
                    });
                    y_off = y_off.saturating_add(row_h);
                }
                y_off = y_off.saturating_add(4);
            }

            // GIT section
            if y_off.saturating_add(18) < y_limit {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: 18,
                    color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
                });
                y_off = y_off.saturating_add(18).saturating_add(2);
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad + 16),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2).saturating_sub(16),
                    height: 12,
                    color: super::theme_adapter::adjust_color(sem.text_faint, 0.28),
                });
                y_off = y_off.saturating_add(12).saturating_add(6);
            }

            // OUTLINE section
            if y_off.saturating_add(18) < y_limit {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: r.width.saturating_sub(pad * 2),
                    height: 18,
                    color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
                });
                y_off = y_off.saturating_add(18).saturating_add(2);

                let outline_items = ["fn main()", "struct App", "fn run()"];
                for &_name in &outline_items {
                    if y_off.saturating_add(14) > r.y.saturating_add(r.height).saturating_sub(36) {
                        break;
                    }
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(pad + 14),
                        y: y_off,
                        width: r.width.saturating_sub(pad * 2).saturating_sub(14),
                        height: 12,
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.28),
                    });
                    y_off = y_off.saturating_add(14);
                }
            }

            // Right separator (1 px)
            if r.width > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(bt),
                    y: r.y,
                    width: bt,
                    height: r.height,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.90),
                });
            }
        }

        _ => {}
    }

    // Text labels
    if r.width > 80 && r.height > 40 && region.id == "sidebar" {
        let labels: Vec<String> = vec![
            "Zaroxi Studio".to_string(),
            "Filter files...".to_string(),
            "PROJECT".to_string(),
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "GIT".to_string(),
            "OUTLINE".to_string(),
        ];
        let inset_x = r.x.saturating_add(12);
        let inset_y = r.y.saturating_add(12);
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            inset_x,
            inset_y,
            r.width.saturating_sub(24),
            r.height.saturating_sub(24),
            &labels,
            theme,
            theme.text_secondary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
