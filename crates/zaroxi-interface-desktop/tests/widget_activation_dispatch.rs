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
            cmd_held: false,
            alt_held: false,
            terminal: Default::default(),
            bottom_tab: Default::default(),
            bottom_scroll: 0,
            problems: Vec::new(),
            parse_problems: Vec::new(),
            parse_problems_owner: None,
            output_log: Default::default(),
            mem_monitor: zaroxi_core_telemetry::MemoryMonitor::from_env(),
            buffer_tracker: zaroxi_core_telemetry::BufferActivityTracker::new(),
            last_mem_sample: None,
            ai_tracer,
            ai_trace_rx: Some(ai_trace_rx),
            ai_session: zaroxi_application_ai::view_model::AiSessionState::default(),
            ai_provider_status: None,
            ai_chat: zaroxi_application_ai::session_manager::SessionManager::new(),
            ai_composer_text: String::new(),
            ai_composer_focused: false,
            ai_mcp: zaroxi_application_ai::mcp_service::McpService::new(),
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
            editor_visual_to_logical: Vec::new(),
            editor_chars_per_row: 0,
            editor_wrap_visual_offset: 0,
            needs_render: true,
            last_explorer_ids: Vec::new(),
            explorer_scroll_top: 0,
            explorer_search_active: false,
            explorer_search_rect: None,
            explorer_search_sel: None,
            rail_selected_index: 0,
            rail_hovered_index: None,
            rail_item_hit_rects: Vec::new(),
            tab_state: zaroxi_interface_desktop::gui::window::destination::WorkbenchTabState::new(),
            sidebar_row_hit_rects: Vec::new(),
            tab_hit_rects: Vec::new(),
            tab_arrow_left_rect: None,
            tab_arrow_right_rect: None,
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
            latest_spans_owner: None,
            last_good_highlight: None,
            cockpit_minimap: zaroxi_interface_widgets::MinimapProjection::empty(),
            cockpit_minimap_key: (None, 0, 0),
            minimap_hit_rect: None,
            minimap_dragging: false,
            git_diff_provider: zaroxi_core_platform_git::GitDiffProvider::new(),
            cockpit_diff_hunks: Vec::new(),
            cockpit_diff_version: 0,
            parse_worker: None,
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
            cached_editor_active_file: None,
            content_generation: 0,
            active_rope_owner_path: None,
            owner_reload_attempted_for: None,
            pending_owner_rehydrate: false,
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
            settings: zaroxi_domain_settings::Settings::default(),
            settings_hit_rects: Vec::new(),
            settings_dropdown: zaroxi_interface_widgets::SettingsDropdownState::default(),
            cached_settings_popup: None,
            doc_buffers: std::collections::HashMap::new(),
            open_documents: std::collections::HashMap::new(),
            document_view_states: std::collections::HashMap::new(),
            restored_view_state_this_activation: false,
            activation_seq: 0,
            last_committed_activation_seq: 0,
            open_intent: None,
            editor_group: zaroxi_interface_desktop::gui::window::EditorGroup::default(),
            last_explorer_click: None,
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

    #[test]
    fn ai_setup_provider_button_opens_settings_destination() {
        use zaroxi_core_engine_ui::layout_constants as lc;
        use zaroxi_interface_desktop::gui::window::destination::{
            WorkbenchDestination, WorkbenchTabId,
        };

        let mut app = make_test_app();
        let result =
            app.dispatch_activation(&WidgetId::Button { index: lc::BTN_ID_AI_SETUP_PROVIDER });
        assert!(result.is_none(), "setup provider CTA returns no work content");
        assert_eq!(
            app.tab_state.active(),
            &WorkbenchTabId::DestinationRoot(WorkbenchDestination::Settings),
            "setup provider CTA must focus the Settings destination"
        );
    }

    #[test]
    fn ai_new_chat_button_resets_session_state() {
        use zaroxi_application_ai::view_model::AiPhase;
        use zaroxi_core_engine_ui::layout_constants as lc;

        let mut app = make_test_app();
        app.ai_session.phase = AiPhase::Streaming;
        app.ai_session.tokens_streamed = 42;

        let result = app.dispatch_activation(&WidgetId::Button { index: lc::BTN_ID_AI_NEW_CHAT });
        assert!(result.is_none(), "no composition wired, so no work content returned");
        assert_eq!(app.ai_session.phase, AiPhase::Idle, "new chat must reset session phase");
        assert_eq!(app.ai_session.tokens_streamed, 0);
    }

    #[test]
    fn ai_clear_button_resets_session_state() {
        use zaroxi_application_ai::view_model::AiPhase;
        use zaroxi_core_engine_ui::layout_constants as lc;

        let mut app = make_test_app();
        app.ai_session.phase = AiPhase::Complete;

        let _ = app.dispatch_activation(&WidgetId::Button { index: lc::BTN_ID_AI_CLEAR });
        assert_eq!(app.ai_session.phase, AiPhase::Idle, "clear must reset session phase");
    }

    #[test]
    fn ai_composer_click_focuses_and_other_clicks_blur() {
        let mut app = make_test_app();
        assert!(!app.ai_composer_focused);

        let _ = app.dispatch_activation(&WidgetId::TextInput { index: 0 });
        assert!(app.ai_composer_focused, "clicking the composer must focus it");

        let _ = app.dispatch_activation(&WidgetId::Tab { index: 0 });
        assert!(!app.ai_composer_focused, "clicking elsewhere must blur the composer");
    }

    #[test]
    fn ai_composer_typing_and_send_without_backend_reports_error() {
        use winit::keyboard::{Key, NamedKey};
        use zaroxi_interface_desktop::gui::window::ProviderUiStatus;

        let mut app = make_test_app();
        // Simulate a ready provider so the composer accepts input; the send
        // path must still surface a truthful error because no backend is wired.
        app.ai_provider_status =
            Some(ProviderUiStatus::Connected { provider: "OpenAI".into(), model: String::new() });
        let _ = app.dispatch_activation(&WidgetId::TextInput { index: 0 });

        app.press_key(&Key::Character("h".into()));
        app.press_key(&Key::Character("i".into()));
        assert_eq!(app.ai_composer_text, "hi");

        app.press_key(&Key::Named(NamedKey::Backspace));
        assert_eq!(app.ai_composer_text, "h");
        app.press_key(&Key::Character("ello".into()));
        app.press_key(&Key::Named(NamedKey::Enter));

        assert!(app.ai_composer_text.is_empty(), "send must clear the composer");
        let conv = app.ai_chat.active_conversation();
        assert_eq!(conv.messages[0].content, "hello");
        assert_eq!(
            conv.status,
            zaroxi_domain_ai::conversation::ConversationStatus::Error,
            "no backend wired: conversation must surface a truthful error"
        );
        assert!(conv.last_error.is_some());
    }

    #[test]
    fn ai_composer_escape_releases_focus() {
        use winit::keyboard::{Key, NamedKey};

        let mut app = make_test_app();
        let _ = app.dispatch_activation(&WidgetId::TextInput { index: 0 });
        assert!(app.ai_composer_focused);

        app.press_key(&Key::Named(NamedKey::Escape));
        assert!(!app.ai_composer_focused, "Escape must release composer focus");
    }

    #[test]
    fn ai_new_chat_archives_conversation_and_clears_composer() {
        use zaroxi_core_engine_ui::layout_constants as lc;

        let mut app = make_test_app();
        app.ai_chat.send_message("first question");
        app.ai_chat.finish_streaming();
        app.ai_composer_text = "draft".into();

        let _ = app.dispatch_activation(&WidgetId::Button { index: lc::BTN_ID_AI_NEW_CHAT });

        assert!(app.ai_chat.active_conversation().messages.is_empty());
        assert_eq!(app.ai_chat.history().len(), 1, "previous conversation must be archived");
        assert!(app.ai_composer_text.is_empty(), "new chat must clear the composer draft");
    }

    #[test]
    fn ai_send_prompt_ignores_empty_and_whitespace_input() {
        let mut app = make_test_app();
        app.ai_composer_text = "   ".into();
        app.ai_send_prompt();
        assert!(app.ai_chat.active_conversation().messages.is_empty());
    }

    #[test]
    fn ai_quick_action_records_user_message_and_truthful_error() {
        use zaroxi_core_engine_ui::layout_constants as lc;
        use zaroxi_interface_desktop::gui::window::ProviderUiStatus;

        let mut app = make_test_app();
        app.ai_provider_status =
            Some(ProviderUiStatus::Connected { provider: "OpenAI".into(), model: String::new() });

        let _ = app.dispatch_activation(&WidgetId::Button { index: lc::BTN_ID_AI_EXPLAIN });

        let conv = app.ai_chat.active_conversation();
        assert_eq!(conv.messages[0].content, "Explain the active file");
        assert_eq!(
            conv.status,
            zaroxi_domain_ai::conversation::ConversationStatus::Error,
            "no backend wired: quick action must surface a truthful error"
        );
    }

    #[test]
    fn ai_quick_action_is_gated_on_provider_readiness() {
        use zaroxi_core_engine_ui::layout_constants as lc;

        let mut app = make_test_app();
        // No provider override, no backend → not ready.
        let _ = app.dispatch_activation(&WidgetId::Button { index: lc::BTN_ID_AI_TESTS });
        assert!(
            app.ai_chat.active_conversation().messages.is_empty(),
            "quick actions must be inert when the provider is not ready"
        );
    }

    #[test]
    fn ai_apply_without_backend_reports_truthful_error() {
        use zaroxi_core_engine_ui::layout_constants as lc;

        let mut app = make_test_app();
        let _ = app.dispatch_activation(&WidgetId::Button { index: lc::BTN_ID_AI_APPLY });

        let conv = app.ai_chat.active_conversation();
        assert_eq!(conv.status, zaroxi_domain_ai::conversation::ConversationStatus::Error);
        assert!(conv.last_error.as_deref().unwrap_or("").contains("Cannot apply"));
    }

    #[test]
    fn ai_reject_records_decision_in_conversation() {
        use zaroxi_core_engine_ui::layout_constants as lc;

        let mut app = make_test_app();
        let _ = app.dispatch_activation(&WidgetId::Button { index: lc::BTN_ID_AI_REJECT });

        let conv = app.ai_chat.active_conversation();
        assert_eq!(conv.messages.last().unwrap().content, "Proposal rejected.");
    }
}
