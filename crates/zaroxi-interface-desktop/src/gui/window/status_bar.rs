/*!
Status bar drawing logic.

Phase 3: semantic theme colours, 1 px separator, compact IDE status bar.
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
        color: super::theme_adapter::adjust_color(sem.status_bar_background, 1.0),
    });

    // Top separator (1 px)
    if r.height > bt {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider, 0.90),
        });
    }

    // Status info cells (left side)
    if r.width > 120 && r.height > 8 {
        let cell_w = [42u32, 36, 28, 36, 28];
        let cell_h = r.height.saturating_sub(bt).saturating_sub(6);
        let mut cx = r.x.saturating_add(8);
        let cy = r.y.saturating_add(bt).saturating_add(3);

        for &w in &cell_w {
            if cx + w < r.x + r.width {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: cx,
                    y: cy,
                    width: w,
                    height: cell_h,
                    color: super::theme_adapter::adjust_color(sem.text_faint, 0.16),
                });
                cx = cx.saturating_add(w + 8);
            }
        }
    }

    // Right side: language + formatter info
    if r.width > 200 && r.height > 8 {
        let right_cells = [42u32, 64];
        let cell_h = r.height.saturating_sub(bt).saturating_sub(6);
        let cy = r.y.saturating_add(bt).saturating_add(3);
        let mut rx = r.x.saturating_add(r.width).saturating_sub(10);

        for &w in right_cells.iter().rev() {
            rx = rx.saturating_sub(w + 8);
            if rx > r.x {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: rx,
                    y: cy,
                    width: w,
                    height: cell_h,
                    color: super::theme_adapter::adjust_color(sem.text_faint, 0.16),
                });
            }
        }
    }

    // Status text labels
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
            r.x.saturating_add(8),
            r.y.saturating_add(bt).saturating_add(2),
            r.width.saturating_sub(16),
            r.height.saturating_sub(bt).saturating_sub(4),
            &status,
            theme,
            theme.text_secondary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
