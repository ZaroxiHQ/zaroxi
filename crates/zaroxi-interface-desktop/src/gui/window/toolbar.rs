/*!
Top toolbar / titlebar drawing logic.

Phase 4: product-parity titlebar — brand left, action icons center row,
window control dots right, thin bottom separator.
*/
use zaroxi_interface_theme::theme::SemanticColors;

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
    sem: &SemanticColors,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let r = &region.rect;

    // Title bar background
    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_color(sem.title_bar_background, 1.0),
    });

    // Brand area (left side with accent bar)
    if r.width > 60 && r.height > 8 {
        let brand_x = r.x.saturating_add(10);
        let brand_y = r.y.saturating_add(5);
        let brand_h = r.height.saturating_sub(10);
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: brand_x,
            y: brand_y,
            width: 32,
            height: brand_h,
            color: super::theme_adapter::adjust_color(sem.accent, 0.82),
        });

        // File/edit/selection/view/go/run icons (simple dots to right of brand)
        if r.width > 300 {
            let mut dot_x = brand_x.saturating_add(48);
            let dot_y = r.y.saturating_add(9);
            let dot_h: u32 = 12;
            for i in 0..6u32 {
                let w: u32 = if i == 0 { 18 } else { 14 };
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: dot_x,
                    y: dot_y,
                    width: w,
                    height: dot_h,
                    color: super::theme_adapter::adjust_color(sem.text_faint, 0.24),
                });
                dot_x = dot_x.saturating_add(w).saturating_add(10);
            }
        }
    }

    // Window controls (right side): minimize | maximize | close
    if r.width > 120 && r.height > 10 {
        let ctrl_r: u32 = 6;
        let cy = r.y.saturating_add(r.height / 2).saturating_sub(ctrl_r);
        let right_edge = r.x.saturating_add(r.width).saturating_sub(10);

        let ctrl_colors = [
            super::theme_adapter::adjust_color(sem.success, 0.70),
            super::theme_adapter::adjust_color(sem.warning, 0.60),
            super::theme_adapter::adjust_color(sem.error, 0.72),
        ];
        let mut cx = right_edge;
        for i in (0..3u32).rev() {
            cx = cx.saturating_sub(ctrl_r * 2 + 8);
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: cx,
                y: cy,
                width: ctrl_r * 2,
                height: ctrl_r * 2,
                color: ctrl_colors[i as usize],
            });
        }
    }

    // Bottom separator
    if r.height > bt {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y.saturating_add(r.height.saturating_sub(bt)),
            width: r.width,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider, 0.8),
        });
    }

    // Brand text label
    if r.width > 80 && r.height > 12 {
        let labels = vec!["Zaroxi".to_string()];
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(10),
            r.y.saturating_add(3),
            r.width.saturating_sub(80),
            r.height.saturating_sub(6),
            &labels,
            theme,
            theme.text_primary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
