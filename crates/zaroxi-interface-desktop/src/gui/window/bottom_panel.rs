/*!
Bottom dock drawing logic (full-width panel above status bar).

Phase 5: engine integration — tab content flows through engine-owned
`Bar` → `compose_bars_scene()` → `WidgetScene`. Desktop owns only visual
styling (separators, body log lines, active accent border).
*/
use zaroxi_core_engine_render_backend::DrawRect;
use zaroxi_core_engine_ui::{Bar, compose_bars_scene};
use zaroxi_interface_theme::theme::SemanticColors;
use zaroxi_kernel_math::Rect;

pub fn draw(
    region: &crate::gui::ShellRegion,
    theme: &crate::gui::Theme,
    sem: &SemanticColors,
) -> Vec<DrawRect> {
    let mut rects: Vec<DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let r = &region.rect;

    // Dock background
    rects.push(DrawRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
        color: super::theme_adapter::adjust_color(sem.panel_background, 1.0),
    });

    // Top separator
    if r.height > bt {
        rects.push(DrawRect {
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
        rects.push(DrawRect {
            x: r.x,
            y: header_y,
            width: r.width,
            height: header_h,
            color: super::theme_adapter::adjust_color(sem.tab_strip_background, 1.0),
        });

        let tabs: u32 = 4;
        let tab_labels = ["Terminal", "Problems", "Output", "Debug"];
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

        // ── engine widget path: each tab is a Bar ──
        let mut bars: Vec<Bar> = Vec::new();
        for i in 0..tabs as usize {
            let tab_rect = Rect::new(tx as f32, tab_y as f32, tab_w as f32, tab_h as f32);
            bars.push(Bar::new(tab_labels[i], tab_rect));
            tx = tx.saturating_add(tab_w).saturating_add(tab_pad);
        }

        let active_bg =
            wgpu_color_to_f32(super::theme_adapter::adjust_color(sem.tab_active_background, 1.0));
        let inactive_bg =
            wgpu_color_to_f32(super::theme_adapter::adjust_color(sem.tab_background, 1.0));
        let text_color =
            wgpu_color_to_f32(super::theme_adapter::parse_hex_color(theme.text_primary));

        // First tab is active
        let mut rect_colors = vec![inactive_bg; bars.len()];
        rect_colors[0] = active_bg;
        let label_colors = vec![text_color; bars.len()];

        let scene = compose_bars_scene(&bars, &rect_colors, &label_colors);

        // Tab background rects
        for rp in &scene.rects {
            rects.push(DrawRect {
                x: rp.x as u32,
                y: rp.y as u32,
                width: rp.width as u32,
                height: rp.height as u32,
                color: f32_to_wgpu(rp.color),
            });
        }

        // Active tab accent border (desktop styling)
        {
            let active_bar = &bars[0];
            let ax = active_bar.rect.x as u32;
            let ay = (active_bar.rect.y + active_bar.rect.height) as u32 - 2;
            rects.push(DrawRect {
                x: ax,
                y: ay,
                width: tab_w,
                height: 2,
                color: super::theme_adapter::adjust_color(sem.accent, 0.88),
            });
        }

        // Text labels from engine scene
        if !scene.labels.is_empty() {
            let label_strings: Vec<String> = scene.labels.iter().map(|l| l.label.clone()).collect();
            let mut text_rects = super::text_adapter::layout_and_publish_text(
                r.x.saturating_add(12),
                r.y.saturating_add(6),
                r.width.saturating_sub(24),
                30,
                &label_strings,
                theme,
                theme.text_primary,
            );
            rects.append(&mut text_rects);
        }

        // Divider below header
        if r.height > header_y.saturating_sub(r.y).saturating_add(header_h).saturating_add(bt) {
            rects.push(DrawRect {
                x: r.x,
                y: header_y.saturating_add(header_h),
                width: r.width,
                height: bt,
                color: super::theme_adapter::adjust_color(sem.divider, 0.8),
            });
        }
    }

    // Body (desktop styling — simulated output log lines)
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
            rects.push(DrawRect {
                x: r.x.saturating_add(14),
                y: ly,
                width: w.saturating_sub(14),
                height: line_h,
                color,
            });
            ly = ly.saturating_add(line_h).saturating_add(gap);
        }
    }

    rects
}

fn wgpu_color_to_f32(c: wgpu::Color) -> [f32; 4] {
    [c.r as f32, c.g as f32, c.b as f32, c.a as f32]
}

fn f32_to_wgpu(c: [f32; 4]) -> wgpu::Color {
    wgpu::Color { r: c[0] as f64, g: c[1] as f64, b: c[2] as f64, a: c[3] as f64 }
}
