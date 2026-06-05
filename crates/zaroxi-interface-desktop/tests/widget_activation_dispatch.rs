#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use zaroxi_core_engine_style::test_utils::test_tokens_dark;
    use zaroxi_core_engine_ui::{
        ShellLayout, WidgetAction, WidgetId, WidgetInteractionModel, build_shell_widget_tree,
    };
    use zaroxi_interface_desktop::gui::window::{GuiApp, WidgetActivationHandler};
    use zaroxi_interface_desktop::gui::{ShellFrame, ShellWorkContent, Size};
    use zaroxi_interface_theme::theme::ZaroxiTheme;

    fn make_shell_frame() -> ShellFrame {
        ShellFrame::new(Size { width: 1200, height: 800 }, ZaroxiTheme::Dark)
    }

    fn make_test_app() -> GuiApp {
        let tokens = test_tokens_dark();
        let layout = ShellLayout::from_window_size(1200, 800);
        GuiApp {
            window_attributes: Default::default(),
            title: "test".into(),
            maybe_window: None,
            shell: make_shell_frame(),
            work_content: Some(ShellWorkContent::default()),
            requested_initial_frame: false,
            already_logged_existing: true,
            first_render_shown: true,
            widget_tree: Some(build_shell_widget_tree(&layout, &tokens)),
            interaction: WidgetInteractionModel::new(),
            editor_cursor_line: 0,
            editor_cursor_col: 0,
            selection_anchor: None,
            theme_mode: ZaroxiTheme::Dark,
            shift_held: false,
            on_widget_activated: None,
        }
    }

    #[test]
    fn activation_handler_is_called_on_activated_action() {
        let mut app = make_test_app();

        let called_with = Rc::new(RefCell::new(None));
        let cw = Rc::clone(&called_with);
        let handler: WidgetActivationHandler = Box::new(move |id| {
            *cw.borrow_mut() = Some(id.clone());
            None
        });
        app.on_widget_activated = Some(handler);

        let tab_id = WidgetId::Tab { index: 0 };
        app.handle_actions(vec![WidgetAction::Activated(tab_id.clone())]);

        assert!(called_with.borrow().is_some(), "activation handler should be called");
        assert_eq!(*called_with.borrow(), Some(tab_id));
    }

    #[test]
    fn activation_handler_sets_work_content_when_some_returned() {
        let mut app = make_test_app();

        let return_wc = ShellWorkContent {
            explorer_items: Some(vec!["test-item".into()]),
            ..ShellWorkContent::default()
        };
        let ren = return_wc.clone();
        let handler: WidgetActivationHandler = Box::new(move |_id| Some(ren.clone()));
        app.on_widget_activated = Some(handler);

        app.handle_actions(vec![WidgetAction::Activated(WidgetId::Tab { index: 0 })]);
        assert!(app.work_content.is_some());
        let wc = app.work_content.as_ref().unwrap();
        assert_eq!(wc.explorer_items, Some(vec!["test-item".into()]));
    }

    #[test]
    fn activation_handler_returning_none_does_not_change_work_content() {
        let mut app = make_test_app();

        let handler: WidgetActivationHandler = Box::new(move |_id| None);
        app.on_widget_activated = Some(handler);

        app.handle_actions(vec![WidgetAction::Activated(WidgetId::ListItem { index: 5 })]);
        assert!(app.work_content.is_some());
        assert!(app.work_content.as_ref().unwrap().explorer_items.is_none());
    }

    #[test]
    fn hover_changed_does_not_call_activation_handler() {
        let mut app = make_test_app();

        let called = Rc::new(RefCell::new(false));
        let ca = Rc::clone(&called);
        let handler: WidgetActivationHandler = Box::new(move |_id| {
            *ca.borrow_mut() = true;
            None
        });
        app.on_widget_activated = Some(handler);

        app.handle_actions(vec![WidgetAction::HoverChanged(Some(WidgetId::Tab { index: 0 }))]);
        assert!(!*called.borrow(), "hover should not trigger activation handler");
    }

    #[test]
    fn scroll_offset_roundtrips_through_interaction_model() {
        let mut app = make_test_app();

        let scroll_id = WidgetId::Scrollbar { index: 0 };
        app.interaction.set_scroll_offset(&scroll_id, 0.3);
        assert!((app.interaction.get_scroll_offset(&scroll_id) - 0.3).abs() < 0.001);

        app.handle_actions(vec![WidgetAction::ScrollOffsetChanged(scroll_id.clone(), 0.8)]);
        assert!((app.interaction.get_scroll_offset(&scroll_id) - 0.8).abs() < 0.001);
    }

    #[test]
    fn focus_changed_action_does_not_panic() {
        let mut app = make_test_app();
        app.handle_actions(vec![WidgetAction::FocusChanged(Some(WidgetId::Tab { index: 0 }))]);
    }
}
