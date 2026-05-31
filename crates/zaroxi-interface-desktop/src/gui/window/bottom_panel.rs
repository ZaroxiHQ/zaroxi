/*!
Bottom dock drawing logic (full-width panel above status bar).

Phase 4: product-parity bottom dock — tabs with accent bottom border,
tab header row, output log lines with colored types.
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

    // Dock background
    rects.push(zaroxi_core_engine_render_backend::DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_color(sem.panel_background, 1.0),
    });

    // Top separator
    if r.height > bt {
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider, 0.85),
        });
    }

    // Tab header row
    let header_h: u32 = std::cmp::min(28, r.height / 4);
    if header_h > 0 && r.width > 40 {
        let header_y = r.y.saturating_add(bt);
        rects.push(zaroxi_core_engine_render_backend::DrawRect {
            x: r.x,
            y: header_y,
            width: r.width,
            height: header_h,
            color: super::theme_adapter::adjust_color(sem.tab_strip_background, 1.0),
        });

        let tabs: u32 = 4;
        let tab_pad: u32 = 10;
        let total_pad = tab_pad * (tabs + 1);
        let tab_w = if r.width > total_pad {
            (r.width.saturating_sub(total_pad)) / tabs
        } else {
            r.width / std::cmp::max(1, tabs)
        };
        let mut tx = r.x.saturating_add(tab_pad);
        let tab_y = header_y.saturating_add(4);
        let tab_h = header_h.saturating_sub(8);

        for i in 0..tabs {
            let active = i == 0;
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: tx,
                y: tab_y,
                width: tab_w,
                height: tab_h,
                color: if active {
                    super::theme_adapter::adjust_color(sem.tab_active_background, 1.0)
                } else {
                    super::theme_adapter::adjust_color(sem.tab_background, 1.0)
                },
            });
            if active {
                rects.push(zaroxi_core_engine_render_backend::DrawRect {
                    x: tx,
                    y: tab_y.saturating_add(tab_h).saturating_sub(2),
                    width: tab_w,
                    height: 2,
                    color: super::theme_adapter::adjust_color(sem.accent, 0.88),
                });
            }
            tx = tx.saturating_add(tab_w).saturating_add(tab_pad);
        }

        if r.height > header_y.saturating_sub(r.y).saturating_add(header_h).saturating_add(bt) {
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x,
                y: header_y.saturating_add(header_h),
                width: r.width,
                height: bt,
                color: super::theme_adapter::adjust_color(sem.divider, 0.8),
            });
        }
    }

    // Body
    let body_start = r.y.saturating_add(bt).saturating_add(header_h).saturating_add(bt);
    if r.height > body_start.saturating_sub(r.y).saturating_add(8) && r.width > 40 {
        let available_h = r.height.saturating_sub(body_start.saturating_sub(r.y)).saturating_sub(6);
        let line_h = 11u32;
        let gap = 3u32;
        let lines = if available_h > (line_h + gap) { available_h / (line_h + gap) } else { 0 };
        let mut ly = body_start.saturating_add(4);

        for i in 0..lines {
            let factor = match i % 4 {
                0 => 0.84,
                1 => 0.52,
                2 => 0.70,
                _ => 0.38,
            };
            let w = ((r.width as f64) * factor) as u32;
            let color = match i % 4 {
                0 => super::theme_adapter::adjust_color(sem.text_secondary, 0.42),
                1 => super::theme_adapter::adjust_color(sem.syntax_function, 0.78),
                3 => super::theme_adapter::adjust_color(sem.syntax_string, 0.78),
                _ => super::theme_adapter::adjust_color(sem.text_secondary, 0.36),
            };
            rects.push(zaroxi_core_engine_render_backend::DrawRect {
                x: r.x.saturating_add(14),
                y: ly,
                width: w.saturating_sub(14),
                height: line_h,
                color,
            });
            ly = ly.saturating_add(line_h).saturating_add(gap);
        }
    }

    // Text labels
    if r.width > 80 {
        let labels = vec![
            "Terminal".to_string(),
            "Problems".to_string(),
            "Output".to_string(),
            "Debug".to_string(),
        ];
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(12),
            r.y.saturating_add(6),
            r.width.saturating_sub(24),
            30,
            &labels,
            theme,
            theme.text_primary,
        );
        rects.append(&mut text_rects);
    }

    rects
}
