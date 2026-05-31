/*!
Status bar drawing logic.

Phase 2: refined with multiple info cells (cursor pos, encoding,
line ending, language, diagnostics) matching IDE reference layout.
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
        color: super::theme_adapter::adjust_brightness(theme.surface, 0.88),
    });

    // Top separator
    if r.height > sep_h {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: sep_h,
            color: super::theme_adapter::adjust_brightness(theme.border_color, 0.80),
        });
    }

    // Status info cells (left side)
    if r.width > 120 && r.height > 10 {
        let cell_w = [44u32, 40, 30, 38, 44]; // Ln, Col, Spaces, UTF-8, LF
        let cell_h = r.height.saturating_sub(sep_h).saturating_sub(6);
        let mut cx = r.x.saturating_add(10);
        let cy = r.y.saturating_add(sep_h).saturating_add(3);

        for &w in &cell_w {
            if cx + w < r.x + r.width {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: cx,
                    y: cy,
                    width: w,
                    height: cell_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
                });
                cx = cx.saturating_add(w + 8);
            }
        }
    }

    // Right side: language + formatter info
    if r.width > 200 && r.height > 10 {
        let right_cells = [46u32, 68]; // Rust, rust-analyzer
        let cell_h = r.height.saturating_sub(sep_h).saturating_sub(6);
        let cy = r.y.saturating_add(sep_h).saturating_add(3);
        let mut rx = r.x.saturating_add(r.width).saturating_sub(12);

        for &w in right_cells.iter().rev() {
            rx = rx.saturating_sub(w + 8);
            if rx > r.x {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: rx,
                    y: cy,
                    width: w,
                    height: cell_h,
                    color: super::theme_adapter::adjust_brightness(theme.surface, 1.02),
                });
            }
        }
    }

    // Status text label
    if r.width > 80 && r.height > 12 {
        let status = vec![
            "Ready".to_string(),
            "Ln 22, Col 14".to_string(),
            "UTF-8".to_string(),
            "LF".to_string(),
            "Rust".to_string(),
            "rust-analyzer".to_string(),
        ];
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(10),
            r.y.saturating_add(sep_h).saturating_add(3),
            r.width.saturating_sub(20),
            r.height.saturating_sub(sep_h).saturating_sub(6),
            &status,
            theme,
            theme.text_secondary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
