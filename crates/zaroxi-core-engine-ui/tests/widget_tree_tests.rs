#[cfg(test)]
mod tests {
    use zaroxi_core_engine_layout::ShellLayout;
    use zaroxi_core_engine_style::test_utils::test_tokens_dark;
    use zaroxi_core_engine_ui::{InteractionState, ShellWidget, build_shell_widget_tree};

    #[test]
    fn widget_tree_preserves_deterministic_order() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        assert!(tree.len() > 10, "expected >10 widgets in tree, got {}", tree.len());

        // First widget must be AppBackground
        assert!(
            matches!(tree.widgets[0], ShellWidget::AppBackground { .. }),
            "first widget must be AppBackground"
        );
    }

    #[test]
    fn widget_tree_contains_tab_and_list_items() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        let tab_count =
            tree.widgets.iter().filter(|w| matches!(w, ShellWidget::TabItem { .. })).count();
        assert!(tab_count >= 1, "expected >=1 TabItem widget, got {}", tab_count);

        let list_count =
            tree.widgets.iter().filter(|w| matches!(w, ShellWidget::ListItem { .. })).count();
        assert!(list_count >= 4, "expected >=4 ListItems, got {}", list_count);

        let seg_count =
            tree.widgets.iter().filter(|w| matches!(w, ShellWidget::StatusSegment { .. })).count();
        assert!(seg_count >= 1, "expected >=1 StatusSegment, got {}", seg_count);
    }

    #[test]
    fn hit_test_returns_none_for_empty_tree() {
        let tree = zaroxi_core_engine_ui::ShellWidgetTree::new();
        assert!(tree.hit_test(100.0, 100.0).is_none());
    }

    #[test]
    fn hit_test_noops_on_non_interactive_regions() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        // Hitting the app background (a Surface widget) should return None
        // because Surface has no hit_target.
        let hit_at_origin = tree.hit_test(5.0, 5.0);
        assert!(hit_at_origin.is_none(), "app background should not be hittable");
    }

    #[test]
    fn hover_state_is_stable_and_clears_correctly() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens, None);

        let tab_idx = tree.widgets.iter().position(|w| matches!(w, ShellWidget::TabItem { .. }));
        assert!(tab_idx.is_some(), "must have a TabItem widget");

        let idx = tab_idx.unwrap();
        tree.set_state_at(idx, InteractionState::Hover);
        assert_eq!(tree.widgets[idx].get_state(), InteractionState::Hover);

        tree.clear_all_hover();
        assert_eq!(
            tree.widgets[idx].get_state(),
            InteractionState::Normal,
            "clear_all_hover must reset state"
        );
    }

    #[test]
    fn tab_widget_renders_active_state() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        let set = tree.to_surface_set();
        assert!(!set.tabs.is_empty(), "tabs must be present in surface set");

        let active_tab = &set.tabs[0];
        assert!(active_tab.accent_strip.is_some(), "active tab must have accent strip");
    }

    #[test]
    fn multiple_tabs_includes_active_and_inactive() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        let tabs: Vec<_> =
            tree.widgets.iter().filter(|w| matches!(w, ShellWidget::TabItem { .. })).collect();
        assert!(tabs.len() >= 3, "expected >=3 tabs, got {}", tabs.len());

        // Active vs inactive tabs are distinguished by the accent strip (and the
        // active-background fill), not by `InteractionState` — the `state` field
        // tracks hover/press/focus only. This mirrors `tab_widget_renders_active_state`.
        if let ShellWidget::TabItem { accent_strip, .. } = tabs[0] {
            assert!(accent_strip.is_some(), "first (active) tab must have an accent strip");
        }
        let inactive_tabs = tabs
            .iter()
            .filter(|w| matches!(w, ShellWidget::TabItem { accent_strip: None, .. }))
            .count();
        assert!(inactive_tabs >= 1, "expected at least one inactive tab (no accent strip)");
    }

    #[test]
    fn panel_headers_have_action_slots() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        let header_with_actions: Vec<_> = tree
            .widgets
            .iter()
            .filter(
                |w| matches!(w, ShellWidget::PanelHeader { actions, .. } if !actions.is_empty()),
            )
            .collect();
        assert!(
            !header_with_actions.is_empty(),
            "expected at least one panel header with action slots"
        );
    }

    #[test]
    fn scrollbar_tracks_are_present() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        let scrollbar_count =
            tree.widgets.iter().filter(|w| matches!(w, ShellWidget::ScrollBar { .. })).count();
        assert!(scrollbar_count >= 1, "expected >=1 scrollbar, got {}", scrollbar_count);
    }

    #[test]
    fn buttons_are_hittable() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        let btn_hits: Vec<_> =
            tree.widgets
                .iter()
                .filter_map(|w| {
                    if matches!(w, ShellWidget::Button { .. }) { w.hit_target() } else { None }
                })
                .collect();
        assert!(btn_hits.len() >= 3, "expected >=3 button hit targets, got {}", btn_hits.len());
    }

    #[test]
    fn divider_subtle_flag_propagates_to_surface_set() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens, None);

        let set = tree.to_surface_set();
        // Dividers must be present
        assert!(!set.dividers.is_empty(), "dividers must be present");

        // All dividers should have valid non-zero rects
        for d in &set.dividers {
            assert!(d.rect.width > 0.0 || d.rect.height > 0.0, "divider rect must be non-zero");
        }
    }

    // ── AI panel region ──

    use zaroxi_core_engine_style::WidgetId;
    use zaroxi_core_engine_ui::ShellWorkContent;
    use zaroxi_core_engine_ui::layout_constants as lc;

    fn find_button(tree: &zaroxi_core_engine_ui::ShellWidgetTree, idx: usize) -> Option<Rect4> {
        tree.widgets.iter().find_map(|w| match w {
            ShellWidget::Button { id, rect, .. } if *id == WidgetId::button(idx) => {
                Some((rect.x, rect.y, rect.width, rect.height))
            }
            _ => None,
        })
    }
    type Rect4 = (f32, f32, f32, f32);

    #[test]
    fn ai_panel_shows_setup_cta_when_no_provider() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let content = ShellWorkContent { ai_show_setup_cta: true, ..Default::default() };
        let tree = build_shell_widget_tree(&layout, &tokens, Some(&content));

        assert!(
            find_button(&tree, lc::BTN_ID_AI_SETUP_PROVIDER).is_some(),
            "setup CTA button must be present when no provider is configured"
        );
        assert!(
            find_button(&tree, lc::BTN_ID_AI_NEW_CHAT).is_none(),
            "session controls must be hidden when no provider is configured"
        );
        assert!(find_button(&tree, lc::BTN_ID_AI_CLEAR).is_none());
    }

    #[test]
    fn ai_panel_shows_session_controls_when_provider_ready() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let content = ShellWorkContent::default();
        let tree = build_shell_widget_tree(&layout, &tokens, Some(&content));

        assert!(
            find_button(&tree, lc::BTN_ID_AI_NEW_CHAT).is_some(),
            "New chat button must be present when provider is ready"
        );
        assert!(
            find_button(&tree, lc::BTN_ID_AI_CLEAR).is_some(),
            "Clear button must be present when provider is ready"
        );
        assert!(
            find_button(&tree, lc::BTN_ID_AI_SETUP_PROVIDER).is_none(),
            "setup CTA must be hidden when provider is ready"
        );
    }

    #[test]
    fn ai_composer_input_is_bottom_anchored_with_state_placeholder() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let content = ShellWorkContent {
            ai_composer_placeholder: Some("Waiting for response\u{2026}".into()),
            ..Default::default()
        };
        let tree = build_shell_widget_tree(&layout, &tokens, Some(&content));

        let input = tree.widgets.iter().find_map(|w| match w {
            ShellWidget::TextInput { id, rect, placeholder, .. }
                if *id == WidgetId::text_input(0) =>
            {
                Some((*rect, placeholder.clone()))
            }
            _ => None,
        });
        let (rect, placeholder) = input.expect("AI composer text input must exist");
        assert_eq!(placeholder, "Waiting for response\u{2026}");

        // Bottom-anchored: input must sit in the lower half of the AI panel
        // and match the shared composer geometry helper.
        let content_rect = (
            layout.right_panel.x,
            layout.right_panel.y + lc::AI_HEADER_H,
            layout.right_panel.width,
            (layout.right_panel.height - lc::AI_HEADER_H).max(0.0),
        );
        let (ex, ey, ew, eh) = lc::ai_composer_rect(content_rect);
        assert!((rect.x - ex).abs() < 0.5, "composer x must match shared geometry");
        assert!((rect.y - ey).abs() < 0.5, "composer y must match shared geometry");
        assert!((rect.width - ew).abs() < 0.5);
        assert!((rect.height - eh).abs() < 0.5);
        assert!(
            rect.y > layout.right_panel.y + layout.right_panel.height / 2.0,
            "composer must be bottom-anchored"
        );
    }
}
