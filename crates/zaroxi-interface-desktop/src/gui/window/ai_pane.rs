/*!
AI pane drawing logic.

GUI-8 refinements:
- Add an internal pane header placeholder region
- Stacked card/message blocks
- Optional footer/input strip near bottom
- Geometric and quiet visual language (no text)
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
        color: super::theme_adapter::adjust_brightness(theme.surface, 0.92),
    });

    // Internal header near the top of the AI content
    let header_h: u32 = 34;
    if r.height > header_h + 20 && r.width > 40 {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(8),
            y: r.y.saturating_add(8),
            width: r.width.saturating_sub(16),
            height: header_h,
            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.88),
        });
    }

    // stacked cards/messages
    let _min_cards = 2u32;
    let max_cards = 4u32;
    let cards = max_cards;
    let pad: u32 = 10;
    if r.height > pad.saturating_mul(cards + 2) && r.width > pad * 2 {
        // compute available area below header
        let start_y = r.y.saturating_add(8).saturating_add(header_h).saturating_add(8);
        let available_h = r.y.saturating_add(r.height).saturating_sub(start_y).saturating_sub(12);
        let card_h = if available_h > 0 { available_h / cards } else { 0 };
        let mut cy = start_y;
        for i in 0..cards {
            // card inset and subtle stacked offset
            let inset = 12u32.saturating_add(i.saturating_mul(6));
            let w = r.width.saturating_sub(inset).saturating_sub(12);
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(inset),
                y: cy,
                width: w,
                height: card_h,
                color: super::theme_adapter::adjust_brightness(theme.surface, 0.94 - (i as f64 * 0.02)),
            });

            // thin separator between cards
            if card_h > sep_h {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: r.x.saturating_add(8),
                    y: cy.saturating_add(card_h),
                    width: r.width.saturating_sub(16),
                    height: sep_h,
                    color: super::theme_adapter::adjust_brightness(theme.border_color, 0.86),
                });
            }
            cy = cy.saturating_add(card_h).saturating_add(pad);
        }
    }

    // Optional footer / input strip near bottom if space allows
    let footer_h: u32 = 28;
    if r.height > footer_h.saturating_mul(3) {
        let fy = r.y.saturating_add(r.height).saturating_sub(footer_h).saturating_sub(8);
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(8),
            y: fy,
            width: r.width.saturating_sub(16),
            height: footer_h,
            color: super::theme_adapter::adjust_brightness(theme.surface, 0.97),
        });

        // small input handle decoration
        let handle_w = std::cmp::min(120, r.width.saturating_sub(40));
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x.saturating_add(16),
            y: fy.saturating_add(6),
            width: handle_w,
            height: footer_h.saturating_sub(12),
            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.85),
        });
    }

    // left separator for the pane
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
