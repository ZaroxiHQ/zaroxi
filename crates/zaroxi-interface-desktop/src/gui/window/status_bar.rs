/*!
Status bar drawing logic.

Phase 4: product-parity status bar — info cells, divider dots,
language badge on right, slim 1 px separator.
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

    // Background
    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_color(sem.status_bar_background, 1.0),
    });

    // Top separator
    if r.height > bt {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider, 0.90),
        });
    }

    // Left info cells with divider dots between them
    if r.width > 120 && r.height > 8 {
        let cells = [(36u32, "Ready"), (52, "Ln 22, Col 14"), (36, "UTF-8"), (28, "LF")];
        let cell_h = r.height.saturating_sub(bt).saturating_sub(4);
        let mut cx = r.x.saturating_add(20);
        let cy = r.y.saturating_add(bt).saturating_add(2);

        for (idx, &(w, _label)) in cells.iter().enumerate() {
            if cx + w > r.x + r.width {
                break;
            }
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: cx,
                y: cy,
                width: w,
                height: cell_h,
                color: super::theme_adapter::adjust_color(sem.text_faint, 0.14),
            });
            cx = cx.saturating_add(w);
            // Divider dot between cells
            if idx < cells.len() - 1 && cx + 6 < r.x + r.width {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: cx.saturating_add(3),
                    y: cy.saturating_add(cell_h / 2).saturating_sub(1),
                    width: 2,
                    height: 2,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.6),
                });
                cx = cx.saturating_add(10);
            }
        }
    }

    // Right side: language badge (Rust) + formatter (rust-analyzer)
    if r.width > 200 && r.height > 8 {
        let badge_w: u32 = 42;
        let badge_h = r.height.saturating_sub(bt).saturating_sub(4);
        let cy = r.y.saturating_add(bt).saturating_add(2);
        let mut rx = r.x.saturating_add(r.width).saturating_sub(16);

        // Formatter badge
        rx = rx.saturating_sub(68);
        if rx > r.x {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: rx,
                y: cy,
                width: 68,
                height: badge_h,
                color: super::theme_adapter::adjust_color(sem.text_faint, 0.12),
            });
        }
        // dot separator
        rx = rx.saturating_sub(8);
        if rx > r.x {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: rx.saturating_sub(2),
                y: cy.saturating_add(badge_h / 2).saturating_sub(1),
                width: 2,
                height: 2,
                color: super::theme_adapter::adjust_color(sem.divider, 0.6),
            });
        }
        // Language badge
        rx = rx.saturating_sub(badge_w + 4);
        if rx > r.x {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: rx,
                y: cy,
                width: badge_w,
                height: badge_h,
                color: super::theme_adapter::adjust_color(sem.accent_soft_background, 2.2),
            });
        }
    }

    // Text labels placed inline with cell positions
    if r.width > 80 && r.height > 10 {
        let status = vec![
            "Ready".to_string(),
            "Ln 22, Col 14".to_string(),
            "UTF-8".to_string(),
            "LF".to_string(),
            "Rust".to_string(),
            "rust-analyzer".to_string(),
        ];
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(20),
            r.y.saturating_add(bt).saturating_add(2),
            r.width.saturating_sub(40),
            r.height.saturating_sub(bt).saturating_sub(4),
            &status,
            theme,
            theme.text_secondary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
