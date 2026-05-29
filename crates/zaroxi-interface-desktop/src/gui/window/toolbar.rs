/*!
Top toolbar / chrome band drawing logic.

GUI-8 refinements:
- Keep the top chrome stable.
- Add subtle control-group placeholders (small blocks) for visual balance.
*/

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
) -> Vec<zaroxi_core_engine_render_backend::DrawRect> {
    let mut rects: Vec<zaroxi_core_engine_render_backend::DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let sep_h: u32 = std::cmp::max(2, bt);

    let r = &region.rect;
    // Full-width chrome band: use a surface-derived fill (so labels sit on a
    // consistent chrome band rather than a heavy border color). This reduces the
    // impression of the title being another chunky block.
    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_brightness(theme.surface, 0.92),
    });

    // Subtle bottom separator to anchor the toolbar
    if r.height > sep_h {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y.saturating_add(r.height.saturating_sub(sep_h)),
            width: r.width,
            height: sep_h,
            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.80),
        });
    }

    // Control group placeholders (left side)
    let ctrl_h = std::cmp::min(26, r.height.saturating_sub(sep_h).saturating_sub(4));
    if ctrl_h > 0 && r.width > 120 {
        let mut cx = r.x.saturating_add(12);
        let cy = r.y.saturating_add(6);
        for i in 0..3 {
            let w = 28u32;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: cx,
                y: cy,
                width: w,
                height: ctrl_h,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.10 - (i as f64 * 0.03)),
            });
            cx = cx.saturating_add(w).saturating_add(8);
        }
    }

    // Control group placeholders (right side)
    if ctrl_h > 0 && r.width > 200 {
        let mut rx = r.x.saturating_add(r.width).saturating_sub(12);
        let cy = r.y.saturating_add(6);
        for i in 0..2 {
            let w = 32u32;
            rx = rx.saturating_sub(w);
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: rx,
                y: cy,
                width: w,
                height: ctrl_h,
                color: super::theme_adapter::adjust_brightness(theme.surface, 1.06 - (i as f64 * 0.02)),
            });
            rx = rx.saturating_sub(8);
        }
    }

    // Add real label text (UI chrome) via the shared Cosmic Text layout path.
    // Use the high-contrast theme.text_primary token so the title reads clearly.
    if r.width > 120 && r.height > 20 {
        let labels = vec!["Zaroxi".to_string(), "Welcome".to_string()];
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(12),
            r.y.saturating_add(8),
            r.width,
            r.height,
            &labels,
            theme,
            theme.text_primary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
