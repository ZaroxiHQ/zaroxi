/*!
Top toolbar / chrome band drawing logic.
This module receives a single ShellRegion and a Theme reference and returns
the low-level DrawRect overlay rects for that region.
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
        color: super::theme_adapter::parse_hex_color(theme.border_color),
    });

    if r.height > sep_h {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y.saturating_add(r.height.saturating_sub(sep_h)),
            width: r.width,
            height: sep_h,
            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.80),
        });
    }

    rects
}
