/*!
Status bar drawing logic.

Receives region + theme and returns DrawRect overlays for the bottom status bar.
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

    if r.height > sep_h {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: sep_h,
            color: super::theme_adapter::parse_hex_color(theme.border_color),
        });
    }

    // Small status label via the shared text layout path.
    if r.width > 80 && r.height > 12 {
        let status = vec!["Ready".to_string()];
        let mut text_rects =
            super::text_adapter::layout_and_publish_text(r.x.saturating_add(8), r.y.saturating_add(4), r.width.saturating_sub(16), r.height.saturating_sub(8), &status, theme);
        rects.append(&mut text_rects);
    }

    rects
}
