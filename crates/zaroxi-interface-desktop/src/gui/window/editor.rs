/*!
Center editor area drawing logic.

Phase 3: semantic theme colours, 1 px separators, IDE-grade editor rendering.
*/
use zaroxi_interface_theme::theme::ZaroxiTheme;

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    use std::cmp;
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let r = &region.rect;
    let sem = ZaroxiTheme::Dark.colors(false);

    match region.id {
        "editor_tabs" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.tab_strip_background, 1.0),
            });

            if r.width > 40 && r.height > 0 {
                let tab_h: u32 = r.height;
                let tab_count: u32 = 4;
                let tab_pad: u32 = 2;
                let tab_w = (r.width.saturating_sub(tab_pad * (tab_count + 1))) / tab_count;
                let active_extra = cmp::min(tab_w / 3, 20);
                let mut tx = r.x.saturating_add(tab_pad);
                let tab_y = r.y;

                for i in 0..tab_count {
                    let is_active = i == 0;
                    let w = if is_active {
                        cmp::min(
                            tab_w.saturating_add(active_extra),
                            r.width.saturating_sub(tx - r.x),
                        )
                    } else {
                        tab_w
                    };
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: tx,
                        y: tab_y,
                        width: w,
                        height: tab_h,
                        color: if is_active {
                            super::theme_adapter::adjust_color(sem.tab_active_background, 1.0)
                        } else {
                            super::theme_adapter::adjust_color(sem.tab_background, 1.0)
                        },
                    });
                    tx = tx.saturating_add(w).saturating_add(tab_pad);
                }

                // Bottom separator (1 px)
                if r.height > bt {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: r.y.saturating_add(r.height.saturating_sub(bt)),
                        width: r.width,
                        height: bt,
                        color: super::theme_adapter::adjust_color(sem.divider, 0.9),
                    });
                }
            }
        }

        "breadcrumb" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.tab_strip_background, 0.92),
            });

            if r.width > 120 && r.height > 10 {
                let seg_count: u32 = 4;
                let seg_pad: u32 = 10;
                let cy = r.y.saturating_add(3);
                let seg_h: u32 = 12;
                let mut sx = r.x.saturating_add(10);
                let seg_widths = [26u32, 20, 20, 44];

                for i in 0..seg_count {
                    if sx + seg_widths[i as usize] < r.x + r.width {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: sx,
                            y: cy,
                            width: seg_widths[i as usize],
                            height: seg_h,
                            color: super::theme_adapter::adjust_color(sem.text_faint, 0.32),
                        });
                        sx = sx.saturating_add(seg_widths[i as usize]);
                        if i < seg_count - 1 && sx + 6 < r.x + r.width {
                            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                                x: sx,
                                y: cy.saturating_add(2),
                                width: seg_pad.saturating_sub(4),
                                height: seg_h.saturating_sub(4),
                                color: super::theme_adapter::adjust_color(sem.divider, 0.6),
                            });
                            sx = sx.saturating_add(seg_pad);
                        }
                    }
                }
            }

            if r.height > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y.saturating_add(r.height.saturating_sub(bt)),
                    width: r.width,
                    height: bt,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.9),
                });
            }
        }

        "center_editor" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.editor_background, 1.0),
            });

            let gutter_w: u32 = 44;
            let content_x = if r.width > gutter_w { r.x.saturating_add(gutter_w) } else { r.x };
            let content_w = if r.width > gutter_w { r.width.saturating_sub(gutter_w) } else { 0 };

            if gutter_w > 0 && r.height > 0 && r.width > gutter_w {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: gutter_w,
                    height: r.height,
                    color: super::theme_adapter::adjust_color(sem.editor_gutter_background, 1.0),
                });

                if gutter_w > bt {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: content_x.saturating_sub(bt),
                        y: r.y,
                        width: bt,
                        height: r.height,
                        color: super::theme_adapter::adjust_color(sem.divider, 0.65),
                    });
                }

                // Line numbers in gutter
                let num_w: u32 = 28;
                let num_h: u32 = 15;
                let num_x = r.x.saturating_add(6);
                let mut num_y = r.y.saturating_add(6);
                let max_nums = r.height / (num_h.saturating_add(2));
                for _ in 0..max_nums {
                    if num_y + num_h > r.y + r.height {
                        break;
                    }
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: num_x,
                        y: num_y,
                        width: num_w,
                        height: num_h.saturating_sub(2),
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.30),
                    });
                    num_y = num_y.saturating_add(num_h).saturating_add(2);
                }
            }

            // Code lines with syntax-colored segments
            if content_w > 0 && r.height > 0 {
                let line_h: u32 = 15;
                let line_gap: u32 = 2;
                let total_line_h = line_h + line_gap;
                let max_lines = r.height / total_line_h;
                let mut ly = r.y.saturating_add(6);

                let code_lines: &[&[(f64, &str)]] = &[
                    &[
                        (0.08, "keyword"),
                        (0.26, "default"),
                        (0.04, "default"),
                        (0.30, "type"),
                        (0.04, "default"),
                    ],
                    &[
                        (0.08, "keyword"),
                        (0.24, "default"),
                        (0.04, "default"),
                        (0.18, "default"),
                        (0.04, "default"),
                    ],
                    &[(0.06, "default")],
                    &[
                        (0.06, "keyword"),
                        (0.14, "function"),
                        (0.04, "default"),
                        (0.06, "keyword"),
                        (0.18, "type"),
                        (0.06, "default"),
                    ],
                    &[
                        (0.10, "keyword"),
                        (0.10, "default"),
                        (0.04, "default"),
                        (0.22, "type"),
                        (0.04, "default"),
                        (0.04, "default"),
                    ],
                    &[
                        (0.10, "keyword"),
                        (0.10, "default"),
                        (0.04, "default"),
                        (0.30, "type"),
                        (0.04, "default"),
                    ],
                    &[
                        (0.10, "default"),
                        (0.14, "function"),
                        (0.04, "default"),
                        (0.08, "keyword"),
                        (0.04, "default"),
                    ],
                    &[(0.04, "default")],
                    &[(0.06, "default")],
                    &[(0.12, "keyword"), (0.18, "type"), (0.06, "default")],
                    &[(0.18, "default"), (0.04, "default"), (0.28, "type"), (0.04, "default")],
                    &[(0.18, "default"), (0.04, "default"), (0.28, "type"), (0.04, "default")],
                    &[(0.12, "comment")],
                    &[(0.18, "default"), (0.04, "default"), (0.16, "type"), (0.04, "default")],
                    &[(0.04, "default")],
                    &[(0.06, "default")],
                    &[(0.10, "keyword"), (0.18, "type"), (0.06, "default")],
                    &[
                        (0.14, "function"),
                        (0.10, "default"),
                        (0.04, "default"),
                        (0.24, "type"),
                        (0.06, "keyword"),
                        (0.06, "default"),
                    ],
                    &[
                        (0.20, "type"),
                        (0.12, "default"),
                        (0.04, "default"),
                        (0.10, "type"),
                        (0.04, "default"),
                        (0.06, "type"),
                        (0.04, "default"),
                        (0.10, "type"),
                    ],
                    &[(0.04, "default")],
                    &[
                        (0.16, "keyword"),
                        (0.18, "function"),
                        (0.04, "default"),
                        (0.06, "keyword"),
                        (0.10, "default"),
                        (0.06, "keyword"),
                        (0.16, "type"),
                        (0.06, "default"),
                    ],
                    &[(0.14, "default"), (0.04, "default"), (0.14, "number"), (0.04, "default")],
                    &[(0.10, "function"), (0.04, "default"), (0.42, "string"), (0.04, "default")],
                    &[(0.12, "comment")],
                ];

                for (line_idx, segments) in code_lines.iter().enumerate() {
                    if ly + line_h > r.y + r.height || line_idx >= max_lines as usize {
                        break;
                    }
                    let mut seg_x = content_x.saturating_add(8);
                    let remaining_w = content_w.saturating_sub(16);

                    for &(pct, syntax_type) in *segments {
                        let seg_w = ((remaining_w as f64) * pct.min(1.0)) as u32;
                        if seg_w == 0 {
                            continue;
                        }
                        let color = match syntax_type {
                            "keyword" => {
                                super::theme_adapter::adjust_color(sem.syntax_keyword, 0.95)
                            }
                            "function" => {
                                super::theme_adapter::adjust_color(sem.syntax_function, 0.95)
                            }
                            "string" => super::theme_adapter::adjust_color(sem.syntax_string, 0.95),
                            "comment" => {
                                super::theme_adapter::adjust_color(sem.syntax_comment, 0.88)
                            }
                            "type" => super::theme_adapter::adjust_color(sem.syntax_type, 0.95),
                            "number" => super::theme_adapter::adjust_color(sem.syntax_number, 0.95),
                            _ => super::theme_adapter::adjust_color(sem.text_secondary, 0.40),
                        };
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: seg_x,
                            y: ly,
                            width: seg_w,
                            height: line_h.saturating_sub(2),
                            color,
                        });
                        seg_x = seg_x.saturating_add(seg_w);
                    }
                    ly = ly.saturating_add(total_line_h);

                    // Thin logical block divider
                    if line_idx == 1 || line_idx == 8 || line_idx == 14 {
                        if ly < r.y + r.height {
                            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                                x: content_x,
                                y: ly.saturating_sub(line_gap),
                                width: content_w,
                                height: bt,
                                color: super::theme_adapter::adjust_color(sem.divider_subtle, 1.0),
                            });
                        }
                    }
                }
            }
        }

        "minimap_lane" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.shell_background, 0.95),
            });

            // Reduced minimap bars (larger gap = fewer bars)
            if r.height > 20 && r.width > 6 {
                let bar_h: u32 = 2;
                let bar_gap: u32 = 3;
                let total_bar = bar_h + bar_gap;
                let max_bars = r.height / total_bar;
                let bar_max_w = r.width.saturating_sub(6);
                let bar_x = r.x.saturating_add(3);
                let mut by = r.y.saturating_add(4);

                for i in 0..max_bars {
                    let pct = match i % 6 {
                        0 => 0.88,
                        1 => 0.40,
                        2 => 0.68,
                        3 => 0.54,
                        4 => 0.84,
                        _ => 0.32,
                    };
                    let w = ((bar_max_w as f64) * pct) as u32;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: bar_x,
                        y: by,
                        width: cmp::max(2, w),
                        height: bar_h,
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.28),
                    });
                    by = by.saturating_add(total_bar);
                }
            }

            // Left separator (1 px)
            if r.width > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: bt,
                    height: r.height,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.65),
                });
            }
        }

        "center_bottom_panel" => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.panel_background, 1.0),
            });

            // Top separator (1 px)
            if r.height > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: bt,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.85),
                });
            }

            // Terminal tabs header
            let tab_header_h: u32 = cmp::min(26, r.height / 3);
            if tab_header_h > 0 && r.width > 40 {
                let header_y = r.y.saturating_add(bt);
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: header_y,
                    width: r.width,
                    height: tab_header_h,
                    color: super::theme_adapter::adjust_color(sem.tab_strip_background, 1.0),
                });

                let tab_count: u32 = 3;
                let tab_pad: u32 = 8;
                let total_pad = tab_pad * (tab_count + 1);
                let tab_w = if r.width > total_pad {
                    (r.width.saturating_sub(total_pad)) / tab_count
                } else {
                    r.width / cmp::max(1, tab_count)
                };
                let mut tx = r.x.saturating_add(tab_pad);
                let tab_y = header_y.saturating_add(3);
                let tab_content_h = tab_header_h.saturating_sub(6);

                for i in 0..tab_count {
                    let active = i == 0;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: tx,
                        y: tab_y,
                        width: tab_w,
                        height: tab_content_h,
                        color: if active {
                            super::theme_adapter::adjust_color(sem.tab_active_background, 1.0)
                        } else {
                            super::theme_adapter::adjust_color(sem.tab_background, 1.0)
                        },
                    });
                    tx = tx.saturating_add(tab_w).saturating_add(tab_pad);
                }

                if r.height > tab_header_h.saturating_add(bt) {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: header_y.saturating_add(tab_header_h),
                        width: r.width,
                        height: bt,
                        color: super::theme_adapter::adjust_color(sem.divider, 0.85),
                    });
                }
            }

            // Terminal body: log lines
            let body_y = r.y.saturating_add(bt).saturating_add(tab_header_h).saturating_add(bt);
            if r.height > body_y.saturating_sub(r.y) && r.width > 40 {
                let available_h =
                    r.height.saturating_sub(body_y.saturating_sub(r.y)).saturating_sub(6);
                let line_h = 11u32;
                let gap = 4u32;
                let lines =
                    if available_h > (line_h + gap) { available_h / (line_h + gap) } else { 0 };
                let mut ly = body_y.saturating_add(4);

                let terminal_lines = [
                    (0.88, super::theme_adapter::adjust_color(sem.syntax_function, 0.85)),
                    (0.68, super::theme_adapter::adjust_color(sem.text_secondary, 0.45)),
                    (0.54, super::theme_adapter::adjust_color(sem.text_secondary, 0.38)),
                    (0.80, super::theme_adapter::parse_hex_color(theme.text_secondary)),
                    (0.44, super::theme_adapter::adjust_color(sem.text_secondary, 0.38)),
                    (0.62, super::theme_adapter::adjust_color(sem.syntax_string, 0.82)),
                ];

                for i in 0..lines {
                    let (pct, color) = terminal_lines[(i as usize) % terminal_lines.len()];
                    let w = ((r.width as f64) * pct) as u32;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(12),
                        y: ly,
                        width: w.saturating_sub(12),
                        height: line_h,
                        color,
                    });
                    ly = ly.saturating_add(line_h).saturating_add(gap);
                }
            }
        }

        _ => {}
    }

    // Text labels
    if r.width > 60 && r.height > 10 {
        let labels: Vec<String> = match region.id {
            "editor_tabs" => vec![
                "main.rs".to_string(),
                "lib.rs".to_string(),
                "mod.rs".to_string(),
                "config.rs".to_string(),
            ],
            "breadcrumb" => vec![
                "src".to_string(),
                "app".to_string(),
                "desktop".to_string(),
                "main.rs".to_string(),
            ],
            "center_bottom_panel" => {
                vec!["Terminal".to_string(), "Problems".to_string(), "Output".to_string()]
            }
            _ => vec![],
        };
        if !labels.is_empty() {
            let inset_x = r.x.saturating_add(8);
            let inset_y = r.y.saturating_add(2);
            let mut text_rects = super::text_adapter::layout_and_publish_text(
                inset_x,
                inset_y,
                r.width.saturating_sub(16),
                r.height.saturating_sub(6),
                &labels,
                theme,
                theme.text_primary,
            );
            rects.append(&mut text_rects);
        }
    }

    rects
}
