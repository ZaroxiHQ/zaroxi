#[cfg(test)]
mod tests {
    use zaroxi_core_engine_layout::ShellLayout;
    use zaroxi_core_engine_style::test_utils::test_tokens_dark;
    use zaroxi_core_engine_ui::{
        PointerButton, ShellWidget, WidgetAction, WidgetInteractionModel, build_shell_widget_tree,
    };

    #[test]
    fn hover_tracks_widget_and_emits_action() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        let actions = model.on_pointer_moved(&mut tree, 5.0, 5.0);
        assert!(
            !actions.iter().any(|a| matches!(a, WidgetAction::HoverChanged(_))),
            "first move to non-interactive region emits no hover change"
        );
        assert!(model.hovered_widget_idx.is_none());

        let tab_x = layout.content_tab_strip.x + 10.0;
        let tab_y = layout.content_tab_strip.y + 5.0;
        let actions = model.on_pointer_moved(&mut tree, tab_x, tab_y);
        assert!(
            actions.iter().any(|a| matches!(a, WidgetAction::HoverChanged(Some(_)))),
            "hover over tab should emit HoverChanged"
        );
        assert!(model.hovered_widget_idx.is_some());
    }

    #[test]
    fn hover_same_widget_does_not_reemit() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        let tab_x = layout.content_tab_strip.x + 10.0;
        let tab_y = layout.content_tab_strip.y + 5.0;
        let _ = model.on_pointer_moved(&mut tree, tab_x, tab_y);

        let actions = model.on_pointer_moved(&mut tree, tab_x + 2.0, tab_y + 2.0);
        let hover_count =
            actions.iter().filter(|a| matches!(a, WidgetAction::HoverChanged(_))).count();
        assert_eq!(hover_count, 0, "second move on same widget should not re-emit hover");
    }

    #[test]
    fn pointer_leave_clears_hover_and_emits() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        let tab_x = layout.content_tab_strip.x + 10.0;
        let tab_y = layout.content_tab_strip.y + 5.0;
        let _ = model.on_pointer_moved(&mut tree, tab_x, tab_y);
        assert!(model.hovered_widget_idx.is_some());

        let actions = model.on_pointer_leave(&mut tree);
        assert!(model.hovered_widget_idx.is_none());
        assert!(
            actions.iter().any(|a| matches!(a, WidgetAction::HoverChanged(None))),
            "pointer leave should emit HoverChanged(None)"
        );
    }

    #[test]
    fn press_and_release_on_same_widget_emits_activated() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        let tab_x = layout.content_tab_strip.x + 10.0;
        let tab_y = layout.content_tab_strip.y + 5.0;

        let _ = model.on_pointer_down(&mut tree, tab_x, tab_y, PointerButton::Primary);
        assert!(model.pressed_widget_idx.is_some(), "press should record pressed widget");

        let actions = model.on_pointer_up(&mut tree, tab_x, tab_y, PointerButton::Primary);
        assert!(
            actions.iter().any(|a| matches!(a, WidgetAction::Activated(_))),
            "same-widget press+release should emit Activated"
        );
    }

    #[test]
    fn press_and_release_on_different_widgets_does_not_activate() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        let tab_x = layout.content_tab_strip.x + 10.0;
        let tab_y = layout.content_tab_strip.y + 5.0;
        let rail_x = 10.0;
        let rail_y = layout.left_panel.y + 10.0;

        let _ = model.on_pointer_down(&mut tree, tab_x, tab_y, PointerButton::Primary);
        let actions = model.on_pointer_up(&mut tree, rail_x, rail_y, PointerButton::Primary);
        assert!(
            !actions.iter().any(|a| matches!(a, WidgetAction::Activated(_))),
            "different-widget press+release should not activate"
        );
    }

    #[test]
    fn focus_traversal_wraps_and_changes_focused_widget() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        assert!(model.focused_widget_idx.is_none());

        let actions = model.focus_next(&mut tree);
        assert!(model.focused_widget_idx.is_some(), "focus_next should set focus");
        assert!(
            actions.iter().any(|a| matches!(a, WidgetAction::FocusChanged(Some(_)))),
            "focus next should emit FocusChanged"
        );

        let first_focus = model.focused_widget_idx;

        let _ = model.focus_previous(&mut tree);
        let last_focus = model.focused_widget_idx;
        assert_ne!(first_focus, last_focus, "focus_previous should move to a different widget");

        let _ = model.focus_next(&mut tree);
        assert_eq!(
            model.focused_widget_idx, first_focus,
            "focus_next after previous should return to first"
        );
    }

    #[test]
    fn activate_focused_emits_activated_for_focusable_widget() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        let _ = model.focus_next(&mut tree);
        assert!(model.focused_widget_idx.is_some());

        let actions = model.activate_focused(&mut tree);
        assert!(
            actions.iter().any(|a| matches!(a, WidgetAction::Activated(_))),
            "activate_focused should emit Activated"
        );
    }

    #[test]
    fn scroll_offset_set_and_get_roundtrips() {
        let id = zaroxi_core_engine_ui::WidgetId::Scrollbar { index: 1 };
        let mut model = WidgetInteractionModel::new();

        assert!((model.get_scroll_offset(&id) - 0.0).abs() < 0.001);
        model.set_scroll_offset(&id, 0.5);
        assert!((model.get_scroll_offset(&id) - 0.5).abs() < 0.001);

        model.set_scroll_offset(&id, -0.2);
        assert!((model.get_scroll_offset(&id) - 0.0).abs() < 0.001);

        model.set_scroll_offset(&id, 1.8);
        assert!((model.get_scroll_offset(&id) - 1.0).abs() < 0.001);
    }

    #[test]
    fn scroll_offset_clamps_to_range() {
        let id = zaroxi_core_engine_ui::WidgetId::Scrollbar { index: 0 };
        let mut model = WidgetInteractionModel::new();

        model.set_scroll_offset(&id, -0.5);
        assert!((model.get_scroll_offset(&id) - 0.0).abs() < 0.001);

        model.set_scroll_offset(&id, 3.0);
        assert!((model.get_scroll_offset(&id) - 1.0).abs() < 0.001);
    }

    #[test]
    fn scroll_offset_application_updates_thumb_positions() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        let original_thumbs: Vec<_> = tree
            .widgets
            .iter()
            .filter_map(|w| {
                if let ShellWidget::ScrollBar { thumb_rect, .. } = w {
                    Some(thumb_rect.y)
                } else {
                    None
                }
            })
            .collect();
        assert!(!original_thumbs.is_empty(), "tree must have scrollbars");

        model.set_scroll_offset(&zaroxi_core_engine_ui::WidgetId::Scrollbar { index: 1 }, 0.5);
        model.apply_scroll_offsets(&mut tree);

        let updated_thumbs: Vec<_> = tree
            .widgets
            .iter()
            .filter_map(|w| {
                if let ShellWidget::ScrollBar { id, thumb_rect, .. } = w {
                    if matches!(id, zaroxi_core_engine_ui::WidgetId::Scrollbar { index: 1 }) {
                        Some(thumb_rect.y)
                    } else {
                        Some(thumb_rect.y)
                    }
                } else {
                    None
                }
            })
            .collect();

        let editor_thumb_moved = original_thumbs
            .iter()
            .zip(updated_thumbs.iter())
            .any(|(orig, upd)| (orig - upd).abs() > 0.1);
        assert!(editor_thumb_moved, "at least one scrollbar thumb should move with offset 0.5");
    }

    #[test]
    fn apply_to_tree_restores_hover_and_press() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let mut tree = build_shell_widget_tree(&layout, &tokens);
        let mut model = WidgetInteractionModel::new();

        let tab_x = layout.content_tab_strip.x + 10.0;
        let tab_y = layout.content_tab_strip.y + 5.0;
        let _ = model.on_pointer_down(&mut tree, tab_x, tab_y, PointerButton::Primary);

        let mut fresh_tree = build_shell_widget_tree(&layout, &tokens);
        model.apply_to_tree(&mut fresh_tree);

        if let Some(idx) = model.pressed_widget_idx {
            let state = fresh_tree.widgets[idx].get_state();
            assert!(
                matches!(state, zaroxi_core_engine_ui::InteractionState::Active),
                "apply_to_tree should restore press state"
            );
        }
    }

    #[test]
    fn focusable_indices_are_deterministic() {
        let layout = ShellLayout::from_window_size(1200, 800);
        let tokens = test_tokens_dark();
        let tree = build_shell_widget_tree(&layout, &tokens);

        let focusables: Vec<usize> = tree
            .widgets
            .iter()
            .enumerate()
            .filter(|(_, w)| w.is_focusable())
            .map(|(i, _)| i)
            .collect();

        assert!(!focusables.is_empty(), "tree must have focusable widgets");
        assert!(
            focusables.windows(2).all(|w| w[0] < w[1]),
            "focusable indices must be in increasing tree order"
        );
    }
}
