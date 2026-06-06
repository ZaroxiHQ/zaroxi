/*!
GuiApp implementation and winit ApplicationHandler lifecycle methods.

Phase 57: slimmed to a thin winit-to-engine bridge; widget interaction
(hit-testing, hover, press, scrollbar drag, focus) now lives in
`zaroxi_core_engine_ui::WidgetInteractionModel`.

Phase 58: added keyboard focus traversal (Tab/Shift+Tab/Enter/Escape) and
`on_widget_activated` callback.

Phase 59: built-in `dispatch_activation` method that routes WidgetId to
DesktopComposition actions (set active buffer, window controls, etc.).
The callback remains as an override capability.
*/

use std::sync::Arc;

use pollster;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::ControlFlow,
    keyboard::{Key, NamedKey},
    window::WindowAttributes,
};

use crate::DesktopComposition;
use crate::gui::{ShellFrame, ShellWorkContent};
use zaroxi_application_workspace::ports::{SessionId, WorkspaceService, WorkspaceView};
use zaroxi_core_engine_ui::WidgetId;
use zaroxi_kernel_types::Id;

fn gui_debug(msg: &str) {
    if std::env::var("ZAROXI_DEBUG_GUI").as_deref() == Ok("1") {
        eprintln!("{}", msg);
    }
}

macro_rules! gui_debug_fmt {
    ($($arg:tt)*) => {
        if std::env::var("ZAROXI_DEBUG_GUI").as_deref() == Ok("1") {
            eprintln!($($arg)*);
        }
    };
}

pub type WidgetActivationHandler = Box<dyn FnMut(&WidgetId) -> Option<ShellWorkContent>>;

pub struct GuiApp {
    pub window_attributes: WindowAttributes,
    pub title: String,
    pub maybe_window: Option<zaroxi_core_engine_window::ZaroxiWindow>,
    pub shell: ShellFrame,
    pub work_content: Option<ShellWorkContent>,
    pub requested_initial_frame: bool,
    pub already_logged_existing: bool,
    pub first_render_shown: bool,
    pub widget_tree: Option<zaroxi_core_engine_ui::ShellWidgetTree>,
    pub interaction: zaroxi_core_engine_ui::WidgetInteractionModel,
    pub editor_cursor_line: usize,
    pub editor_cursor_col: usize,
    pub selection_anchor: Option<(usize, usize)>,
    pub selection_range: Option<(usize, usize, usize, usize)>,
    pub selection_active: bool,
    pub theme_mode: zaroxi_interface_theme::theme::ZaroxiTheme,
    pub shift_held: bool,
    pub ctrl_held: bool,
    /// Optional override handler for widget activation. When set, it is tried
    /// before the built-in `dispatch_activation` method.
    pub on_widget_activated: Option<WidgetActivationHandler>,
    /// DesktopComposition for domain activation dispatch (set by harness).
    pub composition: Option<DesktopComposition>,
    pub workspace_view: Option<Arc<dyn WorkspaceView>>,
    pub workspace_service: Option<Arc<dyn WorkspaceService>>,
    pub session_id: Option<SessionId>,
    pub workspace_id: Option<Id>,
}

