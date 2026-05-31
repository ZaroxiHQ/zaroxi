/*!
AI assistant pane drawing logic.

Phase 3: semantic theme colours, 1 px separators, compact IDE utility panel.
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

    // Pane background
    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_color(sem.assistant_panel_background, 1.0),
    });

    let pad: u32 = 10;
    let inset_w = r.width.saturating_sub(pad * 2);
    let mut y_off = r.y.saturating_add(pad);

    // Pane internal header
    let pane_header_h: u32 = 24;
    if r.height > pane_header_h + 20 && r.width > 80 {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: pane_header_h,
            color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
        });

        // Action buttons
        let btn_w: u32 = 18;
        let btn_gap: u32 = 6;
        if inset_w > btn_w * 2 + btn_gap + 40 {
            let right_edge = r.x.saturating_add(pad).saturating_add(inset_w);
            for i in 0..2u32 {
                let bx = right_edge.saturating_sub((2 - i) * (btn_w + btn_gap));
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: bx,
                    y: y_off.saturating_add(3),
                    width: btn_w,
                    height: pane_header_h.saturating_sub(6),
                    color: super::theme_adapter::adjust_color(sem.divider, 0.82),
                });
            }
        }
        y_off = y_off.saturating_add(pane_header_h).saturating_add(6);
    }

    // Explanation message card
    let msg_h: u32 = 56;
    if y_off.saturating_add(msg_h) < r.y.saturating_add(r.height).saturating_sub(180) {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: msg_h,
            color: super::theme_adapter::adjust_color(sem.nested_surface_background, 1.0),
        });
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: 3,
            height: msg_h,
            color: super::theme_adapter::adjust_color(sem.accent, 0.65),
        });

        let msg_rows: &[(f64, &str)] =
            &[(0.82, "default"), (0.60, "default"), (0.68, "default"), (0.44, "default")];
        let row_h: u32 = 10;
        let mut my = y_off.saturating_add(8);
        let mw = inset_w.saturating_sub(24);
        for &(pct, _) in msg_rows {
            if my + row_h > y_off + msg_h - 4 {
                break;
            }
            let w = ((mw as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 14),
                y: my,
                width: cmp_w(w, 0),
                height: row_h,
                color: super::theme_adapter::adjust_color(sem.text_faint, 0.38),
            });
            my = my.saturating_add(row_h).saturating_add(3);
        }
        y_off = y_off.saturating_add(msg_h).saturating_add(8);
    }

    // Bullet list summary
    let bullet_h: u32 = 46;
    if y_off.saturating_add(bullet_h) < r.y.saturating_add(r.height).saturating_sub(140) {
        let bullet_items: &[(f64, &str)] =
            &[(0.72, "default"), (0.56, "default"), (0.64, "default")];
        let mut by = y_off;
        let bi_h: u32 = 14;
        for &(pct, _) in bullet_items {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 6),
                y: by.saturating_add(4),
                width: 5,
                height: 5,
                color: super::theme_adapter::adjust_color(sem.accent, 0.70),
            });
            let w = ((inset_w.saturating_sub(30) as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 20),
                y: by.saturating_add(2),
                width: cmp_w(w, 0),
                height: bi_h.saturating_sub(4),
                color: super::theme_adapter::adjust_color(sem.text_faint, 0.36),
            });
            by = by.saturating_add(bi_h);
        }
        y_off = y_off.saturating_add(bullet_h).saturating_add(6);
    }

    // Code snippet card
    let snippet_h: u32 = 48;
    if y_off.saturating_add(snippet_h) < r.y.saturating_add(r.height).saturating_sub(90) {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: snippet_h,
            color: super::theme_adapter::adjust_color(sem.nested_surface_background, 1.0),
        });
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: 3,
            height: snippet_h,
            color: super::theme_adapter::adjust_color(sem.syntax_function, 0.70),
        });

        let code_rows = [0.78, 0.54, 0.66];
        let mut sy = y_off.saturating_add(8);
        for &pct in &code_rows {
            if sy + 10 > y_off + snippet_h - 6 {
                break;
            }
            let w = ((inset_w.saturating_sub(24) as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 14),
                y: sy,
                width: cmp_w(w, 0),
                height: 8,
                color: super::theme_adapter::adjust_color(sem.text_faint, 0.36),
            });
            sy = sy.saturating_add(12);
        }
        y_off = y_off.saturating_add(snippet_h).saturating_add(8);
    }

    // Action buttons
    let action_h: u32 = 24;
    if y_off.saturating_add(action_h) < r.y.saturating_add(r.height).saturating_sub(70) {
        let btn_count: u32 = 3;
        let btn_w: u32 = cmp_w(inset_w, btn_count * 56 + 20) / btn_count;
        let act_btns = btn_w.saturating_sub(4);
        let total_w = btn_count * btn_w;
        let action_x = r.x.saturating_add(pad + (inset_w.saturating_sub(total_w)) / 2);

        let btn_colors = [
            super::theme_adapter::adjust_color(sem.accent, 0.85),
            super::theme_adapter::adjust_color(sem.hover_background, 1.6),
            super::theme_adapter::adjust_color(sem.hover_background, 1.6),
        ];
        for i in 0..btn_count {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: action_x.saturating_add(i * btn_w),
                y: y_off,
                width: act_btns,
                height: action_h,
                color: btn_colors[i as usize],
            });
        }
        y_off = y_off.saturating_add(action_h).saturating_add(8);
    }

    // Prompt input row
    let input_h: u32 = 28;
    if y_off.saturating_add(input_h) < r.y.saturating_add(r.height).saturating_sub(20) {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: input_h,
            color: super::theme_adapter::adjust_color(sem.input_background, 1.0),
        });

        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad + 10),
            y: y_off.saturating_add(8),
            width: inset_w.saturating_sub(80),
            height: input_h.saturating_sub(16),
            color: super::theme_adapter::adjust_color(sem.divider, 0.65),
        });

        y_off = y_off.saturating_add(input_h).saturating_add(6);
    }

    // Model dropdown + Send button
    let bottom_row_h: u32 = 26;
    if y_off.saturating_add(bottom_row_h) < r.y.saturating_add(r.height).saturating_sub(12) {
        let dropdown_w = inset_w.saturating_mul(2) / 3;
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: dropdown_w,
            height: bottom_row_h,
            color: super::theme_adapter::adjust_color(sem.panel_background, 1.0),
        });

        let send_w: u32 = 40;
        let send_x = r.x.saturating_add(pad).saturating_add(inset_w).saturating_sub(send_w);
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: send_x,
            y: y_off.saturating_add(2),
            width: send_w,
            height: bottom_row_h.saturating_sub(4),
            color: super::theme_adapter::adjust_color(sem.accent, 0.88),
        });
    }

    // Left separator
    if r.width > bt {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: bt,
            height: r.height,
            color: super::theme_adapter::adjust_color(sem.divider, 0.90),
        });
    }

    // Text labels
    if r.width > 120 && r.height > 40 {
        let labels = vec![
            "Assistant".to_string(),
            "Here are the changes needed to refactor the module:".to_string(),
            "Extract validation logic".to_string(),
            "Add error handling".to_string(),
            "Update tests".to_string(),
            "fn validate(input: &str) -> Result<()> {".to_string(),
            "Accept".to_string(),
            "Reject".to_string(),
            "Edit".to_string(),
            "Ask anything...".to_string(),
            "Claude 3.5 Sonnet".to_string(),
            "Send".to_string(),
        ];
        let inset_x = r.x.saturating_add(12);
        let inset_y = r.y.saturating_add(8);
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            inset_x,
            inset_y,
            r.width.saturating_sub(24),
            r.height.saturating_sub(24),
            &labels,
            theme,
            theme.text_primary,
        );
        rects.append(&mut text_rects);
    }

    rects
}

fn cmp_w(val: u32, min: u32) -> u32 {
    std::cmp::max(val, min)
}
