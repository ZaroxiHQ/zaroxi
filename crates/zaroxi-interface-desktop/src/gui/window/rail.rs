/*!
Left column drawing logic (activity rail + sidebar).

Phase 4: product-parity left column — active icon indicator, section
headers with dividers, tree items with depth, workspace bottom controls.
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
            // Rail background
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.activity_rail_background, 1.0),
            });

            let icon_gap: u32 = 4;
            let icon_w = r.width.saturating_sub(14);
            let icon_h: u32 = 28;
            let mut y_off = r.y.saturating_add(10);

            // Top group: primary activity icons (explorer, search, git, debug)
            let top_icons = [
                (0, true),  // Explorer (active)
                (1, false), // Search
                (2, false), // Git
                (3, false), // Debug
            ];
            for (i, active) in top_icons {
                let active_w = if active { 3u32 } else { 0 };
                if active {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(2),
                        y: y_off.saturating_add(2),
                        width: active_w,
                        height: icon_h.saturating_sub(4),
                        color: super::theme_adapter::adjust_color(sem.accent, 1.0),
                    });
                }
                let bx = r.x.saturating_add(7);
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: bx,
                    y: y_off,
                    width: icon_w,
                    height: icon_h,
                    color: if active {
                        super::theme_adapter::adjust_color(sem.selected_background, 1.6)
                    } else {
                        super::theme_adapter::adjust_color(sem.text_faint, 0.18)
                    },
                });
                y_off = y_off.saturating_add(icon_h).saturating_add(icon_gap);

                // Small separator after active group
                if i == 0 && r.height > y_off.saturating_sub(r.y).saturating_add(200) {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(12),
                        y: y_off,
                        width: r.width.saturating_sub(24),
                        height: bt,
                        color: super::theme_adapter::adjust_color(sem.divider_subtle, 1.0),
                    });
                    y_off = y_off.saturating_add(bt).saturating_add(icon_gap);
                }
            }

            // Bottom group: settings, account
            let bottom_icons: u32 = 2;
            let bottom_row = icon_h + icon_gap;
            let bottom_start =
                r.y.saturating_add(r.height)
                    .saturating_sub((bottom_icons * bottom_row).saturating_add(12));
            y_off = bottom_start;
            for _i in 0..bottom_icons {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(7),
                    y: y_off,
                    width: icon_w,
                    height: icon_h,
                    color: super::theme_adapter::adjust_color(sem.text_faint, 0.16),
                });
                y_off = y_off.saturating_add(bottom_row);
            }

            // Right 1 px separator
            if r.width > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(bt),
                    y: r.y,
                    width: bt,
                    height: r.height,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.85),
                });
            }
        }

        "sidebar" => {
            // Sidebar background
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.sidebar_background, 1.0),
            });

            let pad: u32 = 10;
            let inner_w = r.width.saturating_sub(pad * 2);
            let mut y_off = r.y.saturating_add(pad);

            // Search bar
            let search_h: u32 = 26;
            if y_off.saturating_add(search_h) < r.y.saturating_add(r.height) {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: inner_w,
                    height: search_h,
                    color: super::theme_adapter::adjust_color(sem.input_background, 1.0),
                });
                y_off = y_off.saturating_add(search_h).saturating_add(8);
            }

            // Subtle divider
            if y_off < r.y.saturating_add(r.height) {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(pad),
                    y: y_off,
                    width: inner_w,
                    height: bt,
                    color: super::theme_adapter::adjust_color(sem.divider_subtle, 0.8),
                });
                y_off = y_off.saturating_add(bt).saturating_add(10);
            }

            // ----- PROJECT section -----
            let section_h: u32 = 20;
            let y_limit = r.y.saturating_add(r.height).saturating_sub(120);
            if y_off.saturating_add(section_h) < y_limit {
                // Section header background strip
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: y_off,
                    width: r.width,
                    height: section_h,
                    color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
                });
                y_off = y_off.saturating_add(section_h).saturating_add(4);

                // Tree items with folder/file colors
                let items = [
                    (6u32, true, "src/"),
                    (16, true, "app/"),
                    (22, false, "main.rs"),
                    (22, false, "lib.rs"),
                    (6, true, "tests/"),
                    (16, false, "integration.rs"),
                ];
                let row_h: u32 = 18;
                for &(indent, is_dir, _name) in &items {
                    if y_off.saturating_add(row_h) > y_limit {
                        break;
                    }
                    let ix = r.x.saturating_add(pad).saturating_add(indent);
                    // Folder or file icon dot
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: ix,
                        y: y_off.saturating_add(4),
                        width: 8,
                        height: 8,
                        color: if is_dir {
                            super::theme_adapter::adjust_color(sem.warning, 0.64)
                        } else {
                            super::theme_adapter::adjust_color(sem.accent, 0.68)
                        },
                    });
                    // Name placeholder
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: ix.saturating_add(14),
                        y: y_off.saturating_add(4),
                        width: inner_w.saturating_sub(indent).saturating_sub(20),
                        height: 10,
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.22),
                    });
                    y_off = y_off.saturating_add(row_h);
                }
                y_off = y_off.saturating_add(6);
            }

            // ----- GIT section -----
            if y_off.saturating_add(section_h) < y_limit {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: y_off,
                    width: r.width,
                    height: section_h,
                    color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
                });
                y_off = y_off.saturating_add(section_h).saturating_add(4);

                // Changed files
                let git_items = ["M src/main.rs", " M src/lib.rs"];
                for &_name in &git_items {
                    if y_off.saturating_add(16) > r.y.saturating_add(r.height).saturating_sub(60) {
                        break;
                    }
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(pad + 14),
                        y: y_off.saturating_add(2),
                        width: inner_w.saturating_sub(20),
                        height: 12,
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.20),
                    });
                    y_off = y_off.saturating_add(16);
                }
                y_off = y_off.saturating_add(6);
            }

            // ----- OUTLINE section -----
            if y_off.saturating_add(section_h) < y_limit {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: y_off,
                    width: r.width,
                    height: section_h,
                    color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
                });
                y_off = y_off.saturating_add(section_h).saturating_add(4);

                let outline_items = ["fn main()", "struct App", "fn run()", "impl App"];
                for &_name in &outline_items {
                    if y_off.saturating_add(14) > r.y.saturating_add(r.height).saturating_sub(36) {
                        break;
                    }
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(pad + 12),
                        y: y_off.saturating_add(1),
                        width: inner_w.saturating_sub(18),
                        height: 12,
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.18),
                    });
                    y_off = y_off.saturating_add(14);
                }
            }

            // Bottom workspace area (dock-style icons)
            let bottom_row_h: u32 = 26;
            if r.height > bottom_row_h {
                let by = r.y.saturating_add(r.height).saturating_sub(bottom_row_h);
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: by,
                    width: r.width,
                    height: bottom_row_h,
                    color: super::theme_adapter::adjust_color(sem.activity_rail_background, 1.0),
                });
                // Icon dots
                let mut bx = r.x.saturating_add(12);
                for _i in 0..4u32 {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: bx,
                        y: by.saturating_add(5),
                        width: 16,
                        height: 16,
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.14),
                    });
                    bx = bx.saturating_add(20);
                }
            }

            // Right separator (1 px)
            if r.width > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(r.width).saturating_sub(bt),
                    y: r.y,
                    width: bt,
                    height: r.height,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.85),
                });
            }
        }

        _ => {}
    }

    // Text labels for sidebar
    if r.width > 80 && r.height > 40 && region.id == "sidebar" {
        let labels: Vec<String> = vec![
            "Filter files...".to_string(),
            "PROJECT".to_string(),
            "src/".to_string(),
            "app/".to_string(),
            "main.rs".to_string(),
            "lib.rs".to_string(),
            "tests/".to_string(),
            "integration.rs".to_string(),
            "GIT".to_string(),
            "M src/main.rs".to_string(),
            "M src/lib.rs".to_string(),
            "OUTLINE".to_string(),
            "fn main()".to_string(),
            "struct App".to_string(),
            "fn run()".to_string(),
            "impl App".to_string(),
        ];
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(12),
            r.y.saturating_add(12),
            r.width.saturating_sub(24),
            r.height.saturating_sub(48),
            &labels,
            theme,
            theme.text_secondary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
