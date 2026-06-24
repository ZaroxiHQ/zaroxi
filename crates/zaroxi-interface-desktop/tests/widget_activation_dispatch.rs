#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use zaroxi_core_engine_style::test_utils::test_tokens_dark;
    use zaroxi_core_engine_ui::{
        ShellLayout, WidgetAction, WidgetId, WidgetInteractionModel, build_shell_widget_tree,
    };
    use zaroxi_interface_desktop::gui::window::editor_buf::EditorBufferState;
    use zaroxi_interface_desktop::gui::window::{FrameScheduler, GuiApp, WidgetActivationHandler};
    use zaroxi_interface_desktop::gui::{ShellFrame, ShellWorkContent, Size};
    use zaroxi_interface_theme::theme::ZaroxiTheme;

    fn make_shell_frame() -> ShellFrame {
        ShellFrame::new(Size { width: 1200, height: 800 }, ZaroxiTheme::Dark)
    }

    fn make_test_app() -> GuiApp {
        let tokens = test_tokens_dark();
        let layout = ShellLayout::from_window_size(1200, 800);
        let (ai_tracer, ai_trace_rx) = zaroxi_application_ai::trace::AiTracer::channel();
        GuiApp {
            window_attributes: Default::default(),
            title: "test".into(),
            maybe_window: None,
            shell: make_shell_frame(),
            work_content: Some(ShellWorkContent::default()),
            requested_initial_frame: false,
            already_logged_existing: true,
            first_render_shown: true,
            widget_tree: Some(build_shell_widget_tree(&layout, &tokens, None)),
            interaction: WidgetInteractionModel::new(),
            editor_buffer: EditorBufferState::empty(),
            theme_mode: ZaroxiTheme::Dark,
            shift_held: false,
            ctrl_held: false,
            mem_monitor: zaroxi_core_telemetry::MemoryMonitor::from_env(),
            buffer_tracker: zaroxi_core_telemetry::BufferActivityTracker::new(),
            last_mem_sample: None,
            ai_tracer,
            ai_trace_rx: Some(ai_trace_rx),
            ai_session: zaroxi_application_ai::view_model::AiSessionState::default(),
            on_widget_activated: None,
            composition: None,
            workspace_view: None,
            workspace_service: None,
            session_id: None,
            workspace_id: None,
            folder_picker: None,
            explorer_actions: None,
            explorer_button_rect: None,
            parser_pool: std::sync::Arc::new(zaroxi_core_platform_syntax::parser::ParserPool::new()),
            cached_editor_data: None,
            cached_editor_lines_hash: 0,
            cached_editor_spans_version: 0,
            layout_controller:
                zaroxi_interface_desktop::gui::window::editor_shell::ShellLayoutController::new(),
            editor_viewport: None,
            needs_render: true,
            last_explorer_ids: Vec::new(),
            explorer_scroll_top: 0,
            explorer_search_active: false,
            explorer_search_rect: None,
            explorer_search_sel: None,
            explorer_caret_blink_epoch: std::time::Instant::now(),
            explorer_visible_rows: 1,
            last_render_size: (0, 0),
            pending_scroll_frac: 0.0,
            picker_in_flight: false,
            pending_picker_rx: None,
            last_widget_tree_size: (0, 0),
            last_widget_tree_fingerprint: None,
            render_core: None,
            cockpit_text_active: false,
            cockpit_rendered_once: false,
            last_open_started_at: None,
            last_focus_change_at: None,
            status_model_generation: 0,
            startup_first_paint_done: false,
            startup_first_paint_at: None,
            startup_second_layout_reason: None,
            cockpit_retained_bytes: 0,
            editor_retained_bytes: 0,
            cockpit_status_fingerprint: 0,
            line_syntax_cache: std::collections::HashMap::new(),
            cached_line_hashes: Vec::new(),
            large_file_mode: false,
            current_language: zaroxi_core_platform_syntax::language::LanguageId::PlainText,
            latest_spans: None,
            latest_spans_version: 0,
            cockpit_minimap_symbols: Vec::new(),
            cockpit_symbols_version: 0,
            git_diff_provider: zaroxi_core_platform_git::GitDiffProvider::new(),
            cockpit_diff_hunks: Vec::new(),
            cockpit_diff_version: 0,
            parse_worker: None,
            saved_buffer_version: 0,
            frame_scheduler: FrameScheduler::new(),
            ui_node_tracker: Default::default(),
            open_settling: false,
            open_burst_frames: 0,
            resize_pending: false,
            commit_deferred_open: false,
            commit_deferred_resize: false,
            open_token: 0,
            committed_open_token: 0,
            open_first_screenful_pending: false,
            pending_open: None,
            committed_active_file: None,
            file_switch_count: 0,
            visible_loading_state: false,
            open_request_at: None,
            last_upstream_open_prep_ms: 0.0,
            read_worker: None,
            read_token: 0,
            read_pending: false,
            read_started_at: None,
            read_generation: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            open_worker: None,
            background_open_pending: false,
            open_worker_started_at: None,
            open_present: None,
            open_atomic_first_paint: false,
            startup_geometry_initial: None,
            startup_geometry_final: None,
            startup_geometry_changed_reason: None,
            startup_first_visible_layout_stable: false,
            startup_settle_trimmed: false,
            text_instance_buffer_version: 0,
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
