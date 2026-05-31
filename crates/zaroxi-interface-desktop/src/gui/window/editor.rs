/*!
Center editor area drawing logic.

Phase 2: editor tabs, breadcrumb path row, syntax-colored code lines
(keyword/function/string/comment/type/number), gutter with line numbers,
minimap impression, and terminal panel.
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
        "editor_tabs" => {
            // Tab strip background
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.98),
            });

            if r.width > 40 && r.height > 0 {
                let tab_h: u32 = r.height;
                let tab_count: u32 = 4;
                let tab_pad: u32 = 2;
                let tab_w = (r.width.saturating_sub(tab_pad * (tab_count + 1))) / tab_count;
                let active_extra = tab_w / 3;
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
                            super::theme_adapter::adjust_brightness(theme.surface, 1.18)
                        } else {
                            super::theme_adapter::adjust_brightness(theme.surface, 1.06)
                        },
                    });
                    tx = tx.saturating_add(w).saturating_add(tab_pad);
                }

                // Bottom separator
                if r.height > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: r.y.saturating_add(r.height.saturating_sub(sep_h)),
                        width: r.width,
                        height: sep_h,
                        color: super::theme_adapter::adjust_brightness(theme.border_color, 0.82),
                    });
                }
            }
        }

        "breadcrumb" => {
            // Breadcrumb row background
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.96),
            });

            // Breadcrumb path segments (e.g. src > main.rs)
            if r.width > 120 && r.height > 12 {
                let seg_count: u32 = 4;
                let seg_pad: u32 = 12;
                let cy = r.y.saturating_add(5);
                let seg_h: u32 = 14;
                let mut sx = r.x.saturating_add(12);
                let seg_widths = [28u32, 20, 20, 48]; // src > app > desktop > main.rs

                for i in 0..seg_count {
                    if sx + seg_widths[i as usize] < r.x + r.width {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: sx,
                            y: cy,
                            width: seg_widths[i as usize],
                            height: seg_h,
                            color: super::theme_adapter::adjust_brightness(theme.surface, 1.04),
                        });
                        sx = sx.saturating_add(seg_widths[i as usize]);
                        // separator chevron
                        if i < seg_count - 1 && sx + 6 < r.x + r.width {
                            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                                x: sx,
                                y: cy.saturating_add(2),
                                width: seg_pad.saturating_sub(4),
                                height: seg_h.saturating_sub(4),
                                color: super::theme_adapter::adjust_brightness(
                                    theme.border_color,
                                    0.78,
                                ),
                            });
                            sx = sx.saturating_add(seg_pad);
                        }
                    }
                }
            }

            // Bottom separator
            if r.height > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y.saturating_add(r.height.saturating_sub(sep_h)),
                    width: r.width,
                    height: sep_h,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.82),
                });
            }
        }

        "center_editor" => {
            // Editor background
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.10),
            });

            // Gutter area
            let gutter_w: u32 = 52;
            let content_x = if r.width > gutter_w { r.x.saturating_add(gutter_w) } else { r.x };
            let content_w = if r.width > gutter_w { r.width.saturating_sub(gutter_w) } else { 0 };

            // Gutter background
            if gutter_w > 0 && r.height > 0 && r.width > gutter_w {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: gutter_w,
                    height: r.height,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
                });

                // Gutter divider
                if gutter_w > sep_h {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: content_x.saturating_sub(sep_h),
                        y: r.y,
                        width: sep_h,
                        height: r.height,
                        color: super::theme_adapter::adjust_brightness(theme.border_color, 0.80),
                    });
                }

                // Line numbers in gutter
                let num_w: u32 = 30;
                let num_h: u32 = 17;
                let num_x = r.x.saturating_add(8);
                let mut num_y = r.y.saturating_add(8);
                let max_nums = r.height / (num_h + 1);
                for _ in 0..max_nums {
                    if num_y + num_h > r.y + r.height {
                        break;
                    }
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: num_x,
                        y: num_y,
                        width: num_w,
                        height: num_h.saturating_sub(2),
                        color: super::theme_adapter::adjust_brightness(theme.border_color, 0.72),
                    });
                    num_y = num_y.saturating_add(num_h + 1);
                }
            }

            // Code lines with syntax-colored segments
            if content_w > 0 && r.height > 0 {
                let line_h: u32 = 17;
                let line_gap: u32 = 3;
                let total_line_h = line_h + line_gap;
                let max_lines = r.height / total_line_h;
                let mut ly = r.y.saturating_add(8);

                // Define a pattern of syntax-colored code lines simulating real Rust code
                // Each line is a set of (width_pct, color_brightness, syntax_type)
                // Types: 0=keyword(red), 1=function(green), 2=string(orange), 3=comment(gray), 4=type(blue), 5=number(purple)
                let code_lines: &[&[(f64, &str)]] = &[
                    // use std::collections::HashMap;
                    &[
                        (0.08, "keyword"),
                        (0.26, "default"),
                        (0.04, "default"),
                        (0.30, "type"),
                        (0.04, "default"),
                    ],
                    // use crate::app::{App, Config};
                    &[
                        (0.08, "keyword"),
                        (0.24, "default"),
                        (0.04, "default"),
                        (0.18, "default"),
                        (0.04, "default"),
                    ],
                    // (empty line)
                    &[(0.06, "default")],
                    // pub fn main() -> Result<()> {
                    &[
                        (0.06, "keyword"),
                        (0.14, "function"),
                        (0.04, "default"),
                        (0.06, "keyword"),
                        (0.18, "type"),
                        (0.06, "default"),
                    ],
                    //     let config = Config::default();
                    &[
                        (0.10, "keyword"),
                        (0.10, "default"),
                        (0.04, "default"),
                        (0.22, "type"),
                        (0.04, "default"),
                        (0.04, "default"),
                    ],
                    //     let app = App::new(config);
                    &[
                        (0.10, "keyword"),
                        (0.10, "default"),
                        (0.04, "default"),
                        (0.30, "type"),
                        (0.04, "default"),
                    ],
                    //     app.run().await?;
                    &[
                        (0.10, "default"),
                        (0.14, "function"),
                        (0.04, "default"),
                        (0.08, "keyword"),
                        (0.04, "default"),
                    ],
                    // }
                    &[(0.04, "default")],
                    // (empty line)
                    &[(0.06, "default")],
                    // struct App {
                    &[(0.12, "keyword"), (0.18, "type"), (0.06, "default")],
                    //     config: Config,
                    &[(0.18, "default"), (0.04, "default"), (0.28, "type"), (0.04, "default")],
                    //     state: State,
                    &[(0.18, "default"), (0.04, "default"), (0.28, "type"), (0.04, "default")],
                    //     // Application state holder
                    &[(0.12, "comment")],
                    //     running: bool,
                    &[(0.18, "default"), (0.04, "default"), (0.16, "type"), (0.04, "default")],
                    // }
                    &[(0.04, "default")],
                    // (empty line)
                    &[(0.06, "default")],
                    // impl App {
                    &[(0.10, "keyword"), (0.18, "type"), (0.06, "default")],
                    //     fn new(cfg: Config) -> Self {
                    &[
                        (0.14, "function"),
                        (0.10, "default"),
                        (0.04, "default"),
                        (0.24, "type"),
                        (0.06, "keyword"),
                        (0.06, "default"),
                    ],
                    //         Self { config: cfg, state: State::Ready, running: false }
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
                    //     }
                    &[(0.04, "default")],
                    //     async fn run(&mut self) -> Result<()> {
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
                    //         self.running = true;
                    &[(0.14, "default"), (0.04, "default"), (0.14, "number"), (0.04, "default")],
                    //         println!("Application started.");
                    &[(0.10, "function"), (0.04, "default"), (0.42, "string"), (0.04, "default")],
                    //         // Perform initialization steps
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
                            "keyword" => super::theme_adapter::adjust_brightness("#FF6B6B", 0.95),
                            "function" => super::theme_adapter::adjust_brightness("#4CAF50", 0.95),
                            "string" => super::theme_adapter::adjust_brightness("#FFB74D", 0.95),
                            "comment" => super::theme_adapter::adjust_brightness("#7E8794", 0.90),
                            "type" => super::theme_adapter::adjust_brightness("#5B8CFF", 0.95),
                            "number" => super::theme_adapter::adjust_brightness("#B39DDB", 0.95),
                            _ => super::theme_adapter::adjust_brightness(theme.surface, 1.06),
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

                    // Thin divider every few lines (between logical blocks)
                    if line_idx == 1
                        || line_idx == 8
                        || line_idx == 9
                        || line_idx == 14
                        || line_idx == 15
                    {
                        if ly < r.y + r.height {
                            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                                x: content_x,
                                y: ly.saturating_sub(line_gap / 2),
                                width: content_w,
                                height: cmp::max(1, sep_h.saturating_sub(1)),
                                color: super::theme_adapter::adjust_brightness(
                                    theme.border_color,
                                    0.76,
                                ),
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
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.97),
            });

            // Minimap: dense horizontal bars suggesting code shape
            if r.height > 20 && r.width > 10 {
                let bar_h: u32 = 3;
                let bar_gap: u32 = 2;
                let total_bar = bar_h + bar_gap;
                let max_bars = r.height / total_bar;
                let bar_max_w = r.width.saturating_sub(8);
                let bar_x = r.x.saturating_add(4);
                let mut by = r.y.saturating_add(4);

                for i in 0..max_bars {
                    let pct = match i % 6 {
                        0 => 0.90,
                        1 => 0.44,
                        2 => 0.72,
                        3 => 0.58,
                        4 => 0.88,
                        _ => 0.36,
                    };
                    let w = ((bar_max_w as f64) * pct) as u32;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: bar_x,
                        y: by,
                        width: cmp::max(2, w),
                        height: bar_h,
                        color: super::theme_adapter::adjust_brightness(
                            theme.surface,
                            1.04 - (i as f64 * 0.001),
                        ),
                    });
                    by = by.saturating_add(total_bar);
                }
            }

            // Left separator
            if r.width > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: sep_h,
                    height: r.height,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.78),
                });
            }
        }

        "center_bottom_panel" => {
            // Terminal panel background
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.88),
            });

            // Terminal tabs header
            let tab_header_h: u32 = cmp::min(30, r.height / 4);
            if tab_header_h > 0 && r.width > 40 {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: tab_header_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 0.94),
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
                let tab_y = r.y.saturating_add(4);
                let tab_content_h = tab_header_h.saturating_sub(8);

                for i in 0..tab_count {
                    let active = i == 0;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: tx,
                        y: tab_y,
                        width: tab_w,
                        height: tab_content_h,
                        color: if active {
                            super::theme_adapter::adjust_brightness(theme.surface, 1.08)
                        } else {
                            super::theme_adapter::adjust_brightness(theme.surface, 0.98)
                        },
                    });
                    tx = tx.saturating_add(tab_w).saturating_add(tab_pad);
                }

                // Header bottom separator
                if r.height > tab_header_h.saturating_add(sep_h) {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: r.y.saturating_add(tab_header_h),
                        width: r.width,
                        height: sep_h,
                        color: super::theme_adapter::adjust_brightness(theme.border_color, 0.82),
                    });
                }
            }

            // Terminal body: log lines
            let body_y = r.y.saturating_add(tab_header_h).saturating_add(sep_h);
            if r.height > body_y.saturating_sub(r.y) && r.width > 40 {
                let available_h =
                    r.height.saturating_sub(body_y.saturating_sub(r.y)).saturating_sub(8);
                let line_h = 13u32;
                let gap = 6u32;
                let lines =
                    if available_h > (line_h + gap) { available_h / (line_h + gap) } else { 0 };
                let mut ly = body_y.saturating_add(6);

                let terminal_lines = [
                    (0.90, super::theme_adapter::adjust_brightness("#4CAF50", 0.90)),
                    (0.72, super::theme_adapter::adjust_brightness(theme.surface, 1.04)),
                    (0.58, super::theme_adapter::adjust_brightness(theme.surface, 1.02)),
                    (0.84, super::theme_adapter::parse_hex_color(theme.text_secondary)),
                    (0.46, super::theme_adapter::adjust_brightness(theme.surface, 1.02)),
                    (0.66, super::theme_adapter::adjust_brightness("#FFB74D", 0.88)),
                ];

                for i in 0..lines {
                    let (pct, color) = terminal_lines[(i as usize) % terminal_lines.len()];
                    let w = ((r.width as f64) * pct) as u32;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(14),
                        y: ly,
                        width: w.saturating_sub(14),
                        height: line_h,
                        color,
                    });
                    ly = ly.saturating_add(line_h).saturating_add(gap);
                }
            }

            // Top separator
            if r.height > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: sep_h,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.84),
                });
            }
        }

        _ => {}
    }

    // Text labels via Cosmic Text layout
    if r.width > 80 && r.height > 12 {
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
            let inset_y = r.y.saturating_add(4);
            let mut text_rects = super::text_adapter::layout_and_publish_text(
                inset_x,
                inset_y,
                r.width.saturating_sub(16),
                r.height.saturating_sub(8),
                &labels,
                theme,
                theme.text_primary,
            );
            rects.append(&mut text_rects);
        }
    }

    rects
}
