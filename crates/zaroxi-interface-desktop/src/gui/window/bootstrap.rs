/*!
Bootstrap and public runner for the GUI window.
This file contains run_shell_window which creates the EventLoop, attributes,
instantiates the GuiApp and hands it to run_app.

Phase 59: accepts optional DesktopComposition + service handles so widget
activations dispatch to real domain behavior inside the event loop.
*/

use std::sync::Arc;

use crate::DesktopComposition;
use crate::folder_picker::DynFolderPicker;
use crate::gui::ShellFrame;
use crate::gui::ShellWorkContent;
use crate::gui::window::editor_buf::EditorBufferState;
use crate::gui::window::explorer_panel::ExplorerPanelActions;
use std::error::Error;
use winit::{dpi::PhysicalSize, event_loop::EventLoop, window::WindowAttributes};
use zaroxi_application_workspace::ports::{SessionId, WorkspaceService, WorkspaceView};
use zaroxi_kernel_types::Id;

/// Public runner: open a native window and run a basic winit event loop.
///
/// When `composition` is `Some`, the activation handler will dispatch
/// `WidgetAction::Activated` events to DesktopComposition actions using
/// the provided service handles and session/workspace ids.
pub fn run_shell_window(
    shell: ShellFrame,
    work_content: Option<ShellWorkContent>,
    composition: Option<DesktopComposition>,
    workspace_view: Option<Arc<dyn WorkspaceView>>,
    workspace_service: Option<Arc<dyn WorkspaceService>>,
    session_id: Option<SessionId>,
    workspace_id: Option<Id>,
    folder_picker: Option<DynFolderPicker>,
) -> Result<(), Box<dyn Error>> {
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(err) => {
            eprintln!("EventLoop::new() failed: {}. Falling back to transcript mode.", err);
            return Err(Box::new(err));
        }
    };

    if zaroxi_core_telemetry::startup_trace_enabled() {
        eprintln!("MEM_STARTUP: after_winit_init rss={:.1}MB", zaroxi_core_telemetry::rss_mb());
    }

    let window_attributes = WindowAttributes::default()
        .with_title("Zaroxi - GUI Shell")
        .with_inner_size(PhysicalSize::new(shell.size.width, shell.size.height))
        .with_resizable(true)
        .with_min_inner_size(PhysicalSize::new(480, 320));

    let title = format!("Zaroxi - GUI Shell ({}x{})", shell.size.width, shell.size.height);

    let explorer_actions =
        folder_picker.as_ref().map(|fp| ExplorerPanelActions::new(Some(fp.clone())));

    // Non-blocking AI trace channel: the tracer is handed to async AI tasks and
    // the receiver is drained once per frame by the render loop.
    let (ai_tracer, ai_trace_rx) = zaroxi_application_ai::trace::AiTracer::channel();

    let mut app = super::app::GuiApp {
        window_attributes: window_attributes.clone(),
        title,
        maybe_window: None,
        shell: shell.clone(),
        work_content: work_content.clone(),
        requested_initial_frame: false,
        already_logged_existing: false,
        first_render_shown: false,
        widget_tree: None,
        interaction: zaroxi_core_engine_ui::WidgetInteractionModel::new(),
        editor_buffer: EditorBufferState::empty(),
        theme_mode: zaroxi_interface_theme::theme::ZaroxiTheme::System,
        settings: zaroxi_domain_settings::Settings::default(),
        settings_hit_rects: Vec::new(),
        settings_dropdown: zaroxi_interface_widgets::SettingsDropdownState::default(),
        cached_settings_popup: None,
        shift_held: false,
        ctrl_held: false,
        cmd_held: false,
        mem_monitor: zaroxi_core_telemetry::MemoryMonitor::from_env(),
        buffer_tracker: zaroxi_core_telemetry::BufferActivityTracker::new(),
        last_mem_sample: None,
        ai_tracer,
        ai_trace_rx: Some(ai_trace_rx),
        ai_session: zaroxi_application_ai::view_model::AiSessionState::default(),
        on_widget_activated: None,
        composition,
        workspace_view,
        workspace_service,
        session_id,
        workspace_id,
        folder_picker,
        explorer_actions,
        explorer_button_rect: None,
        parser_pool: Arc::new(zaroxi_core_platform_syntax::parser::ParserPool::new()),
        cached_editor_data: None,
        cached_editor_lines_hash: 0,
        cached_editor_spans_version: 0,
        cached_editor_active_file: None,
        active_rope_owner_path: None,
        owner_reload_attempted_for: None,
        pending_owner_rehydrate: false,
        layout_controller: super::editor_shell::ShellLayoutController::new(),
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
        tab_state: super::destination::WorkbenchTabState::new(),
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
        open_intent: None,
        editor_group: crate::gui::window::EditorGroup::default(),
        last_explorer_click: None,
        cockpit_minimap_symbols: Vec::new(),
        cockpit_symbols_version: 0,
        git_diff_provider: zaroxi_core_platform_git::GitDiffProvider::new(),
        cockpit_diff_hunks: Vec::new(),
        cockpit_diff_version: 0,
        parse_worker: None,
        doc_buffers: std::collections::HashMap::new(),
        open_documents: std::collections::HashMap::new(),
        document_view_states: std::collections::HashMap::new(),
        restored_view_state_this_activation: false,
        activation_seq: 0,
        last_committed_activation_seq: 0,
        frame_scheduler: super::app::FrameScheduler::new(),
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
    };

    if zaroxi_core_telemetry::startup_trace_enabled() {
        eprintln!("MEM_STARTUP: after_app_init rss={:.1}MB", zaroxi_core_telemetry::rss_mb());
    }

    let run_result = event_loop.run_app(&mut app);

    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
