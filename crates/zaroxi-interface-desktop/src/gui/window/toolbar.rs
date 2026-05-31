/*!
Top toolbar / chrome band drawing logic.

Phase 3: compact title bar using semantic theme colours, 1 px separator.
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

    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_color(sem.title_bar_background, 1.0),
    });

    // Thin bottom separator
    if r.height > bt {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y.saturating_add(r.height.saturating_sub(bt)),
            width: r.width,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider, 0.8),
        });
    }

    // Right: window controls (minimize, maximize, close)
    if r.width > 120 {
        let ctrl_w: u32 = 16;
        let ctrl_h: u32 = 14;
        let cy = r.y.saturating_add(8);
        let right_edge = r.x.saturating_add(r.width).saturating_sub(10);

        let ctrl_colors = [
            super::theme_adapter::adjust_color(sem.border, 0.88),
            super::theme_adapter::adjust_color(sem.border, 0.94),
            super::theme_adapter::adjust_color(sem.error, 0.90),
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

    // Add text label
    if r.width > 80 && r.height > 14 {
        let labels = vec!["Zaroxi".to_string()];
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(10),
            r.y.saturating_add(4),
            r.width.saturating_sub(20),
            r.height.saturating_sub(8),
            &labels,
            theme,
            theme.text_primary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
