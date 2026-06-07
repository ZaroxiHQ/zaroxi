use crate::ShellWorkContent;
use crate::primitives::DividerOrientation;
use crate::widgets::{PanelHeaderAction, ShellWidget, ShellWidgetTree};
use zaroxi_core_engine_layout::ShellLayout;
use zaroxi_core_engine_style::{InteractionState, StyleTokens, WidgetId};
use zaroxi_kernel_math::Rect;

/// Build a complete `ShellWidgetTree` from the deterministic shell layout and
/// host-supplied style tokens. Returns ordered widgets in paint order (bg first).
///
/// When `content` is `Some`, tab labels, explorer items, and panel content
/// are driven by the live workspace snapshot. When `None`, hardcoded
/// placeholders are used.
///
/// App-neutral mapping: IDE concepts (explorer, tabs, terminal, AI assistant,
/// status bar) are recomposed from generic widgets (ListItem, ListSectionHeader,
/// TabItem, PanelHeader, Surface, ScrollBar, Button, Divider, EmptyState,
/// StatusSegment) without changing the visible layout contract.
pub fn build_shell_widget_tree(
    layout: &ShellLayout,
    tokens: &StyleTokens,
    content: Option<&ShellWorkContent>,
) -> ShellWidgetTree {
    let mut tree = ShellWidgetTree::new();
    let _dt = zaroxi_core_engine_style::EngineDesignTokens::default();

    // ── 1. App background ──
    tree.push(ShellWidget::AppBackground {
        rect: Rect::new(0.0, 0.0, layout.window_size.width, layout.window_size.height),
        fill_color: tokens.app_background.to_array(),
    });

    // ── 2. Titlebar ──
    tree.push(ShellWidget::Titlebar {
        rect: layout.top_bar,
        fill_color: tokens.titlebar_background.to_array(),
        brand_label: "Zaroxi".into(),
    });

    // Toolbar window-control buttons (right side)
    if layout.top_bar.width > 160.0 && layout.top_bar.height > 8.0 {
        let btn_w = 32.0;
        let btn_h = layout.top_bar.height - 8.0;
        let btn_y = layout.top_bar.y + 4.0;
        let btn_x = layout.top_bar.x + layout.top_bar.width - (btn_w * 3.0 + 18.0);
        for (idx, (label, accent)) in [("_", false), ("[ ]", false), ("x", true)].iter().enumerate()
        {
            let bx = btn_x + idx as f32 * (btn_w + 2.0);
            tree.push(ShellWidget::Button {
                id: WidgetId::button(idx),
                rect: Rect::new(bx, btn_y, btn_w, btn_h),
                label: label.to_string(),
                fill_color: if *accent {
                    tokens.toolbar_close_button.to_array()
                } else {
                    tokens.toolbar_button_default.to_array()
                },
                state: InteractionState::Normal,
            });
        }
    }

    tree.push(ShellWidget::Divider {
        rect: Rect::new(
            layout.top_bar.x,
            layout.top_bar.y + layout.top_bar.height - 1.0,
            layout.top_bar.width,
            1.0,
        ),
        color: tokens.divider_default.to_array(),
        orientation: DividerOrientation::Horizontal,
        subtle: false,
    });

    // Titlebar brand accent
    if layout.top_bar.width > 60.0 && layout.top_bar.height > 8.0 {
        tree.push(ShellWidget::Surface {
            rect: Rect::new(
                layout.top_bar.x + 10.0,
                layout.top_bar.y + 5.0,
                32.0,
                layout.top_bar.height - 10.0,
            ),
            fill_color: tokens.toolbar_brand_accent.to_array(),
            border_color: None,
            border_width: 0.0,
        });
    }

    // ── 3. Activity rail + sidebar (left column) ──
    let rail_w = 44.0;
    let rail_rect = Rect::new(0.0, layout.left_panel.y, rail_w, layout.left_panel.height);
    tree.push(ShellWidget::Surface {
        rect: rail_rect,
        fill_color: tokens.rail_background.to_array(),
        border_color: None,
        border_width: 0.0,
    });

    // Rail items (top group) — composed as ListItem widgets
    if layout.left_panel.height > 48.0 {
        let icon_w = rail_w - 14.0;
        let icon_h: f32 = 28.0;
        let gap: f32 = 4.0;
        let mut y = layout.left_panel.y + 10.0;
        let rail_items: [(usize, &str, bool); 4] = [
            (0, "Explorer", true),
            (1, "Search", false),
            (2, "Source Ctrl", false),
            (3, "Debug", false),
        ];

        for (idx, label, active) in rail_items {
            let icon_rect = Rect::new(rail_rect.x + 7.0, y, icon_w, icon_h);
            let fill = if active {
                tokens.rail_item_active.to_array()
            } else {
                tokens.rail_item_inactive.to_array()
            };
            let accent =
                if active { Some(tokens.rail_item_active_accent.to_array()) } else { None };
            let state = if active { InteractionState::Selected } else { InteractionState::Normal };

            tree.push(ShellWidget::ListItem {
                id: WidgetId::list_item(idx),
                rect: icon_rect,
                label: label.into(),
                fill_color: fill,
                accent_indicator: accent,
                state,
            });
            y += icon_h + gap;

            // Separator after active group (subtle)
            if idx == 0 && layout.left_panel.height > 200.0 {
                tree.push(ShellWidget::Divider {
                    rect: Rect::new(rail_rect.x + 12.0, y, rail_w - 24.0, 1.0),
                    color: tokens.divider_subtle.to_array(),
                    orientation: DividerOrientation::Horizontal,
                    subtle: true,
                });
                y += gap;
            }
        }

        // Bottom rail items (settings, account) — composed as ListItem widgets
        if layout.left_panel.height > 120.0 {
            let bottom_start =
                layout.left_panel.y + layout.left_panel.height - (2.0 * (icon_h + gap) + 12.0);
            let mut by = bottom_start;
            for (idx, label) in [(4, "Settings"), (5, "Account")].iter() {
                tree.push(ShellWidget::ListItem {
                    id: WidgetId::list_item(*idx),
                    rect: Rect::new(rail_rect.x + 7.0, by, icon_w, icon_h),
                    label: label.to_string(),
                    fill_color: tokens.rail_item_bottom.to_array(),
                    accent_indicator: None,
                    state: InteractionState::Normal,
                });
                by += icon_h + gap;
            }
        }
    }

    // ── 4. Sidebar (right of rail) ──
    let sidebar_w =
        if layout.left_panel.width > 0.0 { layout.left_panel.width - rail_w } else { 0.0 };
    if sidebar_w > 0.0 {
        let sx = 44.0;
        let sidebar_rect = Rect::new(sx, layout.left_panel.y, sidebar_w, layout.left_panel.height);
        tree.push(ShellWidget::Surface {
            rect: sidebar_rect,
            fill_color: tokens.sidebar_background.to_array(),
            border_color: None,
            border_width: 0.0,
        });

        let pad = 10.0;
        let search_h = 26.0;
        let inner_w = sidebar_w - pad * 2.0;
        let mut y_off = layout.left_panel.y + pad;

        // Search bar area
        tree.push(ShellWidget::Surface {
            rect: Rect::new(sidebar_rect.x + pad, y_off, inner_w, search_h),
            fill_color: tokens.sidebar_input.to_array(),
            border_color: None,
            border_width: 0.0,
        });
        y_off += search_h + 8.0;

        // Subtle divider below search
        tree.push(ShellWidget::Divider {
            rect: Rect::new(sidebar_rect.x + pad, y_off, inner_w, 2.0),
            color: tokens.sidebar_search_divider.to_array(),
            orientation: DividerOrientation::Horizontal,
            subtle: true,
        });
        y_off += 12.0;

        // Explorer panel — built from structured panel items when available.
        build_explorer_panel_section(
            &mut tree,
            content,
            sidebar_rect,
            layout,
            tokens,
            pad,
            inner_w,
            &mut y_off,
        );

        // Sidebar scrollbar (if content overflows)
        if sidebar_rect.height > 200.0 && sidebar_rect.width > 20.0 {
            let sb_w = 4.0;
            let sb_x = sx + sidebar_w - sb_w - 3.0;
            let track_rect =
                Rect::new(sb_x, sidebar_rect.y + 8.0, sb_w, sidebar_rect.height - 16.0);
            let thumb_h = (track_rect.height * 0.5).max(16.0);
            tree.push(ShellWidget::ScrollBar {
                id: WidgetId::scrollbar(2),
                track_rect,
                thumb_rect: Rect::new(track_rect.x, track_rect.y, sb_w, thumb_h),
                track_fill: tokens.sidebar_scrollbar_track.to_array(),
                thumb_fill: tokens.sidebar_scrollbar_thumb.to_array(),
                state: InteractionState::Normal,
            });
        }
    }

    // Sidebar right-edge divider (subtle)
    tree.push(ShellWidget::Divider {
        rect: Rect::new(
            layout.left_panel.x + layout.left_panel.width - 1.0,
            layout.left_panel.y,
            1.0,
            layout.left_panel.height,
        ),
        color: tokens.sidebar_border.to_array(),
        orientation: DividerOrientation::Vertical,
        subtle: true,
    });

    // ── 5. Editor tab strip ──
    tree.push(ShellWidget::Surface {
        rect: layout.content_tab_strip,
        fill_color: tokens.tab_strip_background.to_array(),
        border_color: None,
        border_width: 0.0,
    });
    tree.push(ShellWidget::Divider {
        rect: Rect::new(
            layout.content_tab_strip.x,
            layout.content_tab_strip.y + layout.content_tab_strip.height - 1.0,
            layout.content_tab_strip.width,
            1.0,
        ),
        color: tokens.divider_default.to_array(),
        orientation: DividerOrientation::Horizontal,
        subtle: false,
    });

    // TabItem widgets: driven by ShellWorkContent.editor_tabs, with fallback
    if layout.content_tab_strip.width > 80.0 && layout.content_tab_strip.height > 4.0 {
        let tab_h = layout.content_tab_strip.height + 1.0;
        let tab_y = layout.content_tab_strip.y - 1.0;
        let tabs: Vec<(&str, bool, usize)> = content
            .and_then(|c| c.editor_tabs.as_ref())
            .map(|tabs| {
                tabs.iter().enumerate().map(|(i, label)| (label.as_str(), i == 0, i)).collect()
            })
            .unwrap_or_else(|| {
                vec![("main.rs", true, 0), ("lib.rs", false, 1), ("mod.rs", false, 2)]
            });
        let tab_w = 110.0;
        let mut tx = layout.content_tab_strip.x;

        for (label, active, idx) in tabs {
            let tw = if active { tab_w + 10.0 } else { tab_w };
            if tx + tw > layout.content_tab_strip.x + layout.content_tab_strip.width {
                break;
            }
            let tab_rect = Rect::new(tx, tab_y, tw, tab_h);
            let fill = if active {
                tokens.tab_active_background.to_array()
            } else {
                tokens.tab_inactive_background.to_array()
            };
            let text_c =
                if active { tokens.text_primary.to_array() } else { tokens.text_muted.to_array() };
            let accent = if active { Some(tokens.accent.to_array()) } else { None };
            let state = if active { InteractionState::Selected } else { InteractionState::Normal };

            tree.push(ShellWidget::TabItem {
                id: WidgetId::tab(idx),
                rect: tab_rect,
                label: label.into(),
                fill_color: fill,
                text_color: text_c,
                accent_strip: accent,
                state,
            });
            tx += tw;
        }
    }

    // ── 6. Editor breadcrumb ──
    tree.push(ShellWidget::Surface {
        rect: layout.content_breadcrumb,
        fill_color: tokens.editor_breadcrumb_background.to_array(),
        border_color: None,
        border_width: 0.0,
    });
    tree.push(ShellWidget::Divider {
        rect: Rect::new(
            layout.content_breadcrumb.x,
            layout.content_breadcrumb.y + layout.content_breadcrumb.height - 1.0,
            layout.content_breadcrumb.width,
            1.0,
        ),
        color: tokens.divider_subtle.to_array(),
        orientation: DividerOrientation::Horizontal,
        subtle: true,
    });

    // ── 7. Editor content ──
    tree.push(ShellWidget::Surface {
        rect: layout.content_area,
        fill_color: tokens.editor_content_background.to_array(),
        border_color: None,
        border_width: 0.0,
    });

    // Empty state when no editor body
    let has_editor = content.and_then(|c| c.editor_body.as_ref()).is_some();
    if !has_editor {
        tree.push(ShellWidget::EmptyState {
            rect: Rect::new(
                layout.content_area.x + 40.0,
                layout.content_area.y + 60.0,
                200.0,
                40.0,
            ),
            message: "No file open".into(),
            fill_color: [0.0, 0.0, 0.0, 0.0],
            text_color: tokens.text_muted.to_array(),
        });
    }

    // Editor scrollbar (right edge)
    if layout.content_area.height > 40.0 && layout.content_area.width > 20.0 {
        let sb_w = 6.0;
        let sb_x = layout.content_area.x + layout.content_area.width - sb_w - 3.0;
        let track_rect =
            Rect::new(sb_x, layout.content_area.y + 4.0, sb_w, layout.content_area.height - 8.0);
        let thumb_h = (track_rect.height * 0.25).max(20.0);
        tree.push(ShellWidget::ScrollBar {
            id: WidgetId::scrollbar(1),
            track_rect,
            thumb_rect: Rect::new(track_rect.x, track_rect.y, sb_w, thumb_h),
            track_fill: tokens.editor_scrollbar_track.to_array(),
            thumb_fill: tokens.editor_scrollbar_thumb.to_array(),
            state: InteractionState::Normal,
        });
    }

    // ── 8. Editor bottom panel (Terminal) ──
    if layout.bottom_panel.height > 0.0 {
        tree.push(ShellWidget::Divider {
            rect: Rect::new(
                layout.bottom_panel.x,
                layout.bottom_panel.y - 1.0,
                layout.bottom_panel.width,
                1.0,
            ),
            color: tokens.divider_default.to_array(),
            orientation: DividerOrientation::Horizontal,
            subtle: false,
        });
        let header_h = 26.0;
        let header_rect = Rect::new(
            layout.bottom_panel.x,
            layout.bottom_panel.y,
            layout.bottom_panel.width,
            header_h,
        );
        // Close action button
        let action_w = 20.0;
        let action_x = header_rect.x + header_rect.width - action_w - 8.0;
        let action_y = header_rect.y + 3.0;
        let action_h = header_rect.height - 6.0;
        let actions = vec![PanelHeaderAction {
            id: WidgetId::panel_action("terminal", "close"),
            rect: Rect::new(action_x, action_y, action_w, action_h),
            label: "x".into(),
            fill_color: tokens.panel_action_fill.to_array(),
            hover_fill: tokens.panel_action_hover.to_array(),
            state: InteractionState::Normal,
        }];
        tree.push(ShellWidget::PanelHeader {
            id: WidgetId::panel_header("terminal"),
            rect: header_rect,
            label: "Terminal".into(),
            fill_color: tokens.panel_header_background.to_array(),
            text_color: tokens.panel_header_text.to_array(),
            actions,
        });
        tree.push(ShellWidget::Button {
            id: WidgetId::button(10),
            rect: Rect::new(action_x, action_y, action_w, action_h),
            label: "x".into(),
            fill_color: tokens.panel_action_fill.to_array(),
            state: InteractionState::Normal,
        });
        tree.push(ShellWidget::Surface {
            rect: Rect::new(
                layout.bottom_panel.x,
                layout.bottom_panel.y + header_h,
                layout.bottom_panel.width,
                (layout.bottom_panel.height - header_h).max(0.0),
            ),
            fill_color: tokens.bottom_panel_background.to_array(),
            border_color: None,
            border_width: 0.0,
        });

        // Bottom panel tabs (Terminal / Problems / Output)
        let tab_labels =
            content.and_then(|c| c.terminal_tabs.as_ref()).cloned().unwrap_or_else(|| {
                vec!["Terminal".to_string(), "Problems".to_string(), "Output".to_string()]
            });
        let tab_w = 70.0;
        let tab_h = 22.0;
        let tab_y = layout.bottom_panel.y + 2.0;
        let mut tab_x = layout.bottom_panel.x + 8.0;
        for (i, label) in tab_labels.iter().enumerate() {
            tree.push(ShellWidget::TabItem {
                id: WidgetId::tab(10 + i),
                rect: Rect::new(tab_x, tab_y, tab_w, tab_h),
                label: label.clone(),
                fill_color: if i == 0 {
                    tokens.tab_active_background.to_array()
                } else {
                    tokens.tab_strip_background.to_array()
                },
                text_color: tokens.text_secondary.to_array(),
                accent_strip: if i == 0 { Some(tokens.accent.to_array()) } else { None },
                state: InteractionState::Normal,
            });
            tab_x += tab_w + 4.0;
        }

        // Scrollbar on right edge of terminal panel
        let sb_w = 6.0;
        let sb_x = layout.bottom_panel.x + layout.bottom_panel.width - sb_w - 2.0;
        let track_rect = Rect::new(
            sb_x,
            layout.bottom_panel.y + header_h + 4.0,
            sb_w,
            layout.bottom_panel.height - header_h - 8.0,
        );
        let thumb_h = (track_rect.height * 0.3).max(16.0);
        tree.push(ShellWidget::ScrollBar {
            id: WidgetId::scrollbar(0),
            track_rect,
            thumb_rect: Rect::new(track_rect.x, track_rect.y, sb_w, thumb_h),
            track_fill: tokens.bottom_scrollbar_track.to_array(),
            thumb_fill: tokens.bottom_scrollbar_thumb.to_array(),
            state: InteractionState::Normal,
        });
    }

    // ── 9. AI panel ──
    if layout.right_panel.width > 0.0 {
        tree.push(ShellWidget::Divider {
            rect: Rect::new(
                layout.right_panel.x - 1.0,
                layout.right_panel.y,
                1.0,
                layout.right_panel.height,
            ),
            color: tokens.divider_default.to_array(),
            orientation: DividerOrientation::Vertical,
            subtle: false,
        });
        let header_h = 28.0;
        let header_rect = Rect::new(
            layout.right_panel.x,
            layout.right_panel.y,
            layout.right_panel.width,
            header_h,
        );
        let action_w = 20.0;
        let action_x = header_rect.x + header_rect.width - action_w - 10.0;
        let action_y = header_rect.y + 4.0;
        let action_h = header_rect.height - 8.0;
        let actions = vec![PanelHeaderAction {
            id: WidgetId::panel_action("ai_assistant", "close"),
            rect: Rect::new(action_x, action_y, action_w, action_h),
            label: "x".into(),
            fill_color: tokens.panel_action_fill.to_array(),
            hover_fill: tokens.panel_action_hover.to_array(),
            state: InteractionState::Normal,
        }];
        tree.push(ShellWidget::PanelHeader {
            id: WidgetId::panel_header("ai_assistant"),
            rect: header_rect,
            label: "AI Assistant".into(),
            fill_color: tokens.panel_header_background.to_array(),
            text_color: tokens.panel_header_text.to_array(),
            actions,
        });
        tree.push(ShellWidget::Button {
            id: WidgetId::button(11),
            rect: Rect::new(action_x, action_y, action_w, action_h),
            label: "x".into(),
            fill_color: tokens.panel_action_fill.to_array(),
            state: InteractionState::Normal,
        });
        tree.push(ShellWidget::Surface {
            rect: Rect::new(
                layout.right_panel.x,
                layout.right_panel.y + header_h,
                layout.right_panel.width,
                (layout.right_panel.height - header_h).max(0.0),
            ),
            fill_color: tokens.assistant_panel_background.to_array(),
            border_color: None,
            border_width: 0.0,
        });

        // Empty state when no AI content
        let has_ai = content.and_then(|c| c.ai_panel_content.as_ref()).is_some();
        if !has_ai {
            tree.push(ShellWidget::EmptyState {
                rect: Rect::new(
                    layout.right_panel.x + 16.0,
                    layout.right_panel.y + header_h + 32.0,
                    layout.right_panel.width - 32.0,
                    40.0,
                ),
                message: "No AI session".into(),
                fill_color: [0.0, 0.0, 0.0, 0.0],
                text_color: tokens.text_muted.to_array(),
            });
        }

        // AI action buttons — placed below header
        let btn_w = 64.0;
        let btn_h = 22.0;
        let btn_y = layout.right_panel.y + header_h + 8.0;
        let mut btn_x = layout.right_panel.x + 12.0;
        for (label, idx) in &[("Explain", 20), ("Review", 21), ("Apply", 22), ("Reject", 23)] {
            tree.push(ShellWidget::Button {
                id: WidgetId::button(*idx),
                rect: Rect::new(btn_x, btn_y, btn_w, btn_h),
                label: label.to_string(),
                fill_color: tokens.rail_background.to_array(),
                state: InteractionState::Normal,
            });
            btn_x += btn_w + 8.0;
        }

        // AI prompt text input
        let input_y = btn_y + btn_h + 8.0;
        let input_w = layout.right_panel.width - 24.0;
        tree.push(ShellWidget::TextInput {
            id: WidgetId::text_input(0),
            rect: Rect::new(layout.right_panel.x + 12.0, input_y, input_w, 28.0),
            text: String::new(),
            placeholder: "Describe what you want to do...".into(),
            fill_color: tokens.rail_background.to_array(),
            text_color: tokens.text_secondary.to_array(),
            focused: false,
        });
    }

    // ── 10. Status bar ──
    tree.push(ShellWidget::Surface {
        rect: layout.bottom_bar,
        fill_color: tokens.status_bar_background.to_array(),
        border_color: None,
        border_width: 0.0,
    });
    tree.push(ShellWidget::Divider {
        rect: Rect::new(layout.bottom_bar.x, layout.bottom_bar.y, layout.bottom_bar.width, 1.0),
        color: tokens.status_divider.to_array(),
        orientation: DividerOrientation::Horizontal,
        subtle: true,
    });

    // Status bar segments (pills)
    if layout.bottom_bar.width > 120.0 && layout.bottom_bar.height > 8.0 {
        let pill_h = layout.bottom_bar.height - 6.0;
        let pill_y = layout.bottom_bar.y + 3.0;

        let left_cells: [(&str, f32); 4] =
            [("Ready", 36.0), ("Ln 22, Col 14", 54.0), ("UTF-8", 36.0), ("LF", 28.0)];
        let mut cx = layout.bottom_bar.x + 20.0;
        for (idx, (label, w)) in left_cells.iter().enumerate() {
            if cx + *w > layout.bottom_bar.x + layout.bottom_bar.width {
                break;
            }
            tree.push(ShellWidget::StatusSegment {
                id: WidgetId::status_segment(idx),
                rect: Rect::new(cx, pill_y, *w, pill_h),
                label: label.to_string(),
                fill_color: tokens.status_pill_fill.to_array(),
                text_color: tokens.status_pill_text.to_array(),
            });
            cx += *w + 10.0;
        }

        // Language badge (right)
        if layout.bottom_bar.width > 200.0 {
            let badge_w = 48.0;
            let badge_x = layout.bottom_bar.x + layout.bottom_bar.width - badge_w - 16.0;
            tree.push(ShellWidget::StatusSegment {
                id: WidgetId::status_segment(4),
                rect: Rect::new(badge_x, pill_y, badge_w, pill_h),
                label: "Rust".into(),
                fill_color: tokens.status_language_badge_fill.to_array(),
                text_color: tokens.status_language_badge_text.to_array(),
            });
        }
    }

    // ── 11. Bottom dock (currently unused placeholder) ──

    tree
}

