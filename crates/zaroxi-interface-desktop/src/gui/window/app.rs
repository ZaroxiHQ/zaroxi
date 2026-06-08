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

Editor Phase 1: extracted editor shell layout/rendering into
`editor_shell` module. `GuiApp` now delegates region layout to
`ShellLayoutController` (Taffy-based) and uses `EditorViewport`
for strict clipping boundaries.
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
use crate::folder_picker::DynFolderPicker;
use crate::gui::window::editor_shell::{EditorViewport, ShellLayoutController};
use crate::gui::window::explorer_panel::ExplorerPanelActions;
use crate::gui::{ShellFrame, ShellWorkContent};
use zaroxi_application_workspace::ports::{SessionId, WorkspaceService, WorkspaceView};
use zaroxi_core_engine_ui::WidgetId;
use zaroxi_core_engine_ui::layout_constants as lc;
use zaroxi_core_platform_syntax::parser::ParserPool;
use zaroxi_kernel_types::Id;

fn gui_debug(msg: &str) {
    if std::env::var("ZAROXI_DEBUG_GUI").as_deref() == Ok("1") {
        eprintln!("{}", msg);
    }
}

fn event_label(event: &winit::event::WindowEvent) -> String {
    use winit::event::WindowEvent;
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            format!("CursorMoved({:.0},{:.0})", position.x, position.y)
        }
        WindowEvent::MouseInput { state, button, .. } => {
            format!("MouseInput({:?},{:?})", state, button)
        }
        WindowEvent::MouseWheel { .. } => "MouseWheel".into(),
        WindowEvent::RedrawRequested => "RedrawRequested".into(),
        WindowEvent::Resized(s) => format!("Resized({}x{})", s.width, s.height),
        WindowEvent::ScaleFactorChanged { .. } => "ScaleFactorChanged".into(),
        WindowEvent::CursorEntered { .. } => "CursorEntered".into(),
        WindowEvent::CursorLeft { .. } => "CursorLeft".into(),
        WindowEvent::Focused(f) => format!("Focused({})", f),
        WindowEvent::CloseRequested => "CloseRequested".into(),
        WindowEvent::ModifiersChanged(_) => "ModifiersChanged".into(),
        WindowEvent::Occluded(b) => format!("Occluded({})", b),
        WindowEvent::ThemeChanged(_) => "ThemeChanged".into(),
        WindowEvent::Touch(_) => "Touch".into(),
        WindowEvent::PinchGesture { .. } => "PinchGesture".into(),
        other => format!("other({})", variant_name(other)),
    }
}

fn variant_name(_ev: &winit::event::WindowEvent) -> &'static str {
    // Quick discriminator for unknown variants
    "unknown"
}

fn click_trace(msg: &str) {
    if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
        eprintln!("{}", msg);
    }
}

macro_rules! click_trace_fmt {
    ($($arg:tt)*) => {
        if std::env::var("ZAROXI_DEBUG_CLICK").as_deref() == Ok("1") {
            eprintln!($($arg)*);
        }
    };
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
    pub folder_picker: Option<DynFolderPicker>,
    pub explorer_actions: Option<ExplorerPanelActions>,
    /// Explorer CTA button hit rect in window coordinates (x, y, w, h).
    /// Computed on each redraw from the same formula used in build_sidebar_block.
    pub explorer_button_rect: Option<(f32, f32, f32, f32)>,
    /// Shared parser pool for syntax highlighting.
    pub parser_pool: ParserPool,
    /// Taffy-based layout controller (Editor Phase 1).
    /// Owns layout computation, caching, and resize detection.
    pub layout_controller: ShellLayoutController,
    /// Current editor viewport for clipping (Editor Phase 1).
    pub editor_viewport: Option<EditorViewport>,
    /// Redraw gate: when false, RedrawRequested is a no-op.
    /// Set to true on content change or resize; cleared after a successful render.
    pub needs_render: bool,
    pub last_explorer_ids: Vec<String>,
    /// Last rendered window size (used to detect resize-then-rerender).
    pub last_render_size: (u32, u32),
}

