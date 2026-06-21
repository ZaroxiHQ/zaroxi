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

    let window_attributes = WindowAttributes::default()
        .with_title("Zaroxi - GUI Shell")
        .with_inner_size(PhysicalSize::new(shell.size.width, shell.size.height))
        .with_resizable(true);

    let title = format!("Zaroxi - GUI Shell ({}x{})", shell.size.width, shell.size.height);

    let explorer_actions =
        folder_picker.as_ref().map(|fp| ExplorerPanelActions::new(Some(fp.clone())));

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
        shift_held: false,
        ctrl_held: false,
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
        layout_controller: super::editor_shell::ShellLayoutController::new(),
        editor_viewport: None,
        needs_render: true,
        last_explorer_ids: Vec::new(),
        last_render_size: (0, 0),
        pending_scroll_frac: 0.0,
        picker_in_flight: false,
        pending_picker_rx: None,
        last_widget_tree_size: (0, 0),
        last_widget_tree_fingerprint: None,
        render_core: None,
        line_syntax_cache: std::collections::HashMap::new(),
        cached_line_hashes: Vec::new(),
        large_file_mode: false,
        current_language: zaroxi_core_platform_syntax::language::LanguageId::PlainText,
        latest_spans: None,
        latest_spans_version: 0,
        parse_worker: None,
        saved_buffer_version: 0,
        frame_scheduler: super::app::FrameScheduler::new(),
    };

    let run_result = event_loop.run_app(&mut app);

    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
