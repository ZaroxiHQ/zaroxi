/*!
AI assistant pane drawing logic.

Phase 1 (architecture): application-owned content path — domain
AiPanelContent → application mapper (into_content_view) →
ContentView → compose_content_view() → WidgetScene.labels.
Structural chrome (cards, buttons, separators) remains desktop-owned.
*/
use zaroxi_application_ai::panel::idle_content_view;
use zaroxi_core_engine_ui::compose_content_view;
use zaroxi_interface_theme::theme::ZaroxiTheme;

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
    work_content: Option<&zaroxi_core_engine_ui::ShellWorkContent>,
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

    // ── Pane header: "Assistant" + action buttons ──
    let pane_header_h: u32 = 26;
    if r.height > pane_header_h + 20 && r.width > 80 {
        // Header background strip
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: pane_header_h,
            color: super::theme_adapter::adjust_color(sem.panel_header_background, 1.0),
        });
        // Bottom separator on header
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off.saturating_add(pane_header_h).saturating_sub(bt),
            width: inset_w,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider, 0.50),
        });

        // Action buttons on right
        let btn_w: u32 = 18;
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
                    color: super::theme_adapter::adjust_color(sem.divider, 0.70),
                });
            }
        }
        y_off = y_off.saturating_add(pane_header_h).saturating_add(10);
    }

    // ── Explanation message card ──
    let msg_h: u32 = 58;
    if y_off.saturating_add(msg_h) < r.y.saturating_add(r.height).saturating_sub(160) {
        // Card background
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: msg_h,
            color: super::theme_adapter::adjust_color(sem.nested_surface_background, 1.0),
        });
        // Left accent border strip
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: 3,
            height: msg_h,
            color: super::theme_adapter::adjust_color(sem.accent, 0.62),
        });
        // Card top separator
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider_subtle, 1.0),
        });
        // Card bottom separator
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off.saturating_add(msg_h).saturating_sub(bt),
            width: inset_w,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider_subtle, 1.0),
        });

        // Message text lines
        let msg_rows: &[(f64, &str)] =
            &[(0.80, "default"), (0.58, "default"), (0.66, "default"), (0.42, "default")];
        let row_h: u32 = 10;
        let mut my = y_off.saturating_add(10);
        let mw = inset_w.saturating_sub(26);
        for &(pct, _) in msg_rows {
            if my + row_h > y_off + msg_h - 6 {
                break;
            }
            let w = ((mw as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 16),
                y: my,
                width: cmp_w(w, 0),
                height: row_h,
                color: super::theme_adapter::adjust_color(sem.text_faint, 0.34),
            });
            my = my.saturating_add(row_h).saturating_add(4);
        }
        y_off = y_off.saturating_add(msg_h).saturating_add(10);
    }

    // ── Bullet list summary ──
    let bullet_h: u32 = 48;
    if y_off.saturating_add(bullet_h) < r.y.saturating_add(r.height).saturating_sub(120) {
        let bullet_items: &[(f64, &str)] =
            &[(0.68, "default"), (0.54, "default"), (0.62, "default")];
        let mut by = y_off;
        let bi_h: u32 = 14;
        for &(pct, _) in bullet_items {
            // Bullet dot
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 8),
                y: by.saturating_add(5),
                width: 4,
                height: 4,
                color: super::theme_adapter::adjust_color(sem.accent, 0.66),
            });
            // Text line
            let w = ((inset_w.saturating_sub(32) as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 22),
                y: by.saturating_add(2),
                width: cmp_w(w, 0),
                height: bi_h.saturating_sub(4),
                color: super::theme_adapter::adjust_color(sem.text_faint, 0.32),
            });
            by = by.saturating_add(bi_h);
        }
        y_off = y_off.saturating_add(bullet_h).saturating_add(8);
    }

    // ── Code snippet card ──
    let snippet_h: u32 = 50;
    if y_off.saturating_add(snippet_h) < r.y.saturating_add(r.height).saturating_sub(70) {
        // Card background
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: snippet_h,
            color: super::theme_adapter::adjust_color(sem.nested_surface_background, 1.0),
        });
        // Left accent (green for code)
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: 3,
            height: snippet_h,
            color: super::theme_adapter::adjust_color(sem.syntax_function, 0.68),
        });
        // Card border top
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider_subtle, 1.0),
        });
        // Card border bottom
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off.saturating_add(snippet_h).saturating_sub(bt),
            width: inset_w,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider_subtle, 1.0),
        });

        // Code lines
        let code_rows = [0.76, 0.52, 0.64];
        let mut sy = y_off.saturating_add(10);
        for &pct in &code_rows {
            if sy + 10 > y_off + snippet_h - 8 {
                break;
            }
            let w = ((inset_w.saturating_sub(26) as f64) * pct) as u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad + 16),
                y: sy,
                width: cmp_w(w, 0),
                height: 8,
                color: super::theme_adapter::adjust_color(sem.text_faint, 0.32),
            });
            sy = sy.saturating_add(14);
        }
        y_off = y_off.saturating_add(snippet_h).saturating_add(10);
    }

    // ── Action buttons (Accept / Reject / Edit) ──
    let action_h: u32 = 26;
    if y_off.saturating_add(action_h) < r.y.saturating_add(r.height).saturating_sub(60) {
        let btn_count: u32 = 3;
        let btn_w: u32 = cmp_w(inset_w, btn_count * 52 + 20) / btn_count;
        let act_btn_w = btn_w.saturating_sub(4);
        let total_w = btn_count * btn_w;
        let action_x = r.x.saturating_add(pad + (inset_w.saturating_sub(total_w)) / 2);

        let btn_colors = [
            super::theme_adapter::adjust_color(sem.accent, 0.82),
            super::theme_adapter::adjust_color(sem.divider, 0.50),
            super::theme_adapter::adjust_color(sem.divider, 0.50),
        ];
        for i in 0..btn_count {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: action_x.saturating_add(i * btn_w),
                y: y_off,
                width: act_btn_w,
                height: action_h,
                color: btn_colors[i as usize],
            });
        }
        y_off = y_off.saturating_add(action_h).saturating_add(10);
    }

    // ── Prompt input field ──
    let input_h: u32 = 30;
    if y_off.saturating_add(input_h) < r.y.saturating_add(r.height).saturating_sub(16) {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: inset_w,
            height: input_h,
            color: super::theme_adapter::adjust_color(sem.input_background, 1.0),
        });
        // Hint text placeholder
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad + 10),
            y: y_off.saturating_add(8),
            width: inset_w.saturating_sub(80),
            height: input_h.saturating_sub(16),
            color: super::theme_adapter::adjust_color(sem.divider, 0.58),
        });
        y_off = y_off.saturating_add(input_h).saturating_add(8);
    }

    // ── Model selector + Send footer ──
    let bottom_row_h: u32 = 28;
    if y_off.saturating_add(bottom_row_h) < r.y.saturating_add(r.height).saturating_sub(8) {
        let dropdown_w = inset_w.saturating_mul(2) / 3;
        // Model dropdown
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(pad),
            y: y_off,
            width: dropdown_w,
            height: bottom_row_h,
            color: super::theme_adapter::adjust_color(sem.panel_background, 1.0),
        });
        // Send button
        let send_w: u32 = 44;
        let send_x = r.x.saturating_add(pad).saturating_add(inset_w).saturating_sub(send_w);
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: send_x,
            y: y_off.saturating_add(2),
            width: send_w,
            height: bottom_row_h.saturating_sub(4),
            color: super::theme_adapter::adjust_color(sem.accent, 0.84),
        });
    }

    // Left separator (1 px)
    if r.width > bt {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: bt,
            height: r.height,
            color: super::theme_adapter::adjust_color(sem.divider, 0.85),
        });
    }

    // Text labels via engine-owned ContentView
    if r.width > 120 && r.height > 40 {
        let content = work_content
            .and_then(|wc| wc.ai_panel_content.clone())
            .unwrap_or_else(idle_content_view);
        let title_c = wgpu_f32(super::theme_adapter::parse_hex_color(theme.text_primary));
        let body_c = wgpu_f32(super::theme_adapter::parse_hex_color(theme.text_secondary));
        let krect =
            zaroxi_kernel_math::Rect::new(r.x as f32, r.y as f32, r.width as f32, r.height as f32);
        let scene = compose_content_view(&krect, &content, title_c, body_c);
        let labels: Vec<String> = scene.labels.iter().map(|l| l.label.clone()).collect();
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(14),
            r.y.saturating_add(10),
            r.width.saturating_sub(28),
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

fn wgpu_f32(c: wgpu::Color) -> [f32; 4] {
    [c.r as f32, c.g as f32, c.b as f32, c.a as f32]
}
