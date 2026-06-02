use crate::primitives::{
    Divider, HeaderBar, IconSlot, ShellSurfaceSet, StatusPill, Surface, TabChrome,
};
use zaroxi_core_engine_layout::ShellLayout;
use zaroxi_core_engine_style::EngineTheme;
use zaroxi_kernel_math::Rect;

/// Build a complete `ShellSurfaceSet` from the deterministic shell layout and
/// the engine theme. Returns ordered primitives in paint order.
///
/// This function is the authoritative bridge between layout geometry and
/// themed visual primitives. It does NOT depend on any interface-layer types.
pub fn build_shell_surface_set(layout: &ShellLayout, theme: &EngineTheme) -> ShellSurfaceSet {
    let mut set = ShellSurfaceSet::new();
    let dt = zaroxi_core_engine_style::EngineDesignTokens::default();
    let r = dt.radius_md;

    // ── Background (full window) ──
    set.add_surface(
        Surface::new(Rect::new(0.0, 0.0, layout.window_size.width, layout.window_size.height))
            .with_fill(theme.app_background.to_array()),
    );

    // ── Titlebar/toolbar surface ──
    set.add_surface(
        Surface::new(layout.titlebar).with_fill(theme.status_bar_background.to_array()),
    );
    // Toolbar bottom divider
    set.add_divider(Divider::horizontal(
        layout.titlebar.x,
        layout.titlebar.y + layout.titlebar.height - 1.0,
        layout.titlebar.width,
        theme.divider_default.to_array(),
    ));

    // ── Titlebar brand accent strip ──
    if layout.titlebar.width > 60.0 && layout.titlebar.height > 8.0 {
        let brand_x = layout.titlebar.x + 10.0;
        let brand_y = layout.titlebar.y + 5.0;
        let brand_h = layout.titlebar.height - 10.0;
        set.add_surface(
            Surface::new(Rect::new(brand_x, brand_y, 32.0, brand_h))
                .with_fill(theme.accent.adjust_brightness(0.82).to_array()),
        );
    }

    // ── Activity rail ──
    set.add_surface(Surface::new(layout.sidebar).with_fill(theme.sidebar_background.to_array()));

    // Rail-icon icons (top group)
    if layout.sidebar.height > 48.0 {
        let rail_x = layout.sidebar.x;
        let icon_w = layout.sidebar.width - 14.0;
        let icon_h: f32 = 28.0;
        let gap: f32 = 4.0;
        let mut y = layout.sidebar.y + 10.0;

        for (idx, active) in [true, false, false, false].iter().enumerate() {
            let icon_rect = Rect::new(rail_x + 7.0, y, icon_w, icon_h);
            let mut slot = IconSlot::new(icon_rect).with_fill(if *active {
                theme.selected_bg.adjust_brightness(1.6).to_array()
            } else {
                theme.text_faint.adjust_brightness(0.18).to_array()
            });
            if *active {
                slot = slot.with_accent(theme.accent.to_array());
            }
            set.add_icon(slot);

            if *active {
                let indicator_rect = Rect::new(rail_x + 2.0, y + 2.0, 3.0, icon_h - 4.0);
                set.add_surface(Surface::new(indicator_rect).with_fill(theme.accent.to_array()));
            }

            y += icon_h + gap;

            // Separator after first icon group
            if idx == 0 && layout.sidebar.height > 200.0 {
                set.add_divider(Divider::horizontal(
                    rail_x + 12.0,
                    y,
                    layout.sidebar.width - 24.0,
                    theme.divider_subtle.to_array(),
                ));
                y += gap;
            }
        }

        // Bottom icons (settings, account)
        if layout.sidebar.height > 120.0 {
            let bottom_start =
                layout.sidebar.y + layout.sidebar.height - (2.0 * (icon_h + gap) + 12.0);
            let mut by = bottom_start;
            for _ in 0..2 {
                let icon_rect = Rect::new(rail_x + 7.0, by, icon_w, icon_h);
                set.add_icon(
                    IconSlot::new(icon_rect)
                        .with_fill(theme.text_faint.adjust_brightness(0.16).to_array()),
                );
                by += icon_h + gap;
            }
        }
    }

    // ── Sidebar (right of rail) ──
    let sidebar_w = if layout.sidebar.width > 0.0 { layout.sidebar.width - 44.0 } else { 0.0 };
    if sidebar_w > 0.0 {
        let sidebar_rect = Rect::new(44.0, layout.sidebar.y, sidebar_w, layout.sidebar.height);
        set.add_surface(Surface::new(sidebar_rect).with_fill(theme.sidebar_background.to_array()));

        // Search bar
        let pad = 10.0;
        let search_h = 26.0;
        let inner_w = sidebar_w - pad * 2.0;
        let mut y_off = layout.sidebar.y + pad;
        set.add_surface(
            Surface::new(Rect::new(sidebar_rect.x + pad, y_off, inner_w, search_h))
                .with_fill(theme.input_background.to_array())
                .with_radius(r),
        );
        y_off += search_h + 8.0;

        // Subtle divider below search
        set.add_divider(Divider::horizontal(
            sidebar_rect.x + pad,
            y_off,
            inner_w,
            theme.divider_subtle.adjust_brightness(0.8).to_array(),
        ));
        y_off += 12.0;

        // Section headers: PROJECT, GIT, OUTLINE
        let section_labels = ["PROJECT", "GIT", "OUTLINE"];
        let section_h = 20.0;
        let row_h = 16.0;
        for section_label in section_labels {
            if y_off + section_h > layout.sidebar.y + layout.sidebar.height - 60.0 {
                break;
            }
            set.add_header(
                HeaderBar::new(
                    Rect::new(sidebar_rect.x, y_off, sidebar_w, section_h),
                    section_label,
                )
                .with_fill(theme.panel_header_bg().to_array())
                .with_text_color(theme.text_secondary.to_array()),
            );
            y_off += section_h + 2.0;

            // Section item placeholders
            let item_count = if section_label == "PROJECT" { 4 } else { 3 };
            for _ in 0..item_count {
                if y_off + row_h > layout.sidebar.y + layout.sidebar.height - 36.0 {
                    break;
                }
                set.add_surface(
                    Surface::new(Rect::new(
                        sidebar_rect.x + pad + 14.0,
                        y_off + 2.0,
                        inner_w - 20.0,
                        12.0,
                    ))
                    .with_fill(theme.text_faint.adjust_brightness(0.20).to_array()),
                );
                y_off += row_h;
            }
            y_off += 6.0;
        }
    }

    // Sidebar right edge divider
    set.add_divider(Divider::vertical(
        layout.sidebar.x + layout.sidebar.width - 1.0,
        layout.sidebar.y,
        layout.sidebar.height,
        theme.divider_default.adjust_brightness(0.85).to_array(),
    ));

    // ── Editor tab strip ──
    set.add_surface(
        Surface::new(layout.editor_tab_bar).with_fill(theme.tab_strip_background.to_array()),
    );
    set.add_divider(Divider::horizontal(
        layout.editor_tab_bar.x,
        layout.editor_tab_bar.y + layout.editor_tab_bar.height - 1.0,
        layout.editor_tab_bar.width,
        theme.divider_default.to_array(),
    ));

    // Active tab
    if layout.editor_tab_bar.width > 80.0 && layout.editor_tab_bar.height > 4.0 {
        let tab_w = 120.0;
        let tab_rect = Rect::new(
            layout.editor_tab_bar.x,
            layout.editor_tab_bar.y - 1.0,
            tab_w,
            layout.editor_tab_bar.height + 1.0,
        );
        set.add_tab(
            TabChrome::new(tab_rect, "main.rs")
                .active(theme.accent.to_array())
                .with_fill(theme.tab_active_background.to_array())
                .with_text_color(theme.text_primary.to_array()),
        );
    }

    // ── Editor breadcrumb ──
    set.add_surface(
        Surface::new(layout.editor_breadcrumb_bar)
            .with_fill(theme.editor_background.adjust_brightness(0.97).to_array()),
    );
    set.add_divider(Divider::horizontal(
        layout.editor_breadcrumb_bar.x,
        layout.editor_breadcrumb_bar.y + layout.editor_breadcrumb_bar.height - 1.0,
        layout.editor_breadcrumb_bar.width,
        theme.divider_subtle.to_array(),
    ));

    // ── Editor content area ──
    set.add_surface(
        Surface::new(layout.editor_content).with_fill(theme.editor_background.to_array()),
    );

    // ── Editor bottom panel (terminal) ──
    if layout.editor_bottom_panel.height > 0.0 {
        set.add_divider(Divider::horizontal(
            layout.editor_bottom_panel.x,
            layout.editor_bottom_panel.y - 1.0,
            layout.editor_bottom_panel.width,
            theme.divider_default.to_array(),
        ));
        set.add_header(
            HeaderBar::new(
                Rect::new(
                    layout.editor_bottom_panel.x,
                    layout.editor_bottom_panel.y,
                    layout.editor_bottom_panel.width,
                    26.0,
                ),
                "Terminal",
            )
            .with_fill(theme.panel_header_bg().to_array())
            .with_text_color(theme.text_secondary.to_array()),
        );
        set.add_surface(
            Surface::new(Rect::new(
                layout.editor_bottom_panel.x,
                layout.editor_bottom_panel.y + 26.0,
                layout.editor_bottom_panel.width,
                (layout.editor_bottom_panel.height - 26.0).max(0.0),
            ))
            .with_fill(theme.bottom_panel_background.to_array()),
        );
    }

    // ── AI panel ──
    if layout.ai_panel.width > 0.0 {
        set.add_divider(Divider::vertical(
            layout.ai_panel.x - 1.0,
            layout.ai_panel.y,
            layout.ai_panel.height,
            theme.divider_default.to_array(),
        ));

        set.add_header(
            HeaderBar::new(
                Rect::new(layout.ai_panel.x, layout.ai_panel.y, layout.ai_panel.width, 28.0),
                "AI Assistant",
            )
            .with_fill(theme.panel_header_bg().to_array())
            .with_text_color(theme.text_secondary.to_array()),
        );

        set.add_surface(
            Surface::new(Rect::new(
                layout.ai_panel.x,
                layout.ai_panel.y + 28.0,
                layout.ai_panel.width,
                (layout.ai_panel.height - 28.0).max(0.0),
            ))
            .with_fill(theme.assistant_panel_background.to_array()),
        );
    }

    // ── Status bar ──
    set.add_surface(
        Surface::new(layout.status_bar).with_fill(theme.status_bar_background.to_array()),
    );
    // Status bar top separator
    set.add_divider(Divider::horizontal(
        layout.status_bar.x,
        layout.status_bar.y,
        layout.status_bar.width,
        theme.divider_default.adjust_brightness(0.9).to_array(),
    ));

    // Status bar pills
    if layout.status_bar.width > 120.0 && layout.status_bar.height > 8.0 {
        let pill_h = layout.status_bar.height - 6.0;
        let pill_y = layout.status_bar.y + 3.0;

        // Left group: info cells
        let left_cells = [("Ready", 36.0), ("Ln 22, Col 14", 54.0), ("UTF-8", 36.0), ("LF", 28.0)];
        let mut cx = layout.status_bar.x + 20.0;
        for (label, w) in left_cells {
            if cx + w > layout.status_bar.x + layout.status_bar.width {
                break;
            }
            set.add_pill(
                StatusPill::new(Rect::new(cx, pill_y, w, pill_h), label)
                    .with_fill(theme.text_faint.adjust_brightness(0.14).to_array())
                    .with_text_color(theme.text_secondary.to_array()),
            );
            cx += w + 10.0;
        }

        // Right group: language badge
        if layout.status_bar.width > 200.0 {
            let badge_w = 48.0;
            let badge_x = layout.status_bar.x + layout.status_bar.width - badge_w - 16.0;
            set.add_pill(
                StatusPill::new(Rect::new(badge_x, pill_y, badge_w, pill_h), "Rust")
                    .with_fill(theme.accent_soft_bg.adjust_brightness(2.2).to_array())
                    .with_text_color(theme.accent.to_array()),
            );
        }
    }

    set
}
