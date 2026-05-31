/*!
AI assistant pane drawing logic.

Phase 2: structured assistant panel with header, explanation card,
bullet list summary, code snippet preview, action buttons,
prompt input, model dropdown, and send button.
*/

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let sep_h: u32 = std::cmp::max(2, bt);
    let r = &region.rect;

    // Pane background
    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_brightness(theme.surface, 0.94),
    });

    let pad: u32 = 10;
    let inset_w = r.width.saturating_sub(pad * 2);
    let mut y_off = r.y.saturating_add(pad);

    // --- Pane internal header: "Assistant" title + actions ---
    let pane_header_h: u32 = 28;
    if r.height > pane_header_h + 20 && r.width > 80 {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: pane_header_h,
            color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
        });

        // Action buttons on right side of header (pin, close)
        let btn_w: u32 = 20;
        let btn_gap: u32 = 6;
        if inset_w > btn_w * 2 + btn_gap + 40 {
            let right_edge = r.x.saturating_add(pad).saturating_add(inset_w);
            for i in 0..2u32 {
                let bx = right_edge.saturating_sub((2 - i) * (btn_w + btn_gap));
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: bx,
                    y: y_off.saturating_add(4),
                    width: btn_w,
                    height: pane_header_h.saturating_sub(8),
                    color: super::theme_adapter::adjust_brightness(
                        theme.border_color,
                        0.84 + (i as f64 * 0.02),
                    ),
                });
            }
        }

        y_off = y_off.saturating_add(pane_header_h).saturating_add(6);
    }

    // --- Explanation / message card ---
    let msg_h: u32 = 62;
    if y_off.saturating_add(msg_h) < r.y.saturating_add(r.height).saturating_sub(200) {
        // Card background (slightly rounded-looking via inset)
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: msg_h,
            color: super::theme_adapter::adjust_brightness(theme.surface, 1.04),
        });
        // Card border top
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: sep_h,
            color: super::theme_adapter::adjust_brightness("#5B8CFF", 0.70),
        });

        // Text content rows inside message card
        let msg_rows: &[(f64, &str)] =
            &[(0.86, "default"), (0.64, "default"), (0.72, "default"), (0.48, "default")];
        let row_h: u32 = 12;
        let mut my = y_off.saturating_add(8);
        let mw = inset_w.saturating_sub(20);
        for &(pct, _) in msg_rows {
            if my + row_h > y_off + msg_h - 4 {
                break;
            }
            let w = ((mw as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 10),
                y: my,
                width: cmp_w(w, 0),
                height: row_h,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.08),
            });
            my = my.saturating_add(row_h).saturating_add(3);
        }

        y_off = y_off.saturating_add(msg_h).saturating_add(10);
    }

    // --- Bullet list summary ---
    let bullet_h: u32 = 52;
    if y_off.saturating_add(bullet_h) < r.y.saturating_add(r.height).saturating_sub(160) {
        let bullet_items: &[(f64, &str)] =
            &[(0.78, "default"), (0.60, "default"), (0.68, "default")];
        let mut by = y_off;
        let bi_h: u32 = 16;
        for &(pct, _) in bullet_items {
            // Bullet dot
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 6),
                y: by.saturating_add(4),
                width: 6,
                height: 6,
                color: super::theme_adapter::adjust_brightness(theme.border_color, 1.00),
            });
            // Text line
            let w = ((inset_w.saturating_sub(30) as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 20),
                y: by.saturating_add(2),
                width: cmp_w(w, 0),
                height: bi_h.saturating_sub(4),
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.06),
            });
            by = by.saturating_add(bi_h);
        }
        y_off = y_off.saturating_add(bullet_h).saturating_add(8);
    }

    // --- Code snippet preview card ---
    let snippet_h: u32 = 56;
    if y_off.saturating_add(snippet_h) < r.y.saturating_add(r.height).saturating_sub(110) {
        // Card background with left accent border
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: snippet_h,
            color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
        });
        // Left accent bar
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: 4,
            height: snippet_h,
            color: super::theme_adapter::adjust_brightness("#4CAF50", 0.80),
        });

        // Code lines inside the snippet card
        let code_rows = [0.82, 0.58, 0.70];
        let mut sy = y_off.saturating_add(8);
        for &pct in &code_rows {
            if sy + 12 > y_off + snippet_h - 6 {
                break;
            }
            let w = ((inset_w.saturating_sub(24) as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 14),
                y: sy,
                width: cmp_w(w, 0),
                height: 10,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.06),
            });
            sy = sy.saturating_add(14);
        }

        y_off = y_off.saturating_add(snippet_h).saturating_add(10);
    }

    // --- Action buttons ---
    let action_h: u32 = 26;
    if y_off.saturating_add(action_h) < r.y.saturating_add(r.height).saturating_sub(80) {
        let btn_count: u32 = 3;
        let btn_w: u32 = cmp_w(inset_w, btn_count * 60 + 20) / btn_count;
        let act_btns = btn_w.saturating_sub(4);
        let total_w = btn_count * btn_w;
        let action_x = r.x.saturating_add(pad + (inset_w.saturating_sub(total_w)) / 2);

        let btn_colors = [
            super::theme_adapter::adjust_brightness("#5B8CFF", 0.90),
            super::theme_adapter::adjust_brightness(theme.surface, 1.02),
            super::theme_adapter::adjust_brightness(theme.surface, 1.02),
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
        y_off = y_off.saturating_add(action_h).saturating_add(10);
    }

    // --- Prompt input row ---
    let input_h: u32 = 32;
    if r.height > r.y.saturating_add(r.height).saturating_sub(y_off).saturating_add(30)
        && y_off.saturating_add(input_h) < r.y.saturating_add(r.height).saturating_sub(20)
    {
        // Input field background
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: input_h,
            color: super::theme_adapter::adjust_brightness(theme.surface, 1.04),
        });

        // Prompt text hint inside
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad + 10),
            y: y_off.saturating_add(8),
            width: inset_w.saturating_sub(80),
            height: input_h.saturating_sub(16),
            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.78),
        });

        y_off = y_off.saturating_add(input_h).saturating_add(8);
    }

    // --- Model dropdown + Send button row ---
    let bottom_row_h: u32 = 28;
    if y_off.saturating_add(bottom_row_h) < r.y.saturating_add(r.height).saturating_sub(16) {
        // Model dropdown (left side)
        let dropdown_w = inset_w.saturating_mul(2) / 3;
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: dropdown_w,
            height: bottom_row_h,
            color: super::theme_adapter::adjust_brightness(theme.surface, 0.98),
        });

        // Send button (right side)
        let send_w: u32 = 42;
        let send_x = r.x.saturating_add(pad).saturating_add(inset_w).saturating_sub(send_w);
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: send_x,
            y: y_off.saturating_add(2),
            width: send_w,
            height: bottom_row_h.saturating_sub(4),
            color: super::theme_adapter::adjust_brightness("#5B8CFF", 0.92),
        });
    }

    // Left separator for the pane
    if r.width > sep_h {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: sep_h,
            height: r.height,
            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.82),
        });
    }

    // Text labels via Cosmic Text layout
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

    // Optional input placeholder
    if r.height > 80 && r.width > 160 {
        let input = vec!["Rewrite this to use Result...".to_string()];
        let fx = r.x.saturating_add(14);
        let fy = r.y.saturating_add(r.height).saturating_sub(72);
        let mut foot = super::text_adapter::layout_and_publish_text(
            fx,
            fy,
            r.width.saturating_sub(28),
            28,
            &input,
            theme,
            theme.text_secondary,
        );
        rects.append(&mut foot);
    }

    rects
}

fn cmp_w(val: u32, min: u32) -> u32 {
    std::cmp::max(val, min)
}