impl GuiApp {
    /// Dispatch a WidgetId activation to DesktopComposition domain actions.
    /// Returns updated ShellWorkContent if the shell should refresh.
    pub fn dispatch_activation(&mut self, id: &WidgetId) -> Option<ShellWorkContent> {
        match id {
            WidgetId::Button { index: lc::BTN_ID_CLOSE_WINDOW } => {
                std::process::exit(0);
            }
            WidgetId::Button { index: lc::BTN_ID_MINIMIZE } => {
                if let Some(z) = self.maybe_window.as_ref() {
                    z.window().set_minimized(true);
                }
                return None;
            }
            WidgetId::Button { index: lc::BTN_ID_MAXIMIZE } => {
                if let Some(z) = self.maybe_window.as_ref() {
                    let maximized = z.window().is_maximized();
                    z.window().set_maximized(!maximized);
                }
                return None;
            }
            WidgetId::Button { index: lc::BTN_ID_EXPLORER_CTA } => {
                click_trace("ZAROXI_CLICK: dispatch_activation matched Explorer CTA");
                if let Some(ref mut actions) = self.explorer_actions {
                    click_trace(
                        "ZAROXI_CLICK: explorer_actions is Some, calling open_workspace_from_picker",
                    );
                    let comp = self.composition.as_mut()?;
                    let service = self.workspace_service.clone()?;
                    let view = self.workspace_view.clone()?;
                    return actions.open_workspace_from_picker(
                        comp,
                        service,
                        view,
                        &mut self.session_id,
                        &mut self.workspace_id,
                    );
                }
                click_trace("ZAROXI_CLICK: explorer_actions is None, cannot open workspace");
                return None;
            }
            _ => {}
        }

        let comp = self.composition.as_mut()?;
        let view = self.workspace_view.as_ref()?;
        let service = self.workspace_service.as_ref()?;
        let session = self.session_id.clone()?;

        match id {
            WidgetId::Button { index: lc::BTN_ID_TERMINAL_CLOSE } => {
                // Terminal panel close — just refresh
                Some(comp.build_work_content())
            }
            WidgetId::Button { index: lc::BTN_ID_AI_CLOSE } => {
                // AI panel close
                pollster::block_on(crate::actions::close_command_bar(comp)).ok();
                Some(comp.build_work_content())
            }
            WidgetId::Button { index }
                if *index == lc::BTN_ID_AI_EXPLAIN
                    || *index == lc::BTN_ID_AI_REVIEW
                    || *index == lc::BTN_ID_AI_APPLY
                    || *index == lc::BTN_ID_AI_REJECT =>
            {
                // AI action buttons — refresh composition
                let _ = service;
                Some(comp.build_work_content())
            }
            WidgetId::TextInput { .. } => {
                // TextInput focus handled by interaction model
                None
            }
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
                if *index >= 10 {
                    let comp = self.composition.as_mut()?;
                    let explorer_idx = *index - 10;

                    // Resolve by cached ID for stability (not by positional index
                    // which can shift if cached_explorer_items refreshes between
                    // widget-tree build and click dispatch).
                    let resolve_idx = || -> Option<usize> {
                        let ids = &self.last_explorer_ids;
                        if ids.is_empty() || explorer_idx >= ids.len() {
                            return Some(explorer_idx); // fallback
                        }
                        let target_id = ids.get(explorer_idx)?;
                        comp.cached_explorer_items.iter().position(|ev| &ev.id == target_id)
                    };
                    let resolved = resolve_idx().unwrap_or(explorer_idx);

                    if let Some(ref mut actions) = self.explorer_actions {
                        if comp.is_explorer_item_dir(resolved) {
                            return actions.toggle_directory(comp, resolved);
                        } else {
                            let service = self.workspace_service.clone()?;
                            let view = self.workspace_view.clone()?;
                            let session = self.session_id.clone()?;
                            return actions.open_file(
                                comp,
                                service,
                                view,
                                session,
                                self.workspace_id,
                                resolved,
                            );
                        }
                    }
                }
                // Rail activation: switch active panel / open command
                match index {
                    0 => { /* Explorer — toggle sidebar */ }
                    1 => { /* Search */ }
                    2 => { /* Source Control */ }
                    3 => { /* Debug */ }
                    4 => { /* Settings */ }
                    5 => { /* Account */ }
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
        let mut content_changed = false;
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

                    if let Some(ref wc) = content {
                        let changed = self.work_content.as_ref().map_or(true, |old| {
                            old.explorer_items != wc.explorer_items
                                || old.editor_body.as_ref().map(|b| &b.lines)
                                    != wc.editor_body.as_ref().map(|b| &b.lines)
                        });
                        if changed {
                            self.work_content = Some(wc.clone());
                            content_changed = true;
                        }
                    }
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::HoverChanged(_)
                | zaroxi_core_engine_ui::WidgetAction::Nothing => {}
            }
        }
        if needs_redraw || content_changed {
            self.needs_render = true;
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
        // ── Permanently-ungated focus & pointer-enter diagnostics ──
        // These must print WITHOUT env vars so we can tell whether the
        // pointer ever enters the client area. Printed once per state change.
        match &event {
            WindowEvent::Focused(f) => {
                eprintln!("ZAROXI_LIVE: window Focused({})", f);
            }
            WindowEvent::CursorEntered { .. } => {
                eprintln!("ZAROXI_LIVE: CursorEntered");
            }
            WindowEvent::CursorLeft { .. } => {
                eprintln!("ZAROXI_LIVE: CursorLeft");
            }
            _ => {}
        }

        // ── Gated full event trace (ZAROXI_DEBUG_CLICK=1) ──
        // Logs every incoming winit event with key details.
        click_trace_fmt!("ZAROXI_EVT: {}", event_label(&event));

        match event {
            WindowEvent::CloseRequested => {
                active_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.needs_render = true;
                if let Some(z) = self.maybe_window.as_mut() {
                    z.update_size(size.width, size.height);
                    gui_debug_fmt!(
                        "GuiApp: Resized -> {size:?}, requesting redraw (engine window)"
                    );
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.needs_render = true;
                if let Some(z) = self.maybe_window.as_ref() {
                    gui_debug("GuiApp: ScaleFactorChanged -> requesting redraw (engine window)");
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Always store cursor position so click handling works even
                // before the first redraw populates the widget tree.
                self.interaction.set_cursor_pos(position.x as f32, position.y as f32);
                click_trace_fmt!(
                    "ZAROXI_CLICK: CursorMoved x={:.1} y={:.1} widget_tree={}",
                    position.x,
                    position.y,
                    self.widget_tree.is_some()
                );

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
                            if let Some(vp) = &self.editor_viewport {
                                if let Some((line, col)) = project_editor_cursor(
                                    position,
                                    vp,
                                    &self.shell.work_content,
                                    self.interaction.get_scroll_offset(
                                        &zaroxi_core_engine_ui::WidgetId::Scrollbar {
                                            index: lc::SCROLLBAR_ID_EDITOR,
                                        },
                                    ),
                                ) {
                                    let (sl, sc) = if line < anchor.0
                                        || (line == anchor.0 && col < anchor.1)
                                    {
                                        (line, col)
                                    } else {
                                        anchor
                                    };
                                    let (el, ec) =
                                        if (line, col) > anchor { (line, col) } else { anchor };
                                    self.selection_range = Some((sl, sc, el, ec));
                                    self.needs_render = true;
                                    if let Some(z) = self.maybe_window.as_ref() {
                                        let _ = z.window().request_redraw();
                                    }
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
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f32,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 16.0,
                };
                let editor_id =
                    zaroxi_core_engine_ui::WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                let current = self.interaction.get_scroll_offset(&editor_id);
                let line_h = lc::LINE_HEIGHT;
                let usable_h = self
                    .editor_viewport
                    .as_ref()
                    .map(|vp| vp.content_rect.3 - lc::CONTENT_HEADER_H - lc::CONTENT_PAD_X * 2.0)
                    .unwrap_or(100.0);
                let total_lines = self
                    .work_content
                    .as_ref()
                    .and_then(|w| w.editor_body.as_ref())
                    .map(|cv| cv.lines.len().max(1))
                    .unwrap_or(1) as f32;
                let visible_lines = (usable_h / line_h).max(1.0);
                let step = visible_lines / (total_lines - visible_lines).max(1.0);
                let new_offset = (current - scroll_lines * step).clamp(0.0, 1.0);
                self.interaction.set_scroll_offset(&editor_id, new_offset);
                if let Some(ref mut tree) = self.widget_tree {
                    self.interaction.apply_scroll_offsets(tree);
                }
                if let Some(z) = self.maybe_window.as_ref() {
                    self.needs_render = true;
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    let (x, y) = match self.interaction.cursor_pos_f32() {
                        Some(pos) => pos,
                        None => {
                            click_trace("ZAROXI_CLICK: MouseInput — cursor_pos is None, skipping");
                            return;
                        }
                    };
                    click_trace_fmt!(
                        "ZAROXI_CLICK: MouseInput state={:?} x={:.1} y={:.1} btn_rect={:?}",
                        state,
                        x,
                        y,
                        self.explorer_button_rect
                    );
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
                            // Check explorer CTA button hit rect directly.
                            let mut explorer_activated = false;
                            if let Some((bx, by, bw, bh)) = self.explorer_button_rect {
                                if x >= bx && x <= bx + bw && y >= by && y <= by + bh {
                                    explorer_activated = true;
                                    click_trace_fmt!(
                                        "ZAROXI_CLICK: RELEASE hit CTA rect=({:.1},{:.1},{:.1},{:.1}) click=({:.1},{:.1})",
                                        bx,
                                        by,
                                        bw,
                                        bh,
                                        x,
                                        y
                                    );
                                } else {
                                    click_trace_fmt!(
                                        "ZAROXI_CLICK: RELEASE outside CTA rect=({:.1},{:.1},{:.1},{:.1}) click=({:.1},{:.1})",
                                        bx,
                                        by,
                                        bw,
                                        bh,
                                        x,
                                        y
                                    );
                                }
                            } else {
                                click_trace_fmt!(
                                    "ZAROXI_CLICK: RELEASE btn_rect is None click=({:.1},{:.1})",
                                    x,
                                    y
                                );
                            }
                            if explorer_activated {
                                let id = zaroxi_core_engine_ui::WidgetId::button(
                                    lc::BTN_ID_EXPLORER_CTA,
                                );
                                click_trace("ZAROXI_CLICK: dispatching Activated(Explorer CTA)");
                                self.handle_actions(vec![
                                    zaroxi_core_engine_ui::WidgetAction::Activated(id),
                                ]);
                                Vec::new()
                            } else if let Some(ref mut tree) = self.widget_tree {
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
                            if let Some(vp) = &self.editor_viewport {
                                if let Some((line, col)) = project_editor_cursor(
                                    phys,
                                    vp,
                                    &self.shell.work_content,
                                    self.interaction.get_scroll_offset(
                                        &zaroxi_core_engine_ui::WidgetId::Scrollbar {
                                            index: lc::SCROLLBAR_ID_EDITOR,
                                        },
                                    ),
                                ) {
                                    self.editor_cursor_line = line;
                                    self.editor_cursor_col = col;
                                    self.selection_anchor = Some((line, col));
                                    self.selection_active = true;
                                    self.selection_range = None;
                                    self.needs_render = true;
                                    if let Some(z) = self.maybe_window.as_ref() {
                                        let _ = z.window().request_redraw();
                                    }
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
                if !self.needs_render {
                    return;
                }
                self.needs_render = false;

                if let Some(z) = self.maybe_window.as_mut() {
                    let _ = z.window().pre_present_notify();

                    let (sw, sh) = z.size();
                    if sw == 0 || sh == 0 {
                        return;
                    }

                    let system_is_dark = z
                        .window()
                        .theme()
                        .map(|t| matches!(t, winit::window::Theme::Dark))
                        .unwrap_or(true);
                    let resolved = self.theme_mode.resolve(system_is_dark);
                    let variant = resolved;

                    // ── Editor Phase 1: Taffy-based layout via controller ──
                    let _ = self.layout_controller.get_or_compute(sw, sh, resolved);
                    self.editor_viewport = Some(self.layout_controller.viewport().clone());
                    // Keep shell.work_content up to date for presenter access
                    self.shell.work_content = self.work_content.clone();

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

                    // Widget tree from engine shell layout (cached by controller)
                    let engine_layout = self.layout_controller.engine_shell_layout();
                    let mut widget_tree = zaroxi_core_engine_ui::build_shell_widget_tree(
                        engine_layout,
                        &tokens,
                        self.work_content.as_ref(),
                    );
                    self.interaction.apply_to_tree(&mut widget_tree);
                    self.interaction.apply_scroll_offsets(&mut widget_tree);
                    self.widget_tree = Some(widget_tree.clone());
                    self.last_explorer_ids = self
                        .work_content
                        .as_ref()
                        .and_then(|wc| wc.explorer_panel_items.as_deref())
                        .map(|items| items.iter().map(|it| it.id.clone()).collect())
                        .unwrap_or_default();
                    click_trace_fmt!(
                        "ZAROXI_REDRAW: widget_tree built widgets={} cta_rect_present={}",
                        widget_tree.widgets.len(),
                        self.explorer_button_rect.is_some()
                    );

                    // RenderLayout from shell regions (cached by controller)
                    let shell_regions = self.layout_controller.shell_regions();
                    let render_layout =
                        super::renderbridge::build_render_layout(shell_regions, &tokens);

                    // Update shell.regions for backward compat (cursor projection, etc.)
                    self.shell.regions = shell_regions.to_vec();
                    self.shell.size = *self.layout_controller.size();

                    let editor_data = super::presenters::shape_editor_content(
                        &self.shell.work_content,
                        &sem,
                        &self.parser_pool,
                    );
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

                    let (mut render_blocks, explorer_cta_rect) =
                        super::frame::compose_blocks(shell_regions, &tokens, &ctx);
                    self.explorer_button_rect = explorer_cta_rect;
                    click_trace_fmt!(
                        "ZAROXI_REDRAW: cta_rect={:?}",
                        explorer_cta_rect
                            .map(|(x, y, w, h)| format!("({:.0},{:.0},{:.0}x{:.0})", x, y, w, h))
                    );

                    // Compute scrollbar blocks from shell regions
                    let editor_total_lines = self
                        .shell
                        .work_content
                        .as_ref()
                        .and_then(|wc| wc.editor_body.as_ref())
                        .map(|cv| cv.lines.len())
                        .unwrap_or(0);
                    let line_h = 16.0f32;
                    let editor_region = crate::gui::region_dispatch::find_region_by_role(
                        shell_regions,
                        zaroxi_core_engine_style::PanelRole::ContentArea,
                    );
                    let editor_visible_lines = editor_region
                        .map(|r| lc::visible_lines_from_region(r.rect.height as f32))
                        .unwrap_or(1);

                    let sidebar_region = crate::gui::region_dispatch::find_region_by_role(
                        shell_regions,
                        zaroxi_core_engine_style::PanelRole::SidePanel,
                    );
                    let sidebar_visible = sidebar_region
                        .map(|r| (r.rect.height as f32 / line_h).max(1.0) as usize)
                        .unwrap_or(1);

                    let bottom_region = crate::gui::region_dispatch::find_region_by_role(
                        shell_regions,
                        zaroxi_core_engine_style::PanelRole::BottomPanel,
                    );
                    let bottom_visible = bottom_region
                        .map(|r| lc::visible_lines_from_region(r.rect.height as f32))
                        .unwrap_or(1);

                    let editor_scroll_offset = self.interaction.get_scroll_offset(
                        &zaroxi_core_engine_ui::WidgetId::Scrollbar {
                            index: lc::SCROLLBAR_ID_EDITOR,
                        },
                    );

                    let scroll_blocks = super::frame::compute_scrollbar_blocks(
                        shell_regions,
                        &tokens,
                        editor_total_lines,
                        editor_visible_lines,
                        0,
                        sidebar_visible,
                        0,
                        bottom_visible,
                        editor_scroll_offset,
                    );
                    render_blocks.extend(scroll_blocks);

                    // ── Explorer row hover/focus bridging ──
                    // Sync widget-tree interaction states into painted row blocks
                    // so hovered/focused rows get visual feedback.
                    if let Some(ref tree) = self.widget_tree {
                        for (idx, w) in tree.widgets.iter().enumerate() {
                            if let zaroxi_core_engine_ui::ShellWidget::ListItem {
                                id: zaroxi_core_engine_ui::WidgetId::ListItem { index },
                                state,
                                ..
                            } = w
                            {
                                if *index >= 10 {
                                    let row_idx = *index - 10;
                                    let state = *state;
                                    let hover_focus_color = match state {
                                        zaroxi_core_engine_ui::InteractionState::Hover => {
                                            Some(tokens.hover_bg.to_array())
                                        }
                                        zaroxi_core_engine_ui::InteractionState::Focused
                                        | zaroxi_core_engine_ui::InteractionState::Selected => {
                                            Some(tokens.rail_item_active.to_array())
                                        }
                                        _ => None,
                                    };
                                    if let Some(color) = hover_focus_color {
                                        let block_id = format!("explorer_row_{}", row_idx);
                                        for block in &mut render_blocks {
                                            if block.id == block_id {
                                                block.header_color = Some(color);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            let _ = idx;
                        }
                    }

                    // Phase 72: gated debug
                    if std::env::var("ZAROXI_DEBUG_SEAMS").as_deref() == Ok("1") {
                        for blk in &render_blocks {
                            let narrow_or_tall =
                                blk.rect.w <= 10.0 || blk.rect.h > blk.rect.w * 2.0;
                            if narrow_or_tall {
                                eprintln!(
                                    "ZAROXI_SEAM: win={}x{} id='{}' x={:.1} y={:.1} w={:.1} h={:.1}",
                                    sw, sh, blk.id, blk.rect.x, blk.rect.y, blk.rect.w, blk.rect.h,
                                );
                            }
                        }
                    }

                    // Apply live editor cursor and selection to the ContentArea block
                    // Editor Phase 1: also attach viewport clip rect
                    let is_content_block = |id: &str| {
                        id.contains("ContentArea")
                            || id.contains("content_area")
                            || id == "editor_content"
                    };
                    if let Some(vp) = &self.editor_viewport {
                        for block in &mut render_blocks {
                            if is_content_block(&block.id) {
                                block.cursor_line = Some(self.editor_cursor_line);
                                block.cursor_col = Some(self.editor_cursor_col);
                                block.selection_range = self.selection_range;
                                block.clip_rect = Some(zaroxi_core_engine_render::Rect {
                                    x: vp.clip_rect.0,
                                    y: vp.clip_rect.1,
                                    w: vp.clip_rect.2,
                                    h: vp.clip_rect.3,
                                });
                            }
                        }
                    } else {
                        for block in &mut render_blocks {
                            if is_content_block(&block.id) {
                                block.cursor_line = Some(self.editor_cursor_line);
                                block.cursor_col = Some(self.editor_cursor_col);
                                block.selection_range = self.selection_range;
                            }
                        }
                    }

                    // ── Renderer lifecycle ──
                    let size_changed = (sw as u32, sh as u32) != self.last_render_size;
                    self.last_render_size = (sw as u32, sh as u32);

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

                    // Drop renderer (freed when leaving this scope).
                    // Size changed tracking above allows downstream
                    // text/atlas caches to validate stale layout.
                    let _ = size_changed;

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
                    Key::Named(NamedKey::ArrowDown) => {
                        if let Some(ref mut tree) = self.widget_tree {
                            self.interaction.focus_next_explorer_item(tree)
                        } else {
                            Vec::new()
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        if let Some(ref mut tree) = self.widget_tree {
                            self.interaction.focus_prev_explorer_item(tree)
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
                        Key::Character(c) if c == "w" || c == "W" => {
                            if let Some(comp) = self.composition.as_mut() {
                                let buf_id =
                                    comp.latest_metadata().and_then(|m| m.active_buffer.clone());
                                if let Some(ref id) = buf_id {
                                    if comp.close_opened_buffer(id) {
                                        self.work_content = Some(comp.build_work_content());
                                        self.needs_render = true;
                                        if let Some(z) = self.maybe_window.as_ref() {
                                            let _ = z.window().request_redraw();
                                        }
                                    }
                                }
                            }
                            Vec::new()
                        }
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
    viewport: &EditorViewport,
    work_content: &Option<crate::gui::ShellWorkContent>,
    editor_scroll_offset: f32,
) -> Option<(usize, usize)> {
    let px = cursor_pos.x as f32;
    let py = cursor_pos.y as f32;

    if !viewport.contains_point(px, py) {
        return None;
    }

    let content_pad = lc::CONTENT_PAD_X;
    let header_h = lc::CONTENT_HEADER_H;
    let line_h = lc::LINE_HEIGHT;
    let char_w = lc::CHAR_WIDTH_STUB;
    let content_x = viewport.content_rect.0 + content_pad;
    let content_y = viewport.content_rect.1 + header_h + content_pad;
    let rel_y = py - content_y;
    let rel_x = px - content_x;
    let visible_line = (rel_y / line_h).max(0.0) as usize;
    let col = (rel_x / char_w).max(0.0) as usize;

    let usable_h = viewport.content_rect.3 - header_h - content_pad * 2.0;

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