impl GuiApp {
    /// Dispatch a WidgetId activation to DesktopComposition domain actions.
    /// Returns updated ShellWorkContent if the shell should refresh.
    pub fn dispatch_activation(&mut self, id: &WidgetId) -> Option<ShellWorkContent> {
        match id {
            WidgetId::Button { index: 2 } => {
                std::process::exit(0);
            }
            WidgetId::Button { index: 0 } => {
                if let Some(z) = self.maybe_window.as_ref() {
                    z.window().set_minimized(true);
                }
                return None;
            }
            WidgetId::Button { index: 1 } => {
                if let Some(z) = self.maybe_window.as_ref() {
                    let maximized = z.window().is_maximized();
                    z.window().set_maximized(!maximized);
                }
                return None;
            }
            _ => {}
        }

        let comp = self.composition.as_mut()?;
        let view = self.workspace_view.as_ref()?;
        let service = self.workspace_service.as_ref()?;
        let session = self.session_id.clone()?;

        match id {
            WidgetId::Tab { index } => {
                let items = comp.latest_opened_buffers_summary().items;
                let entry = items.get(*index)?;
                let buffer_id = entry.buffer_id.clone();

                let result =
                    pollster::block_on(crate::actions::set_active_buffer_and_get_shell_context(
                        comp,
                        service.clone(),
                        view.clone(),
                        session,
                        self.workspace_id,
                        buffer_id,
                    ));
                result.ok().map(|_| comp.build_work_content())
            }
            WidgetId::PanelAction { header_id, action } => {
                match (*header_id, *action) {
                    ("ai_assistant", "close") => {
                        pollster::block_on(crate::actions::close_command_bar(comp)).ok();
                    }
                    ("terminal", "close") => {
                        // Toggle bottom panel — delegated to composition status
                    }
                    _ => {}
                }
                Some(comp.build_work_content())
            }
            WidgetId::ListItem { index } => {
                // Rail activation: switch active panel / open command
                match index {
                    0 => { /* Explorer — toggle sidebar */ }
                    1 => { /* Search */ }
                    2 => { /* Source Ctrl */ }
                    3 => { /* Debug */ }
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }

    /// Extract selected text from editor_body lines using the live selection range.
    fn copy_selected_text(&self) -> Option<String> {
        let (sl, sc, el, ec) = self.selection_range?;
        let wc = self.work_content.as_ref()?;
        let body = wc.editor_body.as_ref()?;
        if body.lines.is_empty() {
            return None;
        }
        let mut selected = String::new();
        for line_idx in sl..=el {
            if line_idx >= body.lines.len() {
                break;
            }
            let line = &body.lines[line_idx];
            let start = if line_idx == sl { sc } else { 0 };
            let end = if line_idx == el { ec.min(line.len()) } else { line.len() };
            if start < end && start <= line.len() {
                selected.push_str(&line[start..end.min(line.len())]);
            }
            if line_idx < el {
                selected.push('\n');
            }
        }
        if selected.is_empty() { None } else { Some(selected) }
    }

    /// Dispatch engine-emitted widget actions into app-specific effects.
    pub fn handle_actions(&mut self, actions: Vec<zaroxi_core_engine_ui::WidgetAction>) {
        let mut needs_redraw = false;
        for action in actions {
            match action {
                zaroxi_core_engine_ui::WidgetAction::StateNeedsRedraw => {
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::FocusChanged(_) => {
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::ScrollOffsetChanged(id, offset) => {
                    self.interaction.set_scroll_offset(&id, offset);
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::Activated(ref id) => {
                    let content = self
                        .on_widget_activated
                        .as_mut()
                        .and_then(|handler| handler(id))
                        .or_else(|| self.dispatch_activation(id));

                    if let Some(wc) = content {
                        self.work_content = Some(wc);
                    }
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::HoverChanged(_)
                | zaroxi_core_engine_ui::WidgetAction::Nothing => {}
            }
        }
        if needs_redraw {
            if let Some(z) = self.maybe_window.as_ref() {
                let _ = z.window().request_redraw();
            }
        }
    }
}

impl winit::application::ApplicationHandler for GuiApp {
    fn new_events(&mut self, active_loop: &winit::event_loop::ActiveEventLoop, cause: StartCause) {
        if self.maybe_window.is_none() && matches!(cause, StartCause::Init) {
            gui_debug("GuiApp: attempting to create window (StartCause::Init)");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    gui_debug_fmt!("GuiApp: created engine window id={:?}", wid);
                    zaroxi_w.window().set_title(&self.title);
                    let _ = zaroxi_w.window().set_outer_position(PhysicalPosition::new(100, 100));
                    self.maybe_window = Some(zaroxi_w);

                    if let Some(z) = self.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                    active_loop.set_control_flow(ControlFlow::Wait);
                    gui_debug("GuiApp: window created (hidden); initial redraw requested");
                }
                Err(e) => {
                    eprintln!("GuiApp: failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else if self.maybe_window.is_some() {
            if !self.already_logged_existing {
                gui_debug("GuiApp: new_events called but window already created");
                self.already_logged_existing = true;
            }
        } else {
            gui_debug_fmt!("GuiApp: new_events called with cause={:?} (no creation)", cause);
        }
    }

    fn resumed(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        if self.maybe_window.is_none() {
            gui_debug("GuiApp: resumed -> attempting to create window");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    gui_debug_fmt!("GuiApp: created engine window on resumed id={:?}", wid);
                    self.maybe_window = Some(zaroxi_w);

                    if let Some(z) = self.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                    gui_debug(
                        "GuiApp: window created on resumed (hidden); initial redraw requested",
                    );
                }
                Err(e) => {
                    eprintln!("GuiApp: resumed failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else {
            gui_debug("GuiApp: resumed called but window already created");
        }
    }

    fn about_to_wait(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        if self.requested_initial_frame {
            if let Some(z) = self.maybe_window.as_ref() {
                gui_debug("GuiApp: about_to_wait -> requesting initial redraw (engine window)");
                let _ = z.window().request_redraw();
            }
            self.requested_initial_frame = false;
            active_loop.set_control_flow(ControlFlow::Wait);
            gui_debug("GuiApp: about_to_wait -> switched control flow back to Wait");
        }
    }

    fn window_event(
        &mut self,
        active_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                active_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(z) = self.maybe_window.as_mut() {
                    z.update_size(size.width, size.height);
                    gui_debug_fmt!(
                        "GuiApp: Resized -> {size:?}, requesting redraw (engine window)"
                    );
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(z) = self.maybe_window.as_ref() {
                    gui_debug("GuiApp: ScaleFactorChanged -> requesting redraw (engine window)");
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(ref mut tree) = self.widget_tree {
                    let actions = self.interaction.on_pointer_moved(
                        tree,
                        position.x as f32,
                        position.y as f32,
                    );
                    self.handle_actions(actions);

                    // Drag-selection: extend selection range while mouse is held
                    if self.selection_active {
                        if let Some(anchor) = self.selection_anchor {
                            if let Some((line, col)) = project_editor_cursor(
                                position,
                                &self.shell.regions,
                                &self.shell.work_content,
                                self.interaction.get_scroll_offset(
                                    &zaroxi_core_engine_ui::WidgetId::Scrollbar { index: 1 },
                                ),
                            ) {
                                let (sl, sc) =
                                    if line < anchor.0 || (line == anchor.0 && col < anchor.1) {
                                        (line, col)
                                    } else {
                                        anchor
                                    };
                                let (el, ec) =
                                    if (line, col) > anchor { (line, col) } else { anchor };
                                self.selection_range = Some((sl, sc, el, ec));
                                if let Some(z) = self.maybe_window.as_ref() {
                                    let _ = z.window().request_redraw();
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(ref mut tree) = self.widget_tree {
                    let actions = self.interaction.on_pointer_leave(tree);
                    self.handle_actions(actions);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    let (x, y) = match self.interaction.cursor_pos_f32() {
                        Some(pos) => pos,
                        None => return,
                    };
                    let actions = match state {
                        ElementState::Pressed => {
                            if let Some(ref mut tree) = self.widget_tree {
                                let actions = self.interaction.on_pointer_down(
                                    tree,
                                    x,
                                    y,
                                    zaroxi_core_engine_ui::PointerButton::Primary,
                                );
                                actions
                            } else {
                                Vec::new()
                            }
                        }
                        ElementState::Released => {
                            if let Some(ref mut tree) = self.widget_tree {
                                let actions = self.interaction.on_pointer_up(
                                    tree,
                                    x,
                                    y,
                                    zaroxi_core_engine_ui::PointerButton::Primary,
                                );
                                actions
                            } else {
                                Vec::new()
                            }
                        }
                    };
                    self.handle_actions(actions);

                    if let ElementState::Pressed = state {
                        if let Some(pos) = self.interaction.cursor_pos_f32() {
                            let phys = PhysicalPosition::new(pos.0 as f64, pos.1 as f64);
                            if let Some((line, col)) = project_editor_cursor(
                                phys,
                                &self.shell.regions,
                                &self.shell.work_content,
                                self.interaction.get_scroll_offset(
                                    &zaroxi_core_engine_ui::WidgetId::Scrollbar { index: 1 },
                                ),
                            ) {
                                self.editor_cursor_line = line;
                                self.editor_cursor_col = col;
                                self.selection_anchor = Some((line, col));
                                self.selection_active = true;
                                self.selection_range = None;
                                if let Some(z) = self.maybe_window.as_ref() {
                                    let _ = z.window().request_redraw();
                                }
                            }
                        }
                    }
                    if let ElementState::Released = state {
                        self.selection_active = false;
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(z) = self.maybe_window.as_mut() {
                    let _ = z.window().pre_present_notify();

                    let (sw, sh) = z.size();
                    if sw > 0 && sh > 0 {
                        let system_is_dark = z
                            .window()
                            .theme()
                            .map(|t| matches!(t, winit::window::Theme::Dark))
                            .unwrap_or(true);
                        let resolved = self.theme_mode.resolve(system_is_dark);
                        let actual = crate::gui::Size { width: sw, height: sh };
                        self.shell = crate::gui::ShellFrame::new(actual, resolved);
                    }

                    self.shell.work_content = self.work_content.clone();

                    let system_is_dark = z
                        .window()
                        .theme()
                        .map(|t| matches!(t, winit::window::Theme::Dark))
                        .unwrap_or(true);
                    let variant = self.theme_mode.resolve(system_is_dark);
                    let mut sem = variant.colors(false);

                    let debug_theme_active =
                        std::env::var("ZAROXI_DEBUG_THEME").as_deref() == Ok("1");
                    if debug_theme_active {
                        sem = zaroxi_interface_theme::theme::SemanticColors::debug();
                        gui_debug("ZAROXI_DEBUG_THEME: debug theme override ACTIVE");
                    }

                    if !self.first_render_shown && debug_theme_active {
                        gui_debug_fmt!(
                            "ZAROXI_THEME_TRACE: mode={:?} system_is_dark={} resolved={:?}",
                            self.theme_mode,
                            system_is_dark,
                            variant
                        );
                        gui_debug_fmt!(
                            "ZAROXI_THEME_TRACE: sem.shell_background={:?} sem.app_background={:?} sem.editor_background={:?}",
                            sem.shell_background,
                            sem.app_background,
                            sem.editor_background
                        );
                    }

                    let tokens = super::style_tokens_adapter::resolve_style_tokens(
                        &sem,
                        &Default::default(),
                    );

                    if !self.first_render_shown && debug_theme_active {
                        gui_debug_fmt!(
                            "ZAROXI_STYLE_TOKENS: app_bg={:?} titlebar_bg={:?} editor_bg={:?} sidebar_bg={:?}",
                            tokens.app_background.to_array(),
                            tokens.titlebar_background.to_array(),
                            tokens.editor_content_background.to_array(),
                            tokens.sidebar_background.to_array(),
                        );
                    }

                    let layout = zaroxi_core_engine_ui::ShellLayout::from_window_size(sw, sh);
                    let mut widget_tree = zaroxi_core_engine_ui::build_shell_widget_tree(
                        &layout,
                        &tokens,
                        self.work_content.as_ref(),
                    );
                    self.interaction.apply_to_tree(&mut widget_tree);
                    self.interaction.apply_scroll_offsets(&mut widget_tree);
                    self.widget_tree = Some(widget_tree.clone());

                    let render_layout =
                        super::renderbridge::build_render_layout(&self.shell.regions, &tokens);

                    let editor_data =
                        super::presenters::shape_editor_content(&self.shell.work_content, &sem);
                    let explorer_data =
                        super::presenters::shape_explorer_content(&self.shell.work_content);
                    let ai_data = super::presenters::shape_ai_content(&self.shell.work_content);
                    let status_data = super::presenters::shape_status_content(
                        &self.shell.work_content,
                        self.editor_cursor_line,
                        self.editor_cursor_col,
                    );

                    let ctx = super::frame::ShellBlockContext {
                        editor_data,
                        explorer_data,
                        status_bar_data: status_data,
                        ai_data,
                        terminal_tabs: self
                            .work_content
                            .as_ref()
                            .and_then(|wc| wc.terminal_tabs.clone()),
                    };

                    let mut render_blocks: Vec<zaroxi_core_engine_render::UiBlock> =
                        super::frame::compose_blocks(&self.shell.regions, &tokens, &ctx);

                    // Compute scrollbar blocks from ShellFrame regions for
                    // correct spatial placement (the widget tree uses a
                    // different layout system that disagrees on panel widths).
                    let editor_total_lines = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|wc| wc.editor_body.as_ref())
                        .map(|cv| cv.lines.len())
                        .unwrap_or(0);
                    let line_h = 16.0f32;
                    let content_pad = 8.0f32;
                    let header_h = 28.0f32;
                    let editor_region = crate::gui::region_dispatch::find_region_by_role(
                        &self.shell.regions,
                        zaroxi_core_engine_style::PanelRole::ContentArea,
                    );
                    let editor_visible_lines = editor_region
                        .map(|r| {
                            let usable_h = r.rect.height as f32 - header_h - content_pad * 2.0;
                            (usable_h / line_h).max(1.0) as usize
                        })
                        .unwrap_or(1);
                    let scroll_blocks = super::frame::compute_scrollbar_blocks(
                        &self.shell.regions,
                        &tokens,
                        editor_total_lines,
                        editor_visible_lines,
                    );
                    render_blocks.extend(scroll_blocks);

                    // Apply live editor cursor and selection to the ContentArea block
                    for block in &mut render_blocks {
                        if block.id.contains("ContentArea") || block.id.contains("content_area") {
                            block.cursor_line = Some(self.editor_cursor_line);
                            block.cursor_col = Some(self.editor_cursor_col);
                            block.selection_range = self.selection_range;
                        }
                    }

                    match pollster::block_on(zaroxi_core_engine_render::Renderer::new(
                        z.window(),
                        [
                            tokens.app_background.r as f64,
                            tokens.app_background.g as f64,
                            tokens.app_background.b as f64,
                            1.0,
                        ],
                    )) {
                        Ok(mut renderer) => {
                            let app_state = zaroxi_core_engine_render::renderer::core::AppState;
                            match renderer.render_with_layout(
                                &app_state,
                                &render_layout,
                                &render_blocks,
                            ) {
                                Ok(()) => {
                                    if !self.first_render_shown {
                                        let _ = z.window().set_visible(true);
                                        let _ = z.window().pre_present_notify();
                                        self.first_render_shown = true;
                                        eprintln!(
                                            "GuiApp: first full-renderer frame; window visible"
                                        );
                                    }
                                }
                                Err(e) => {
                                    eprintln!("GuiApp: render_with_layout failed: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("GuiApp: failed to create renderer: {:?}", e);
                        }
                    }

                    if std::env::var("ZAROXI_DEBUG_RENDER").as_deref() == Ok("1") {
                        eprintln!(
                            "GUI_TEXT_FRAME_SUMMARY: surface={}x{} render_blocks={}",
                            sw,
                            sh,
                            render_blocks.len()
                        );
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
                self.ctrl_held = modifiers.state().control_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let actions = match event.logical_key {
                    Key::Named(NamedKey::Tab) => {
                        if let Some(ref mut tree) = self.widget_tree {
                            if self.shift_held {
                                self.interaction.focus_previous(tree)
                            } else {
                                self.interaction.focus_next(tree)
                            }
                        } else {
                            Vec::new()
                        }
                    }
                    Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) => {
                        if let Some(ref mut tree) = self.widget_tree {
                            self.interaction.activate_focused(tree)
                        } else {
                            Vec::new()
                        }
                    }
                    Key::Named(NamedKey::Escape) => {
                        if let Some(ref mut tree) = self.widget_tree {
                            if let Some(old) = self.interaction.focused_widget_idx {
                                tree.set_state_at(
                                    old,
                                    zaroxi_core_engine_ui::InteractionState::Normal,
                                );
                            }
                            self.interaction.focused_widget_idx = None;
                            vec![
                                zaroxi_core_engine_ui::WidgetAction::FocusChanged(None),
                                zaroxi_core_engine_ui::WidgetAction::StateNeedsRedraw,
                            ]
                        } else {
                            Vec::new()
                        }
                    }
                    ref key if self.ctrl_held => match key {
                        Key::Character(c) if c == "c" => {
                            if let Some(text) = self.copy_selected_text() {
                                let _ = zaroxi_core_engine_clipboard::copy_text(&text);
                            }
                            Vec::new()
                        }
                        Key::Character(c) if c == "x" => {
                            if let Some(text) = self.copy_selected_text() {
                                let _ = zaroxi_core_engine_clipboard::copy_text(&text);
                            }
                            Vec::new()
                        }
                        Key::Character(c) if c == "v" => {
                            match zaroxi_core_engine_clipboard::get_text() {
                                Ok(text) => {
                                    gui_debug_fmt!(
                                        "ZAROXI_CLIPBOARD: paste at line={} col={} len={}",
                                        self.editor_cursor_line,
                                        self.editor_cursor_col,
                                        text.len()
                                    );
                                }
                                Err(e) => {
                                    eprintln!("ZAROXI_CLIPBOARD: paste failed: {}", e);
                                }
                            }
                            Vec::new()
                        }
                        Key::Character(c) if c == "z" => {
                            gui_debug_fmt!(
                                "ZAROXI_UNDO: undo at cursor line={} col={}",
                                self.editor_cursor_line,
                                self.editor_cursor_col
                            );
                            Vec::new()
                        }
                        Key::Character(c) if c == "y" => {
                            gui_debug_fmt!(
                                "ZAROXI_REDO: redo at cursor line={} col={}",
                                self.editor_cursor_line,
                                self.editor_cursor_col
                            );
                            Vec::new()
                        }
                        _ => Vec::new(),
                    },
                    _ => Vec::new(),
                };
                self.handle_actions(actions);
            }
            _ => {}
        }
    }
}

fn project_editor_cursor(
    cursor_pos: winit::dpi::PhysicalPosition<f64>,
    regions: &[crate::gui::ShellRegion],
    work_content: &Option<crate::gui::ShellWorkContent>,
    editor_scroll_offset: f32,
) -> Option<(usize, usize)> {
    let editor_region = crate::gui::region_dispatch::find_region_by_role(
        regions,
        zaroxi_core_engine_style::PanelRole::ContentArea,
    )?;

    let ex = editor_region.rect.x as f32;
    let ey = editor_region.rect.y as f32;
    let px = cursor_pos.x as f32;
    let py = cursor_pos.y as f32;

    if px < ex
        || py < ey
        || px >= ex + editor_region.rect.width as f32
        || py >= ey + editor_region.rect.height as f32
    {
        return None;
    }

    let dt = zaroxi_interface_theme::theme::DesignTokens::default();
    let content_pad = dt.spacing_sm; // 8.0
    let header_h = dt.spacing_md + dt.spacing_lg; // 28.0
    let line_h = dt.font_size_md + 2.0; // 16.0
    let char_w = dt.font_size_sm / 1.5; // 8.0
    let content_x = ex + content_pad;
    let content_y = ey + header_h + content_pad;
    let rel_y = py - content_y;
    let rel_x = px - content_x;
    let visible_line = (rel_y / line_h).max(0.0) as usize;
    let col = (rel_x / char_w).max(0.0) as usize;

    let usable_h = editor_region.rect.height as f32 - header_h - content_pad * 2.0;

    let total_lines = work_content
        .as_ref()
        .and_then(|w| w.editor_body.as_ref())
        .map(|cv| cv.lines.len().max(1))
        .unwrap_or(1);
    let visible_lines_c = (usable_h / line_h).max(1.0) as usize;
    let max_scroll_c = (total_lines.saturating_sub(visible_lines_c)).max(1);
    let first_visible = (editor_scroll_offset * max_scroll_c as f32) as usize;
    let absolute_line = first_visible + visible_line;

    Some((absolute_line, col))
}