/// Build a `ShellSurfaceSet` (backward-compat wrapper that builds a widget
/// tree and converts to the flat primitives collection).
pub fn build_shell_surface_set(
    layout: &ShellLayout,
    tokens: &StyleTokens,
) -> crate::primitives::ShellSurfaceSet {
    build_shell_widget_tree(layout, tokens, None).to_surface_set()
}

// ── Explorer panel section builder ──────────────────────────────────

fn build_explorer_panel_section(
    tree: &mut ShellWidgetTree,
    content: Option<&ShellWorkContent>,
    sidebar_rect: Rect,
    layout: &ShellLayout,
    tokens: &StyleTokens,
    pad: f32,
    inner_w: f32,
    y_off: &mut f32,
) {
    let row_h = 18.0;
    let sidebar_w = sidebar_rect.width;
    let max_y = layout.left_panel.y + layout.left_panel.height - 12.0;

    // ------------------------------------------------------------------
    // Panel header
    // ------------------------------------------------------------------
    let panel_title = content.and_then(|c| c.explorer_panel_title.as_deref());
    if let Some(title) = panel_title {
        let hdr_h = 22.0;
        if *y_off + hdr_h <= max_y {
            tree.push(ShellWidget::ListSectionHeader {
                rect: Rect::new(sidebar_rect.x, *y_off, sidebar_w, hdr_h),
                label: title.to_string(),
                fill_color: tokens.panel_header_background.to_array(),
                text_color: tokens.panel_header_text.to_array(),
            });
            *y_off += hdr_h + 4.0;
        }
    }

    // ------------------------------------------------------------------
    // Structured panel items (new path)
    // ------------------------------------------------------------------
    let panel_items = content.and_then(|c| c.explorer_panel_items.as_deref());
    if let Some(items) = panel_items {
        if !items.is_empty() {
            for (item_idx, item) in items.iter().enumerate() {
                if *y_off + row_h > max_y {
                    break;
                }
                let indent_px = item.depth as f32 * 14.0;
                tree.push(ShellWidget::ListItem {
                    id: WidgetId::list_item(10 + item_idx),
                    rect: Rect::new(
                        sidebar_rect.x + pad + 14.0 + indent_px,
                        *y_off + 2.0,
                        inner_w - 20.0 - indent_px,
                        14.0,
                    ),
                    label: item.label.clone(),
                    fill_color: if item.is_active {
                        tokens.rail_item_active.to_array()
                    } else {
                        tokens.sidebar_file_item.to_array()
                    },
                    accent_indicator: if item.is_active {
                        Some(tokens.accent.to_array())
                    } else {
                        None
                    },
                    state: if item.is_active {
                        InteractionState::Selected
                    } else {
                        InteractionState::Normal
                    },
                });
                *y_off += row_h;
            }
            return;
        }

        // Empty panel — show empty state or button
        let btn_label = content.and_then(|c| c.explorer_empty_button.as_deref());
        let empty_msg = content.and_then(|c| c.explorer_empty_message.as_deref());
        if let Some(label) = btn_label {
            let btn_w = 140.0;
            let btn_h = 30.0;
            tree.push(ShellWidget::Button {
                id: WidgetId::button(30),
                rect: Rect::new(sidebar_rect.x + pad + 10.0, *y_off + 8.0, btn_w, btn_h),
                label: label.to_string(),
                fill_color: tokens.accent.to_array(),
                state: InteractionState::Normal,
            });
        } else if let Some(msg) = empty_msg {
            tree.push(ShellWidget::EmptyState {
                rect: Rect::new(sidebar_rect.x + pad, *y_off, inner_w, 24.0),
                message: msg.to_string(),
                fill_color: [0.0, 0.0, 0.0, 0.0],
                text_color: tokens.text_muted.to_array(),
            });
        }
        return;
    }

    // ------------------------------------------------------------------
    // Legacy path: string-based explorer_items
    // ------------------------------------------------------------------
    let legacy_items: Option<&[String]> = content.and_then(|c| c.explorer_items.as_deref());
    let legacy_button = content.and_then(|c| c.explorer_empty_button.as_deref());

    if let Some(items) = legacy_items {
        if !items.is_empty() {
            for (item_idx, item) in items.iter().enumerate() {
                if *y_off + row_h > max_y {
                    break;
                }
                tree.push(ShellWidget::ListItem {
                    id: WidgetId::list_item(10 + item_idx),
                    rect: Rect::new(
                        sidebar_rect.x + pad + 14.0,
                        *y_off + 2.0,
                        inner_w - 20.0,
                        14.0,
                    ),
                    label: item.clone(),
                    fill_color: tokens.sidebar_file_item.to_array(),
                    accent_indicator: None,
                    state: InteractionState::Normal,
                });
                *y_off += row_h;
            }
        } else if let Some(label) = legacy_button {
            let btn_w = 140.0;
            let btn_h = 30.0;
            tree.push(ShellWidget::Button {
                id: WidgetId::button(30),
                rect: Rect::new(sidebar_rect.x + pad + 10.0, *y_off + 8.0, btn_w, btn_h),
                label: label.to_string(),
                fill_color: tokens.accent.to_array(),
                state: InteractionState::Normal,
            });
        }
    } else if let Some(label) = legacy_button {
        let btn_w = 140.0;
        let btn_h = 30.0;
        tree.push(ShellWidget::Button {
            id: WidgetId::button(30),
            rect: Rect::new(sidebar_rect.x + pad + 10.0, *y_off + 8.0, btn_w, btn_h),
            label: label.to_string(),
            fill_color: tokens.accent.to_array(),
            state: InteractionState::Normal,
        });
    }

    // No legacy items and content is None → hardcoded placeholders (original fallback)
    if legacy_items.is_none()
        && panel_items.is_none()
        && content.map(|c| c.explorer_panel_title.is_some()).unwrap_or(false)
    {
        // Had a title with no items — keep placeholder
    }
}
