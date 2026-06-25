use crate::ShellWorkContent;
use crate::layout_constants::{
    AI_ACTION_BTN_GAP, AI_ACTION_BTN_H, AI_ACTION_BTN_W, AI_ACTION_X_INSET, AI_HEADER_H,
    AI_INPUT_H, BRAND_ACCENT_BOTTOM_INSET, BRAND_ACCENT_LEFT, BRAND_ACCENT_TOP, BRAND_ACCENT_W,
    BTN_ID_AI_APPLY, BTN_ID_AI_CLOSE, BTN_ID_AI_EXPLAIN, BTN_ID_AI_REJECT, BTN_ID_AI_REVIEW,
    BTN_ID_CLOSE_WINDOW, BTN_ID_EXPLORER_CTA, BTN_ID_MAXIMIZE, BTN_ID_MINIMIZE,
    BTN_ID_TERMINAL_CLOSE, DIVIDER_SPACE, EMPTY_STATE_H, EMPTY_STATE_W, EMPTY_STATE_X_OFFSET,
    EMPTY_STATE_Y_OFFSET, EXPLORER_CTA_BTN_H, EXPLORER_CTA_BTN_W, EXPLORER_CTA_BTN_X_EXTRA,
    EXPLORER_CTA_BTN_Y_OFFSET, EXPLORER_HEADER_H, EXPLORER_HEADER_TO_ROWS_GAP, EXPLORER_INDENT_PX,
    EXPLORER_MAX_Y_INSET, EXPLORER_ROW_H, EXPLORER_ROW_TEXT_INSET, EXPLORER_ROW_VIS_H,
    EXPLORER_ROW_W_REDUCTION, EXPLORER_SEARCH_TO_ROWS_GAP, PANEL_ACTION_V_REDUCTION,
    PANEL_ACTION_W, PANEL_ACTION_X_INSET, PANEL_ACTION_Y_INSET, RAIL_STRIP_H, SB_BOTTOM_SPEC,
    SB_EDITOR_SPEC, SB_INTERACTIVE_GUTTER_PAD, SB_SIDEBAR_SPEC, SCROLLBAR_ID_BOTTOM,
    SCROLLBAR_ID_EDITOR, SCROLLBAR_ID_SIDEBAR, SEARCH_BAR_H, SEARCH_TO_DIVIDER_GAP, SIDEBAR_PAD,
    STATUSBAR_BADGE_W, STATUSBAR_PILL_H_INSET, STATUSBAR_PILL_Y, TAB_W_ACTIVE_EXTRA,
    TAB_W_INACTIVE, TAB_Y_HANG, TERMINAL_HEADER_H, TERMINAL_TAB_GAP, TERMINAL_TAB_H,
    TERMINAL_TAB_W, TERMINAL_TAB_X_OFFSET, TERMINAL_TAB_Y_OFFSET, TOOLBAR_BTN_GAP,
    TOOLBAR_BTN_RIGHT_MARGIN, TOOLBAR_BTN_V_INSET, TOOLBAR_BTN_W, compute_scrollbar_geometry,
};
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
        let btn_w = TOOLBAR_BTN_W;
        let btn_h = layout.top_bar.height - TOOLBAR_BTN_V_INSET * 2.0;
        let btn_y = layout.top_bar.y + TOOLBAR_BTN_V_INSET;
        let btn_x =
            layout.top_bar.x + layout.top_bar.width - (btn_w * 3.0 + TOOLBAR_BTN_RIGHT_MARGIN);
        for (idx, (label, accent)) in [("_", false), ("[ ]", false), ("x", true)].iter().enumerate()
        {
            let bx = btn_x + idx as f32 * (btn_w + TOOLBAR_BTN_GAP);
            let id = match idx {
                0 => WidgetId::button(BTN_ID_MINIMIZE),
                1 => WidgetId::button(BTN_ID_MAXIMIZE),
                2 => WidgetId::button(BTN_ID_CLOSE_WINDOW),
                _ => WidgetId::button(idx),
            };
            tree.push(ShellWidget::Button {
                id,
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
                layout.top_bar.x + BRAND_ACCENT_LEFT,
                layout.top_bar.y + BRAND_ACCENT_TOP,
                BRAND_ACCENT_W,
                layout.top_bar.height - BRAND_ACCENT_BOTTOM_INSET,
            ),
            fill_color: tokens.toolbar_brand_accent.to_array(),
            border_color: None,
            border_width: 0.0,
        });
    }

    // ── 3. Activity rail + sidebar (left column) ──
    // Rail sits at the bottom of the left column, between the sidebar content
    // and the status bar.  Background via shell shape pass (before text pass so
    // cockpit icon glyphs render on top).  Rail icons + highlights are cockpit-
    // rendered (vello + cosmic-text layers).
    let rail_strip_h = RAIL_STRIP_H;
    let sidebar_h = (layout.left_panel.height - rail_strip_h).max(0.0);
    let sidebar_rect =
        Rect::new(layout.left_panel.x, layout.left_panel.y, layout.left_panel.width, sidebar_h);
    tree.push(ShellWidget::Surface {
        rect: sidebar_rect,
        fill_color: tokens.sidebar_background.to_array(),
        border_color: None,
        border_width: 0.0,
    });

    // ── 4. Sidebar content (fills the left column above the rail) ──
    let sidebar_w = layout.left_panel.width;
    if sidebar_w > 0.0 {
        let sx = 0.0;
        let sidebar_rect = Rect::new(sx, layout.left_panel.y, sidebar_w, sidebar_h);

        let pad = SIDEBAR_PAD;
        let search_h = SEARCH_BAR_H;
        let inner_w = sidebar_w - pad * 2.0;
        let mut y_off = layout.left_panel.y + pad;

        // Search bar area
        tree.push(ShellWidget::Surface {
            rect: Rect::new(sidebar_rect.x + pad, y_off, inner_w, search_h),
            fill_color: tokens.sidebar_input.to_array(),
            border_color: None,
            border_width: 0.0,
        });
        y_off += search_h + SEARCH_TO_DIVIDER_GAP;

        // Subtle divider below search
        tree.push(ShellWidget::Divider {
            rect: Rect::new(sidebar_rect.x + pad, y_off, inner_w, 2.0),
            color: tokens.sidebar_search_divider.to_array(),
            orientation: DividerOrientation::Horizontal,
            subtle: true,
        });
        y_off += DIVIDER_SPACE;

        // Explorer panel
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
            let (sb_x, track_y, _sb_w, track_h, thumb_h) = compute_scrollbar_geometry(
                (sidebar_rect.x, sidebar_rect.y, sidebar_rect.width, sidebar_rect.height),
                &SB_SIDEBAR_SPEC,
                0.0,
            );
            let content_right = sidebar_rect.x + sidebar_rect.width;
            let interactive_x = sb_x - SB_INTERACTIVE_GUTTER_PAD;
            let interactive_w = (content_right - interactive_x).max(0.0);
            let track_rect = Rect::new(interactive_x, track_y, interactive_w, track_h);
            tree.push(ShellWidget::ScrollBar {
                id: WidgetId::scrollbar(SCROLLBAR_ID_SIDEBAR),
                track_rect,
                thumb_rect: Rect::new(interactive_x, track_rect.y, interactive_w, thumb_h),
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

    // TabItem widgets: driven by ShellWorkContent.editor_tabs + non_file_tabs
    if layout.content_tab_strip.width > 80.0 && layout.content_tab_strip.height > 4.0 {
        let tab_h = layout.content_tab_strip.height + TAB_Y_HANG;
        let tab_y = layout.content_tab_strip.y - TAB_Y_HANG;
        let active_idx = content.and_then(|c| c.active_tab_index).unwrap_or(0);
        let mut tabs: Vec<(String, bool, usize)> = content
            .and_then(|c| c.editor_tabs.as_ref())
            .map(|ts| {
                ts.iter()
                    .enumerate()
                    .map(|(i, label)| (label.clone(), i == active_idx, i))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(nf) = content.and_then(|c| c.editor_non_file_tabs.as_ref()) {
            let base = tabs.len();
            for (i, (label, _kind)) in nf.iter().enumerate() {
                let idx = base + i;
                tabs.push((label.clone(), idx == active_idx, idx));
            }
        }
        if tabs.is_empty() {
            tabs = vec![
                ("main.rs".into(), true, 0),
                ("lib.rs".into(), false, 1),
                ("mod.rs".into(), false, 2),
            ];
        }
        let tab_w = TAB_W_INACTIVE;
        let mut tx = layout.content_tab_strip.x;

        for (label, active, idx) in tabs {
            let tw = if active { tab_w + TAB_W_ACTIVE_EXTRA } else { tab_w };
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
                layout.content_area.x + EMPTY_STATE_X_OFFSET,
                layout.content_area.y + EMPTY_STATE_Y_OFFSET,
                EMPTY_STATE_W,
                EMPTY_STATE_H,
            ),
            message: "No file open".into(),
            fill_color: [0.0, 0.0, 0.0, 0.0],
            text_color: tokens.text_muted.to_array(),
        });
    }

    // Editor scrollbar (right edge)
    if layout.content_area.height > 40.0 && layout.content_area.width > 20.0 {
        let (sb_x, track_y, _sb_w, track_h, thumb_h) = compute_scrollbar_geometry(
            (
                layout.content_area.x,
                layout.content_area.y,
                layout.content_area.width,
                layout.content_area.height,
            ),
            &SB_EDITOR_SPEC,
            0.0,
        );
        // Interactive gutter extends left from the visual rail by GUTTER_PAD.
        // The right edge is the content area's right edge — the canonical
        // boundary between editor scrollbar territory and AI panel / window edge.
        // The interactive rect NEVER extends past content_area_right, so it does
        // not overlap the AI panel and does not create ownership ambiguity.
        let content_right = layout.content_area.x + layout.content_area.width;
        let interactive_x = sb_x - SB_INTERACTIVE_GUTTER_PAD;
        let interactive_w = (content_right - interactive_x).max(0.0);
        let track_rect = Rect::new(interactive_x, track_y, interactive_w, track_h);
        tree.push(ShellWidget::ScrollBar {
            id: WidgetId::scrollbar(SCROLLBAR_ID_EDITOR),
            track_rect,
            thumb_rect: Rect::new(interactive_x, track_rect.y, interactive_w, thumb_h),
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
        let header_h = TERMINAL_HEADER_H;
        let header_rect = Rect::new(
            layout.bottom_panel.x,
            layout.bottom_panel.y,
            layout.bottom_panel.width,
            header_h,
        );
        // Close action button
        let action_w = PANEL_ACTION_W;
        let action_x = header_rect.x + header_rect.width - action_w - PANEL_ACTION_X_INSET;
        let action_y = header_rect.y + PANEL_ACTION_Y_INSET;
        let action_h = header_rect.height - PANEL_ACTION_V_REDUCTION;
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
            id: WidgetId::button(BTN_ID_TERMINAL_CLOSE),
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
        let tab_w = TERMINAL_TAB_W;
        let tab_h = TERMINAL_TAB_H;
        let tab_y = layout.bottom_panel.y + TERMINAL_TAB_Y_OFFSET;
        let mut tab_x = layout.bottom_panel.x + TERMINAL_TAB_X_OFFSET;
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
            tab_x += tab_w + TERMINAL_TAB_GAP;
        }

        // Scrollbar on right edge of terminal panel (skipping header)
        let (sb_x, track_y, _sb_w, track_h, thumb_h) = compute_scrollbar_geometry(
            (
                layout.bottom_panel.x,
                layout.bottom_panel.y,
                layout.bottom_panel.width,
                layout.bottom_panel.height,
            ),
            &SB_BOTTOM_SPEC,
            header_h,
        );
        let content_right = layout.bottom_panel.x + layout.bottom_panel.width;
        let interactive_x = sb_x - SB_INTERACTIVE_GUTTER_PAD;
        let interactive_w = (content_right - interactive_x).max(0.0);
        let track_rect = Rect::new(interactive_x, track_y, interactive_w, track_h);
        tree.push(ShellWidget::ScrollBar {
            id: WidgetId::scrollbar(SCROLLBAR_ID_BOTTOM),
            track_rect,
            thumb_rect: Rect::new(interactive_x, track_rect.y, interactive_w, thumb_h),
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
        let header_h = AI_HEADER_H;
        let header_rect = Rect::new(
            layout.right_panel.x,
            layout.right_panel.y,
            layout.right_panel.width,
            header_h,
        );
        let action_w = PANEL_ACTION_W;
        let action_x = header_rect.x + header_rect.width - action_w - AI_ACTION_X_INSET;
        let action_y = header_rect.y + PANEL_ACTION_Y_INSET;
        let action_h = header_rect.height - PANEL_ACTION_V_REDUCTION;
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
            id: WidgetId::button(BTN_ID_AI_CLOSE),
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
        let btn_w = AI_ACTION_BTN_W;
        let btn_h = AI_ACTION_BTN_H;
        let btn_y = layout.right_panel.y + header_h + AI_ACTION_BTN_GAP;
        let mut btn_x = layout.right_panel.x + AI_ACTION_X_INSET;
        for (label, idx) in &[
            ("Explain", BTN_ID_AI_EXPLAIN as usize),
            ("Review", BTN_ID_AI_REVIEW as usize),
            ("Apply", BTN_ID_AI_APPLY as usize),
            ("Reject", BTN_ID_AI_REJECT as usize),
        ] {
            tree.push(ShellWidget::Button {
                id: WidgetId::button(*idx),
                rect: Rect::new(btn_x, btn_y, btn_w, btn_h),
                label: label.to_string(),
                fill_color: tokens.rail_background.to_array(),
                state: InteractionState::Normal,
            });
            btn_x += btn_w + AI_ACTION_BTN_GAP;
        }

        // AI prompt text input
        let input_y = btn_y + btn_h + AI_ACTION_BTN_GAP;
        let input_w = layout.right_panel.width - AI_ACTION_X_INSET * 2.0;
        tree.push(ShellWidget::TextInput {
            id: WidgetId::text_input(0),
            rect: Rect::new(layout.right_panel.x + AI_ACTION_X_INSET, input_y, input_w, AI_INPUT_H),
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
        let pill_h = layout.bottom_bar.height - STATUSBAR_PILL_H_INSET;
        let pill_y = layout.bottom_bar.y + STATUSBAR_PILL_Y;

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
            let badge_w = STATUSBAR_BADGE_W;
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
    let sidebar_w = sidebar_rect.width;
    let row_h = EXPLORER_ROW_H;

    // ── Extensions sidebar (replaces explorer when active) ──
    if let Some(items) = content.and_then(|c| c.extension_sidebar_items.as_ref()) {
        let header_h = 30.0;
        tree.push(ShellWidget::Surface {
            rect: Rect::new(sidebar_rect.x, *y_off, sidebar_w, header_h),
            fill_color: tokens.panel_header_background.to_array(),
            border_color: None,
            border_width: 0.0,
        });
        *y_off += header_h + 4.0;
        for (i, (name, _id)) in items.iter().enumerate() {
            let row_y = *y_off + i as f32 * row_h;
            tree.push(ShellWidget::ListItem {
                id: WidgetId::list_item(100 + i),
                rect: Rect::new(sidebar_rect.x + pad, row_y, inner_w, row_h),
                label: name.clone(),
                fill_color: tokens.sidebar_background.to_array(),
                accent_indicator: None,
                state: InteractionState::Normal,
            });
        }
        *y_off += items.len() as f32 * row_h;
        return;
    }

    let row_h = EXPLORER_ROW_H;
    let sidebar_w = sidebar_rect.width;
    let max_y = layout.left_panel.y + layout.left_panel.height - EXPLORER_MAX_Y_INSET;

    // ------------------------------------------------------------------
    // Panel header
    // ------------------------------------------------------------------
    let panel_title = content.and_then(|c| c.explorer_panel_title.as_deref());
    if let Some(title) = panel_title {
        let hdr_h = EXPLORER_HEADER_H;
        if *y_off + hdr_h <= max_y {
            tree.push(ShellWidget::ListSectionHeader {
                rect: Rect::new(sidebar_rect.x, *y_off, sidebar_w, hdr_h),
                label: title.to_string(),
                fill_color: tokens.panel_header_background.to_array(),
                text_color: tokens.panel_header_text.to_array(),
            });
            *y_off += hdr_h + EXPLORER_HEADER_TO_ROWS_GAP;
        }
    }

    // ------------------------------------------------------------------
    // Structured panel items (new path)
    // ------------------------------------------------------------------
    let panel_items = content.and_then(|c| c.explorer_panel_items.as_deref());
    if let Some(items) = panel_items {
        if !items.is_empty() {
            // Vertical scroll: skip the first `scroll_top` rows. The absolute
            // `item_idx` stays encoded in the widget id so click dispatch and
            // hover bridging keep resolving to the correct entry regardless of
            // scroll position.
            let scroll_top = content.map(|c| c.explorer_scroll_top).unwrap_or(0);
            let row_y_inset = (EXPLORER_ROW_H - EXPLORER_ROW_VIS_H) / 2.0;
            // Rows begin flush at the top of the explorer content area, matching
            // the render path (`rail.rs`). The header above (when present) is not
            // rendered and is non-interactive, so it neither shows nor blocks hits.
            // Rows begin below the rendered search box, matching the render
            // path (`rail.rs`) so hit targets line up with the visible rows.
            let mut row_y = layout.left_panel.y + pad + SEARCH_BAR_H + EXPLORER_SEARCH_TO_ROWS_GAP;
            for (item_idx, item) in items.iter().enumerate() {
                if item_idx < scroll_top {
                    continue;
                }
                if row_y + row_h > max_y {
                    break;
                }
                let indent_px = item.depth as f32 * EXPLORER_INDENT_PX;
                tree.push(ShellWidget::ListItem {
                    id: WidgetId::list_item(10 + item_idx),
                    rect: Rect::new(
                        sidebar_rect.x + pad + EXPLORER_ROW_TEXT_INSET + indent_px,
                        row_y + row_y_inset,
                        (inner_w - EXPLORER_ROW_W_REDUCTION - indent_px).max(4.0),
                        EXPLORER_ROW_VIS_H,
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
                row_y += row_h;
            }
            return;
        }

        // Empty panel — show empty state or button
        let btn_label = content.and_then(|c| c.explorer_empty_button.as_deref());
        let empty_msg = content.and_then(|c| c.explorer_empty_message.as_deref());
        if let Some(label) = btn_label {
            let btn_w = EXPLORER_CTA_BTN_W;
            let btn_h = EXPLORER_CTA_BTN_H;
            tree.push(ShellWidget::Button {
                id: WidgetId::button(BTN_ID_EXPLORER_CTA),
                rect: Rect::new(
                    sidebar_rect.x + pad + EXPLORER_CTA_BTN_X_EXTRA,
                    *y_off + EXPLORER_CTA_BTN_Y_OFFSET,
                    btn_w,
                    btn_h,
                ),
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
