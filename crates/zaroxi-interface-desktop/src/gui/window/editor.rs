/*!
Center editor area drawing logic.

Phase 6: engine content path — text labels flow through
ContentView → compose_content_view() → WidgetScene.labels.
Visual styling (gutter, syntax colors, cursor, minimap) remains desktop-owned.

Phase 3: accepts optional ShellWorkContent for live editor body, tabs, breadcrumb.
*/
use crate::gui::ShellWorkContent;
use crate::gui::region_dispatch::region_role;
use zaroxi_core_engine_ui::HighlightKind;
use zaroxi_core_engine_ui::PanelRole;
use zaroxi_core_engine_ui::{ContentView, compose_content_view};
use zaroxi_interface_theme::theme::SemanticColors;

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
    work_content: Option<&ShellWorkContent>,
    sem: &SemanticColors,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    use std::cmp;
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let r = &region.rect;

    match region_role(region.id) {
        // ── TAB STRIP ───────────────────────────────────────────────
        PanelRole::ContentTabStrip => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.tab_strip_background, 1.0),
            });

            if r.width > 20 && r.height > 4 {
                let tab_h: u32 = r.height;
                let tab_count: u32 = 4;
                let tab_pad: u32 = 2;
                let total_pad = tab_pad * (tab_count + 1);
                let tab_w = if r.width > total_pad {
                    (r.width.saturating_sub(total_pad)) / tab_count
                } else {
                    cmp::max(1, r.width.saturating_sub(total_pad) / cmp::max(1, tab_count))
                };
                let mut tx = r.x.saturating_add(tab_pad);

                for i in 0..tab_count {
                    let is_active = i == 0;
                    let w = if is_active {
                        cmp::min(tab_w.saturating_add(tab_w / 3), r.width.saturating_sub(tx - r.x))
                    } else {
                        tab_w
                    };
                    // Active tab gets top accent line
                    if is_active {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: tx,
                            y: r.y,
                            width: w,
                            height: 2,
                            color: super::theme_adapter::adjust_color(sem.accent, 1.0),
                        });
                    }
                    // Tab body
                    let tab_bg_y = if is_active { r.y.saturating_add(2) } else { r.y };
                    let tab_bg_h = if is_active { tab_h.saturating_sub(2) } else { tab_h };
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: tx,
                        y: tab_bg_y,
                        width: w,
                        height: tab_bg_h,
                        color: if is_active {
                            super::theme_adapter::adjust_color(sem.tab_active_background, 1.0)
                        } else {
                            super::theme_adapter::adjust_color(sem.tab_background, 1.0)
                        },
                    });
                    tx = tx.saturating_add(w).saturating_add(tab_pad);
                }

                // Bottom separator
                if r.height > bt {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: r.y.saturating_add(r.height.saturating_sub(bt)),
                        width: r.width,
                        height: bt,
                        color: super::theme_adapter::adjust_color(sem.divider, 0.88),
                    });
                }
            }
        }

        // ── BREADCRUMB ──────────────────────────────────────────────
        PanelRole::ContentBreadcrumb => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.tab_strip_background, 0.88),
            });

            if r.width > 80 && r.height > 8 {
                let segments = ["src", "app", "desktop", "main.rs"];
                let seg_pad: u32 = 6;
                let cy = r.y.saturating_add(4);
                let seg_h: u32 = 10;
                let arrow_w: u32 = 6;
                let mut sx = r.x.saturating_add(10);

                for (idx, _) in segments.iter().enumerate() {
                    let seg_w: u32 = match idx {
                        0 => 20,
                        1 => 22,
                        2 => 38,
                        _ => 36,
                    };
                    if sx + seg_w > r.x + r.width {
                        break;
                    }
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: sx,
                        y: cy,
                        width: seg_w,
                        height: seg_h,
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.28),
                    });
                    sx = sx.saturating_add(seg_w);
                    // Breadcrumb arrow
                    if idx < segments.len() - 1 && sx + arrow_w + seg_pad < r.x + r.width {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: sx.saturating_add(seg_pad),
                            y: cy.saturating_add(2),
                            width: arrow_w,
                            height: seg_h.saturating_sub(4),
                            color: super::theme_adapter::adjust_color(sem.divider, 0.5),
                        });
                        sx = sx.saturating_add(arrow_w + seg_pad * 2);
                    }
                }
            }

            if r.height > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y.saturating_add(r.height.saturating_sub(bt)),
                    width: r.width,
                    height: bt,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.88),
                });
            }
        }

        // ── EDITOR CANVAS ───────────────────────────────────────────
        PanelRole::ContentArea => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.editor_background, 1.0),
            });

            let gutter_w: u32 = 48;
            let content_x = if r.width > gutter_w { r.x.saturating_add(gutter_w) } else { r.x };
            let content_w = if r.width > gutter_w { r.width.saturating_sub(gutter_w) } else { 0 };

            // ---------- Gutter ----------
            if gutter_w > 0 && r.width > gutter_w {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: gutter_w,
                    height: r.height,
                    color: super::theme_adapter::adjust_color(sem.editor_gutter_background, 1.0),
                });
                // Gutter separator
                if gutter_w > bt {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: content_x.saturating_sub(bt),
                        y: r.y,
                        width: bt,
                        height: r.height,
                        color: super::theme_adapter::adjust_color(sem.divider, 0.55),
                    });
                }
            }

            // Line-height rhythm
            let line_h: u32 = 18;
            let _gap: u32 = 0;
            let max_lines = r.height / line_h;
            let mut ly = r.y.saturating_add(6);
            let current_line_idx: i32 = 7; // highlight line ~7

            // Phase 43: build code segments from syntax_highlights when available,
            // falling back to the hardcoded code_lines array.
            let dynamic_code_lines: Option<Vec<Vec<(f64, &str)>>> =
                work_content.and_then(|wc| wc.syntax_highlights.as_ref()).map(|sh| {
                    sh.highlights
                        .iter()
                        .map(|spans| {
                            if spans.is_empty() {
                                return vec![(1.0, "default")];
                            }
                            let total_chars: f64 = spans
                                .iter()
                                .map(|s| (s.end_col - s.start_col) as f64)
                                .sum::<f64>()
                                .max(1.0);
                            spans
                                .iter()
                                .map(|s| {
                                    let pct = (s.end_col - s.start_col) as f64 / total_chars;
                                    let kind_str: &str = match s.kind {
                                        HighlightKind::Comment => "comment",
                                        HighlightKind::String => "string",
                                        HighlightKind::Keyword => "keyword",
                                        HighlightKind::Function => "function",
                                        HighlightKind::Type => "type",
                                        HighlightKind::Number => "number",
                                        HighlightKind::Constant => "number",
                                        HighlightKind::Variable => "default",
                                        HighlightKind::Operator => "default",
                                        HighlightKind::Attribute => "default",
                                        HighlightKind::Plain => "default",
                                    };
                                    (pct, kind_str)
                                })
                                .collect()
                        })
                        .collect()
                });

            let hardcoded_fallback: &[&[(f64, &str)]] = &[
                // 0: module-level structure
                &[
                    (0.06, "keyword"),
                    (0.22, "default"),
                    (0.04, "default"),
                    (0.28, "type"),
                    (0.04, "default"),
                ],
                // 1
                &[
                    (0.08, "keyword"),
                    (0.16, "default"),
                    (0.04, "default"),
                    (0.20, "default"),
                    (0.04, "default"),
                ],
                // 2: blank
                &[],
                // 3: fn main with return type
                &[
                    (0.04, "keyword"),
                    (0.12, "function"),
                    (0.04, "default"),
                    (0.06, "keyword"),
                    (0.18, "type"),
                    (0.06, "default"),
                    (0.04, "default"),
                ],
                // 4: generic constraint
                &[
                    (0.08, "keyword"),
                    (0.10, "default"),
                    (0.04, "default"),
                    (0.24, "type"),
                    (0.04, "default"),
                    (0.04, "default"),
                ],
                // 5: where clause style
                &[
                    (0.08, "keyword"),
                    (0.20, "default"),
                    (0.04, "default"),
                    (0.28, "type"),
                    (0.04, "default"),
                ],
                // 6: expression with method call
                &[
                    (0.08, "default"),
                    (0.14, "function"),
                    (0.04, "default"),
                    (0.10, "keyword"),
                    (0.04, "default"),
                ],
                // 7: CURRENT LINE (highlighted)
                &[
                    (0.14, "keyword"),
                    (0.22, "type"),
                    (0.06, "default"),
                    (0.04, "default"),
                    (0.12, "default"),
                ],
                // 8: blank
                &[],
                // 9: impl block
                &[(0.06, "keyword"), (0.20, "type"), (0.06, "default")],
                // 10: struct field with type
                &[(0.14, "default"), (0.04, "default"), (0.28, "type"), (0.04, "default")],
                // 11: another field
                &[(0.14, "default"), (0.04, "default"), (0.26, "type"), (0.04, "default")],
                // 12: comment
                &[(0.10, "comment")],
                // 13: method definition
                &[(0.18, "default"), (0.04, "default"), (0.18, "type"), (0.04, "default")],
                // 14: return expr
                &[
                    (0.16, "keyword"),
                    (0.08, "default"),
                    (0.04, "default"),
                    (0.04, "default"),
                    (0.10, "function"),
                    (0.04, "default"),
                    (0.04, "default"),
                ],
                // 15: blank
                &[],
                // 16: closing
                &[(0.04, "default")],
                // 17: top-level calling code
                &[(0.10, "keyword"), (0.20, "type"), (0.06, "default")],
                // 18: let binding
                &[
                    (0.04, "keyword"),
                    (0.08, "default"),
                    (0.06, "default"),
                    (0.04, "default"),
                    (0.16, "type"),
                    (0.04, "default"),
                    (0.04, "default"),
                    (0.04, "default"),
                    (0.18, "type"),
                    (0.04, "default"),
                ],
                // 19: method call chain
                &[
                    (0.08, "default"),
                    (0.18, "function"),
                    (0.04, "default"),
                    (0.14, "keyword"),
                    (0.08, "default"),
                    (0.06, "default"),
                    (0.08, "default"),
                ],
                // 20: another call
                &[
                    (0.14, "default"),
                    (0.04, "default"),
                    (0.16, "number"),
                    (0.04, "default"),
                    (0.04, "default"),
                ],
                // 21: string / match
                &[(0.10, "function"), (0.04, "default"), (0.48, "string"), (0.04, "default")],
                // 22: comment
                &[(0.12, "comment")],
            ];

            let using_dynamic = dynamic_code_lines.is_some();
            let code_segments: Vec<Vec<(f64, &str)>> = dynamic_code_lines
                .unwrap_or_else(|| hardcoded_fallback.iter().map(|s| s.to_vec()).collect());

            for (line_idx, segments) in code_segments.iter().enumerate() {
                if ly + line_h > r.y + r.height || line_idx >= max_lines as usize {
                    break;
                }

                // Current-line highlight
                if line_idx == current_line_idx as usize {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: ly,
                        width: r.width,
                        height: line_h,
                        color: super::theme_adapter::adjust_color(sem.editor_line_highlight, 2.4),
                    });
                    // Cursor block
                    let cursor_x = content_x.saturating_add(40);
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: cursor_x,
                        y: ly.saturating_add(2),
                        width: 2,
                        height: line_h.saturating_sub(4),
                        color: super::theme_adapter::adjust_color(sem.editor_cursor, 1.0),
                    });
                }

                // Line numbers in gutter
                if gutter_w > 0 {
                    let num_w: u32 = 30;
                    let num_h = line_h.saturating_sub(4);
                    let is_current = line_idx == current_line_idx as usize;
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(8),
                        y: ly.saturating_add(2),
                        width: num_w,
                        height: num_h,
                        color: if is_current {
                            super::theme_adapter::adjust_color(sem.text_secondary, 0.30)
                        } else {
                            super::theme_adapter::adjust_color(sem.text_faint, 0.24)
                        },
                    });
                }

                // Code segments (indent derived from content for dynamic, hardcoded for fallback)
                let indent_level = if using_dynamic {
                    0 // Dynamic content: no hardcoded indentation
                } else {
                    match line_idx {
                        0..=2 => 0,
                        3..=6 => 1,
                        7..=8 => 0,
                        9..=11 => 1,
                        12 => 1,
                        13..=14 => 2,
                        15..=16 => 1,
                        17..=18 => 0,
                        19..=20 => 1,
                        _ => 0,
                    }
                };
                let indent_px: u32 = indent_level * 16;
                let mut seg_x = content_x.saturating_add(8).saturating_add(indent_px);
                let remaining_w = content_w.saturating_sub(16).saturating_sub(indent_px);

                for &(pct, syntax_type) in segments.iter() {
                    let seg_w = ((remaining_w as f64) * pct.min(1.0)) as u32;
                    if seg_w == 0 {
                        continue;
                    }
                    let color = match syntax_type {
                        "keyword" => super::theme_adapter::adjust_color(sem.syntax_keyword, 0.92),
                        "function" => super::theme_adapter::adjust_color(sem.syntax_function, 0.92),
                        "string" => super::theme_adapter::adjust_color(sem.syntax_string, 0.92),
                        "comment" => super::theme_adapter::adjust_color(sem.syntax_comment, 0.82),
                        "type" => super::theme_adapter::adjust_color(sem.syntax_type, 0.92),
                        "number" => super::theme_adapter::adjust_color(sem.syntax_number, 0.92),
                        _ => super::theme_adapter::adjust_color(sem.text_secondary, 0.38),
                    };
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: seg_x,
                        y: ly.saturating_add(2),
                        width: seg_w,
                        height: line_h.saturating_sub(4),
                        color,
                    });
                    seg_x = seg_x.saturating_add(seg_w);
                }

                ly = ly.saturating_add(line_h);

                // Block-level separator (hardcoded fallback only)
                if !using_dynamic && (line_idx == 2 || line_idx == 8 || line_idx == 15) {
                    if ly < r.y + r.height {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: content_x,
                            y: ly,
                            width: content_w,
                            height: bt,
                            color: super::theme_adapter::adjust_color(sem.divider_subtle, 1.0),
                        });
                    }
                }
            }
        }

        // ── MINIMAP ─────────────────────────────────────────────────
        PanelRole::MinimapLane => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.shell_background, 0.92),
            });

            if r.height > 24 && r.width > 4 {
                let bar_h: u32 = 3;
                let bar_gap: u32 = 2;
                let total_h = bar_h + bar_gap;
                let max_bars = r.height / total_h;
                let bar_max_w = r.width.saturating_sub(8);
                let bar_x = r.x.saturating_add(4);
                let mut by = r.y.saturating_add(6);

                for i in 0..max_bars {
                    let pct = match i % 7 {
                        0 => 0.84,
                        1 => 0.38,
                        2 => 0.62,
                        3 => 0.50,
                        4 => 0.80,
                        5 => 0.44,
                        _ => 0.28,
                    };
                    let w = cmp::max(2, ((bar_max_w as f64) * pct) as u32);
                    // Color varies: green for functions, blue for types, white for others
                    let color = match (i as usize) % 7 {
                        1 | 4 => super::theme_adapter::adjust_color(sem.syntax_function, 0.40),
                        3 => super::theme_adapter::adjust_color(sem.syntax_type, 0.40),
                        _ => super::theme_adapter::adjust_color(sem.text_faint, 0.22),
                    };
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: bar_x,
                        y: by,
                        width: w,
                        height: bar_h,
                        color,
                    });
                    by = by.saturating_add(total_h);
                }

                // Viewport indicator (shows current scroll position)
                if r.height > 60 {
                    let vp_h: u32 = 26;
                    let vp_y = r.y.saturating_add(r.height / 3);
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x.saturating_add(1),
                        y: vp_y,
                        width: r.width.saturating_sub(2),
                        height: vp_h,
                        color: super::theme_adapter::adjust_color(sem.text_faint, 0.06),
                    });
                }
            }

            // Left separator
            if r.width > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: bt,
                    height: r.height,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.55),
                });
            }
        }

        // ── BOTTOM DOCK (terminal) ──────────────────────────────────
        PanelRole::BottomPanel => {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                color: super::theme_adapter::adjust_color(sem.panel_background, 1.0),
            });

            // Top separator
            if r.height > bt {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: bt,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.85),
                });
            }

            // Terminal tab header
            let tab_hdr_h: u32 = cmp::min(26, r.height / 3);
            if tab_hdr_h > 0 && r.width > 40 {
                let hdr_y = r.y.saturating_add(bt);
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: hdr_y,
                    width: r.width,
                    height: tab_hdr_h,
                    color: super::theme_adapter::adjust_color(sem.tab_strip_background, 1.0),
                });

                let tabs: u32 = 3;
                let tab_pad: u32 = 10;
                let total_pad = tab_pad * (tabs + 1);
                let tab_w = if r.width > total_pad {
                    (r.width.saturating_sub(total_pad)) / tabs
                } else {
                    r.width / cmp::max(1, tabs)
                };
                let mut tx = r.x.saturating_add(tab_pad);
                let tab_y = hdr_y.saturating_add(4);
                let tab_content_h = tab_hdr_h.saturating_sub(4);

                for i in 0..tabs {
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
                    // Active tab gets bottom accent line
                    if active {
                        rects.push(zaroxi_core_engine_render_backend::DrawRect {
                            x: tx,
                            y: tab_y.saturating_add(tab_content_h).saturating_sub(2),
                            width: tab_w,
                            height: 2,
                            color: super::theme_adapter::adjust_color(sem.accent, 0.9),
                        });
                    }
                    tx = tx.saturating_add(tab_w).saturating_add(tab_pad);
                }

                if r.height > tab_hdr_h.saturating_add(bt) {
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: r.x,
                        y: hdr_y.saturating_add(tab_hdr_h),
                        width: r.width,
                        height: bt,
                        color: super::theme_adapter::adjust_color(sem.divider, 0.8),
                    });
                }
            }

            // Terminal body: output lines with prompt
            let body_y = r.y.saturating_add(bt).saturating_add(tab_hdr_h).saturating_add(bt);
            if r.height > body_y.saturating_sub(r.y).saturating_add(10) && r.width > 40 {
                let available_h =
                    r.height.saturating_sub(body_y.saturating_sub(r.y)).saturating_sub(8);
                let line_h = 12u32;
                let gap = 3u32;
                let max_out =
                    if available_h > (line_h + gap) { available_h / (line_h + gap) } else { 0 };
                let mut ly = body_y.saturating_add(4);
                let pad_x = r.x.saturating_add(12);

                // Prompt line
                {
                    // Green `$` prompt
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: pad_x,
                        y: ly,
                        width: 8,
                        height: line_h,
                        color: super::theme_adapter::adjust_color(sem.syntax_function, 0.86),
                    });
                    // Command text
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: pad_x.saturating_add(12),
                        y: ly,
                        width: 110,
                        height: line_h,
                        color: super::theme_adapter::adjust_color(sem.text_secondary, 0.48),
                    });
                    ly = ly.saturating_add(line_h).saturating_add(gap);
                }

                for i in 0..max_out {
                    let (pct, color) = match i % 5 {
                        0 => (0.82, super::theme_adapter::adjust_color(sem.syntax_function, 0.78)),
                        1 => (0.66, super::theme_adapter::adjust_color(sem.text_secondary, 0.40)),
                        2 => (0.72, super::theme_adapter::adjust_color(sem.text_secondary, 0.44)),
                        3 => (0.90, super::theme_adapter::adjust_color(sem.syntax_string, 0.74)),
                        _ => (0.46, super::theme_adapter::adjust_color(sem.text_faint, 0.28)),
                    };
                    let w = cmp::max(20, ((r.width.saturating_sub(24) as f64) * pct) as u32);
                    rects.push(zaroxi_core_engine_render_backend::DrawRect {
                        x: pad_x,
                        y: ly,
                        width: w,
                        height: line_h,
                        color,
                    });
                    ly = ly.saturating_add(line_h).saturating_add(gap);
                }
            }
        }

        _ => {}
    }

    // Label text for tabs / breadcrumb / bottom dock
    if r.width > 40 && r.height > 8 {
        let labels: Vec<String> = match region_role(region.id) {
            PanelRole::ContentTabStrip => {
                work_content.and_then(|wc| wc.editor_tabs.clone()).unwrap_or_else(|| {
                    vec![
                        "main.rs".to_string(),
                        "lib.rs".to_string(),
                        "mod.rs".to_string(),
                        "config.rs".to_string(),
                    ]
                })
            }
            PanelRole::ContentBreadcrumb => work_content
                .and_then(|wc| wc.editor_breadcrumb.clone())
                .map(|b| b.split(" > ").map(|s| s.to_string()).collect::<Vec<_>>())
                .unwrap_or_else(|| {
                    vec![
                        "src".to_string(),
                        "app".to_string(),
                        "desktop".to_string(),
                        "main.rs".to_string(),
                    ]
                }),
            PanelRole::BottomPanel => {
                vec!["Terminal".to_string(), "Problems".to_string(), "Output".to_string()]
            }
            PanelRole::ContentArea => {
                let content =
                    work_content.and_then(|wc| wc.editor_body.clone()).unwrap_or_else(|| {
                        ContentView::new(
                            "main.rs",
                            "src/app/",
                            vec![
                                "fn main() {".into(),
                                "    println!(\"hello\");".into(),
                                "}".into(),
                            ],
                        )
                    });
                let title_c = wgpu_f32(super::theme_adapter::parse_hex_color(theme.text_primary));
                let body_c = wgpu_f32(super::theme_adapter::parse_hex_color(theme.text_secondary));
                let krect = zaroxi_kernel_math::Rect::new(
                    r.x as f32,
                    r.y as f32,
                    r.width as f32,
                    r.height as f32,
                );
                let scene = compose_content_view(&krect, &content, title_c, body_c);
                scene.labels.iter().map(|l| l.label.clone()).collect()
            }
            _ => vec![],
        };
        if !labels.is_empty() {
            let (lx, ly) = match region_role(region.id) {
                PanelRole::ContentArea => (r.x.saturating_add(8), r.y.saturating_add(6)),
                _ => (r.x.saturating_add(8), r.y.saturating_add(2)),
            };
            let syntax = work_content.and_then(|wc| wc.syntax_highlights.as_ref());
            let mut text_rects =
                super::text_adapter::layout_text_with_syntax(lx, ly, &labels, syntax, theme);
            rects.append(&mut text_rects);
        }
    }

    rects
}

fn wgpu_f32(c: wgpu::Color) -> [f32; 4] {
    [c.r as f32, c.g as f32, c.b as f32, c.a as f32]
}
