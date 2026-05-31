/*!
Top toolbar / chrome band drawing logic.

Phase 2: refined title bar with Zaroxi brand, control group placeholders,
and tab strip impression.
*/

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let sep_h: u32 = std::cmp::max(2, bt);

    let r = &region.rect;
    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_brightness(theme.surface, 0.90),
    });

    // Bottom separator
    if r.height > sep_h {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y.saturating_add(r.height.saturating_sub(sep_h)),
            width: r.width,
            height: sep_h,
            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.78),
        });
    }

    // Left: Zaroxi brand area
    if r.width > 100 && r.height > 16 {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(12),
            y: r.y.saturating_add(6),
            width: 64,
            height: r.height.saturating_sub(12),
            color: super::theme_adapter::adjust_brightness(theme.surface, 1.04),
        });
    }

    // Center: window title area
    if r.width > 200 {
        let center_x = r.x.saturating_add(r.width / 2 - 40);
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: center_x,
            y: r.y.saturating_add(8),
            width: 80,
            height: r.height.saturating_sub(16),
            color: super::theme_adapter::adjust_brightness(theme.surface, 1.00),
        });
    }

    // Right: window controls (minimize, maximize, close)
    if r.width > 120 {
        let ctrl_w: u32 = 20;
        let ctrl_h: u32 = 18;
        let cy = r.y.saturating_add(10);
        let right_edge = r.x.saturating_add(r.width).saturating_sub(12);

        let ctrl_colors = [
            super::theme_adapter::adjust_brightness(theme.border_color, 0.92),
            super::theme_adapter::adjust_brightness(theme.border_color, 0.96),
            super::theme_adapter::adjust_brightness("#F44336", 0.96),
        ];
        let mut cx = right_edge;
        for i in (0..3u32).rev() {
            cx = cx.saturating_sub(ctrl_w + 4);
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: cx,
                y: cy,
                width: ctrl_w,
                height: ctrl_h,
                color: ctrl_colors[i as usize],
            });
        }
    }

    // Add text labels
    if r.width > 120 && r.height > 20 {
        let labels = vec!["Zaroxi".to_string()];
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(16),
            r.y.saturating_add(8),
            r.width,
            r.height.saturating_sub(8),
            &labels,
            theme,
            theme.text_primary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
