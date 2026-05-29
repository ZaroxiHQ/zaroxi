/*!
AI pane drawing logic.

Handles the ai_panel_content region. Receives region + theme and returns DrawRect overlays.
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
        color: super::theme_adapter::adjust_brightness(theme.surface, 0.92),
    });

    // stacked cards inside the AI pane
    let cards: u32 = 3;
    let pad: u32 = 10;
    if r.height > pad.saturating_mul(cards + 1) && r.width > pad * 2 {
        let available_h = r.height.saturating_sub(pad.saturating_mul(cards + 1));
        let card_h = if available_h > 0 { available_h / cards } else { 0 };
        let mut cy = r.y.saturating_add(pad);
        for i in 0..cards {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(pad),
                y: cy,
                width: r.width.saturating_sub(pad * 2),
                height: card_h,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.94 - (i as f64 * 0.02)),
            });
            if card_h > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x,
                    y: cy.saturating_add(card_h),
                    width: r.width,
                    height: sep_h,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.86),
                });
            }
            cy = cy.saturating_add(card_h.saturating_add(pad));
        }
    }

    if r.width > sep_h {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: sep_h,
            height: r.height,
            color: super::theme_adapter::parse_hex_color(theme.border_color),
        });
    }

    rects
}
