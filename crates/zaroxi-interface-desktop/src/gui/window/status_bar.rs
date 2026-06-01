/*!
Status bar drawing logic.

Phase 5: engine integration — structural cell content flows through
engine-owned `Bar` → `compose_bars_scene()` → `WidgetScene`.
Desktop owns only visual styling (separator, divider dots).
*/
use zaroxi_core_engine_render_backend::DrawRect;
use zaroxi_core_engine_ui::{Bar, compose_bars_scene};
use zaroxi_interface_theme::theme::ZaroxiTheme;
use zaroxi_kernel_math::Rect;

pub fn draw(region: &crate::gui::ShellRegion, theme: &crate::gui::Theme) -> Vec<DrawRect> {
    let mut rects: Vec<DrawRect> = Vec::new();
    let bt: u32 = theme.border_thickness as u32;
    let r = &region.rect;
    let sem = ZaroxiTheme::Dark.colors(false);

    // ── status bar background ──
    let bg = super::theme_adapter::adjust_color(sem.status_bar_background, 1.0);
    rects.push(DrawRect { x: r.x, y: r.y, width: r.width, height: r.height, color: bg });

    // ── top separator (desktop styling) ──
    if r.height > bt {
        rects.push(DrawRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: bt,
            color: super::theme_adapter::adjust_color(sem.divider, 0.90),
        });
    }

    // ── build engine-owned Bar widgets for status cells ──
    let cell_h = r.height.saturating_sub(bt).saturating_sub(4);
    let cy = r.y.saturating_add(bt).saturating_add(2);
    let mut bars: Vec<Bar> = Vec::new();

    // Info cells (left side): width + label
    if r.width > 120 && r.height > 8 {
        let cells = [(36u32, "Ready"), (52, "Ln 22, Col 14"), (36, "UTF-8"), (28, "LF")];
        let mut cx: u32 = r.x.saturating_add(20);

        for (w, label) in cells.iter() {
            if cx + w > r.x + r.width {
                break;
            }
            let cell_rect = Rect::new(cx as f32, cy as f32, *w as f32, cell_h as f32);
            bars.push(Bar::new(*label, cell_rect));
            cx = cx.saturating_add(*w);
            // gap for divider dot
            cx = cx.saturating_add(10);
        }
    }

    // Badges (right side)
    if r.width > 200 && r.height > 8 {
        let badge_h = r.height.saturating_sub(bt).saturating_sub(4);
        let badge_w: u32 = 42;
        let mut rx: u32 = r.x.saturating_add(r.width).saturating_sub(16);

        // Formatter badge (rust-analyzer, width ~68 wide enough for typical formatter names)
        let fmt_w: u32 = 68;
        rx = rx.saturating_sub(fmt_w);
        if rx > r.x {
            let fmt_rect = Rect::new(rx as f32, cy as f32, fmt_w as f32, badge_h as f32);
            bars.push(Bar::new("rust-analyzer", fmt_rect));
        }

        // Gap for dot
        rx = rx.saturating_sub(8);

        // Language badge (Rust)
        rx = rx.saturating_sub(badge_w + 4);
        if rx > r.x {
            let lang_rect = Rect::new(rx as f32, cy as f32, badge_w as f32, badge_h as f32);
            bars.push(Bar::new("Rust", lang_rect));
        }
    }

    // ── engine composition: bars → WidgetScene ──
    let cell_bg_color =
        wgpu_color_to_f32_array(super::theme_adapter::adjust_color(sem.text_faint, 0.14));
    let badge_bg_color = wgpu_color_to_f32_array(super::theme_adapter::adjust_color(
        sem.accent_soft_background,
        2.2,
    ));
    let text_color =
        wgpu_color_to_f32_array(super::theme_adapter::parse_hex_color(theme.text_secondary));

    // Slice colors: info cells (first 4) get cell_bg, badges get badge_bg
    let info_count = if r.width > 120 && r.height > 8 { 4 } else { 0 };
    let bar_count = bars.len();
    let mut rect_colors = vec![cell_bg_color; bar_count];
    let label_colors = vec![text_color; bar_count];

    // Apply badge colors to the last bars (the badges)
    let badge_count = bar_count.saturating_sub(info_count);
    for i in (bar_count - badge_count)..bar_count {
        rect_colors[i] = badge_bg_color;
    }

    // Compose & convert rects to DrawRect
    let scene = compose_bars_scene(&bars, &rect_colors, &label_colors);
    for rp in &scene.rects {
        rects.push(DrawRect {
            x: rp.x as u32,
            y: rp.y as u32,
            width: rp.width as u32,
            height: rp.height as u32,
            color: f32_array_to_wgpu(rp.color),
        });
    }

    // ── divider dots (desktop styling) ──
    if r.width > 120 && r.height > 8 {
        let cells = [(36u32, ""), (52, ""), (36, ""), (28, "")];
        let mut cx: u32 = r.x.saturating_add(20);
        for (idx, &(w, _)) in cells.iter().enumerate() {
            if cx + w > r.x + r.width {
                break;
            }
            cx = cx.saturating_add(w);
            if idx < cells.len() - 1 && cx + 6 < r.x + r.width {
                rects.push(DrawRect {
                    x: cx.saturating_add(3),
                    y: cy.saturating_add(cell_h / 2).saturating_sub(1),
                    width: 2,
                    height: 2,
                    color: super::theme_adapter::adjust_color(sem.divider, 0.6),
                });
                cx = cx.saturating_add(10);
            }
        }
    }

    // ── text labels: route through engine labels ──
    if !scene.labels.is_empty() {
        let label_strings: Vec<String> = scene.labels.iter().map(|l| l.label.clone()).collect();
        let mut text_rects = super::text_adapter::layout_and_publish_text(
            r.x.saturating_add(20),
            r.y.saturating_add(bt).saturating_add(2),
            r.width.saturating_sub(40),
            r.height.saturating_sub(bt).saturating_sub(4),
            &label_strings,
            theme,
            theme.text_secondary,
        );
        rects.append(&mut text_rects);
    }

    rects
}

fn wgpu_color_to_f32_array(c: wgpu::Color) -> [f32; 4] {
    [c.r as f32, c.g as f32, c.b as f32, c.a as f32]
}

fn f32_array_to_wgpu(c: [f32; 4]) -> wgpu::Color {
    wgpu::Color { r: c[0] as f64, g: c[1] as f64, b: c[2] as f64, a: c[3] as f64 }
}
