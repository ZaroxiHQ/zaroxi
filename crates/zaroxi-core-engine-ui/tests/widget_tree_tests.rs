#[cfg(test)]
mod tests {
    use zaroxi_core_engine_layout::ShellLayout;
    use zaroxi_core_engine_style::EngineTheme;
    use zaroxi_core_engine_ui::{build_shell_widget_tree, InteractionState, ShellWidget};

    #[test]
    fn widget_tree_preserves_deterministic_order() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let theme = EngineTheme::dark();
        let tree = build_shell_widget_tree(&layout, &theme);

        assert!(tree.len() > 10, "expected >10 widgets in tree, got {}", tree.len());

        // First widget must be AppBackground
        assert!(
            matches!(tree.widgets[0], ShellWidget::AppBackground { .. }),
            "first widget must be AppBackground"
        );
    }

    #[test]
    fn widget_tree_contains_tab_and_rail_items() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let theme = EngineTheme::dark();
        let tree = build_shell_widget_tree(&layout, &theme);

        // At least one Tab widget
        let tab_count = tree
            .widgets
            .iter()
            .filter(|w| matches!(w, ShellWidget::Tab { .. }))
            .count();
        assert!(tab_count >= 1, "expected >=1 Tab widget, got {}", tab_count);

        // At least one RailItem widget
        let rail_count = tree
            .widgets
            .iter()
            .filter(|w| matches!(w, ShellWidget::RailItem { .. }))
            .count();
        assert!(rail_count >= 4, "expected >=4 RailItems, got {}", rail_count);

        // At least one StatusSegment
        let seg_count = tree
            .widgets
            .iter()
            .filter(|w| matches!(w, ShellWidget::StatusSegment { .. }))
            .count();
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
        let theme = EngineTheme::dark();
        let tree = build_shell_widget_tree(&layout, &theme);

        // Hitting the app background (a RegionSurface) should return None
        // because RegionSurface has no hit_target.
        let hit_at_origin = tree.hit_test(5.0, 5.0);
        assert!(hit_at_origin.is_none(), "app background should not be hittable");
    }

    #[test]
    fn hover_state_is_stable_and_clears_correctly() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let theme = EngineTheme::dark();
        let mut tree = build_shell_widget_tree(&layout, &theme);

        // Find a Tab widget and simulate hover
        let tab_idx = tree
            .widgets
            .iter()
            .position(|w| matches!(w, ShellWidget::Tab { .. }));
        assert!(tab_idx.is_some(), "must have a Tab widget");

        let idx = tab_idx.unwrap();
        tree.set_state_at(idx, InteractionState::Hover);
        assert_eq!(tree.widgets[idx].get_state(), InteractionState::Hover);

        // Clear all hover
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
        let theme = EngineTheme::dark();
        let tree = build_shell_widget_tree(&layout, &theme);

        // Convert to surface set and verify the tab's surface is present
        let set = tree.to_surface_set();
        assert!(!set.tabs.is_empty(), "tabs must be present in surface set");

        let active_tab = &set.tabs[0];
        assert!(active_tab.accent_strip.is_some(), "active tab must have accent strip");
    }
}
