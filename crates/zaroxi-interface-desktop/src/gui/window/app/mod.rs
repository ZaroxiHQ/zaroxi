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

Phase 60 (Architecture Refactor): `app.rs` split into focused sub-modules
so that `mod.rs` only contains the struct, thin winit-lifecycle wiring,
and high-level delegation.  Detail lives in:
- `activation.rs`         — widget activation routing & explorer CTA
- `input.rs`              — keyboard interpretation & mouse-wheel normalisation
- `editor_interaction.rs` — cursor projection, selection & hit-testing
- `render_state.rs`       — content hashing, editor-data caching
- `debug.rs`              — shared debug/trace helpers
*/

mod activation;
pub(crate) mod debug;
mod editor_interaction;
mod input;
mod render_state;

use std::sync::Arc;
use std::sync::mpsc;

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::ControlFlow,
    window::WindowAttributes,
};

use crate::DesktopComposition;
use crate::folder_picker::{DynFolderPicker, PickerOutcome};
use crate::gui::window::editor_shell::{EditorViewport, ShellLayoutController};
use crate::gui::window::explorer_panel::ExplorerPanelActions;
use crate::gui::{ShellFrame, ShellWorkContent};
use zaroxi_application_workspace::ports::{SessionId, WorkspaceService, WorkspaceView};
use zaroxi_core_engine_render::RenderCore;
use zaroxi_core_engine_ui::WidgetId;
use zaroxi_core_engine_ui::layout_constants as lc;
use zaroxi_core_platform_syntax::parser::ParserPool;
use zaroxi_kernel_types::Id;

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
    pub on_widget_activated: Option<WidgetActivationHandler>,
    pub composition: Option<DesktopComposition>,
    pub workspace_view: Option<Arc<dyn WorkspaceView>>,
    pub workspace_service: Option<Arc<dyn WorkspaceService>>,
    pub session_id: Option<SessionId>,
    pub workspace_id: Option<Id>,
    pub folder_picker: Option<DynFolderPicker>,
    pub explorer_actions: Option<ExplorerPanelActions>,
    pub explorer_button_rect: Option<(f32, f32, f32, f32)>,
    pub parser_pool: ParserPool,
    pub cached_editor_data: Option<crate::gui::window::editor::EditorContentData>,
    pub cached_editor_lines_hash: u64,
    pub layout_controller: ShellLayoutController,
    pub editor_viewport: Option<EditorViewport>,
    pub needs_render: bool,
    pub last_explorer_ids: Vec<String>,
    pub last_render_size: (u32, u32),
    pub pending_scroll_frac: f32,
    pub render_core: Option<RenderCore>,
    pub picker_in_flight: bool,
    pub pending_picker_rx: Option<mpsc::Receiver<PickerOutcome>>,
}

impl GuiApp {
    pub fn dispatch_activation(&mut self, id: &WidgetId) -> Option<ShellWorkContent> {
        activation::dispatch_activation(self, id)
    }

    pub fn process_picker_result(&mut self) {
        if !self.picker_in_flight {
            return;
        }
        if let Some(ref rx) = self.pending_picker_rx {
            if let Ok(outcome) = rx.try_recv() {
                self.pending_picker_rx = None;
                self.picker_in_flight = false;
                match outcome {
                    PickerOutcome::Selected(path) => {
                        debug::click_trace_fmt!(
                            "ZAROXI_PICKER: thread result=Selected({})",
                            path.display()
                        );
                        if let Some(ref mut actions) = self.explorer_actions {
                            let comp = match self.composition.as_mut() {
                                Some(c) => c,
                                None => return,
                            };
                            let service = match self.workspace_service.clone() {
                                Some(s) => s,
                                None => return,
                            };
                            let view = match self.workspace_view.clone() {
                                Some(v) => v,
                                None => return,
                            };
                            let content = actions.open_workspace(
                                comp,
                                service,
                                view,
                                &mut self.session_id,
                                &mut self.workspace_id,
                                path,
                            );
                            if let Some(wc) = content {
                                self.work_content = Some(wc);
                                self.needs_render = true;
                                if let Some(z) = self.maybe_window.as_ref() {
                                    let _ = z.window().request_redraw();
                                }
                            }
                        }
                    }
                    PickerOutcome::Cancelled => {
                        debug::click_trace("ZAROXI_PICKER: thread result=Cancelled");
                        if let Some(ref mut comp) = self.composition {
                            comp.set_status_message("No folder selected".to_string());
                            self.work_content = Some(comp.build_work_content());
                            self.needs_render = true;
                            if let Some(z) = self.maybe_window.as_ref() {
                                let _ = z.window().request_redraw();
                            }
                        }
                    }
                    PickerOutcome::Unavailable { reason, .. } => {
                        debug::click_trace_fmt!(
                            "ZAROXI_PICKER: thread result=Unavailable({})",
                            reason
                        );
                        if let Some(ref mut comp) = self.composition {
                            let msg = if reason.len() > 90 {
                                "Workspace picker unavailable — see log for details".to_string()
                            } else {
                                format!("Workspace picker unavailable: {}", reason)
                            };
                            comp.set_status_message(msg);
                            self.work_content = Some(comp.build_work_content());
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

    pub fn handle_actions(&mut self, actions: Vec<zaroxi_core_engine_ui::WidgetAction>) {
        let mut needs_redraw = false;
        let mut content_changed = false;
        for action in actions {
            match action {
                zaroxi_core_engine_ui::WidgetAction::StateNeedsRedraw => {
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::FocusChanged(_prev_focus) => {
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::ScrollOffsetChanged(id, offset) => {
                    let old_offset = self.interaction.get_scroll_offset(&id);
                    let offset_delta = offset - old_offset;
                    self.interaction.set_scroll_offset(&id, offset);
                    if (id == WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR })
                        && offset_delta.abs() > 0.0001
                    {
                        let total_lines = self
                            .work_content
                            .as_ref()
                            .and_then(|w| w.editor_body.as_ref())
                            .map(|cv| cv.lines.len().max(1))
                            .unwrap_or(1) as f32;
                        let visible = self
                            .editor_viewport
                            .as_ref()
                            .map(|vp| lc::visible_lines_from_region(vp.content_rect.3) as f32)
                            .unwrap_or(1.0) as f32;
                        let max_scroll_lines = (total_lines - visible).max(1.0);
                        let line_delta = (offset_delta * max_scroll_lines).round() as isize;
                        if let Some(ref mut comp) = self.composition {
                            comp.pending_scroll_lines += line_delta;
                            comp.pending_refresh_reason = Some(
                                zaroxi_application_workspace::workspace_view::RefreshReason::CursorMoved,
                            );
                        }
                    }
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::Activated(ref id) => {
                    let content = self
                        .on_widget_activated
                        .as_mut()
                        .and_then(|handler| handler(id))
                        .or_else(|| activation::dispatch_activation(self, id));

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
            debug::gui_debug("GuiApp: attempting to create window (StartCause::Init)");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    debug::gui_debug_fmt!("GuiApp: created engine window id={:?}", wid);
                    zaroxi_w.window().set_title(&self.title);
                    let _ = zaroxi_w.window().set_outer_position(PhysicalPosition::new(100, 100));
                    self.maybe_window = Some(zaroxi_w);

                    if let Some(z) = self.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                    active_loop.set_control_flow(ControlFlow::Wait);
                    debug::gui_debug("GuiApp: window created (hidden); initial redraw requested");
                }
                Err(e) => {
                    eprintln!("GuiApp: failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else if self.maybe_window.is_some() {
            if !self.already_logged_existing {
                debug::gui_debug("GuiApp: new_events called but window already created");
                self.already_logged_existing = true;
            }
        } else {
            debug::gui_debug_fmt!("GuiApp: new_events called with cause={:?} (no creation)", cause);
        }
    }

    fn resumed(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        if self.maybe_window.is_none() {
            debug::gui_debug("GuiApp: resumed -> attempting to create window");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    debug::gui_debug_fmt!("GuiApp: created engine window on resumed id={:?}", wid);
                    self.maybe_window = Some(zaroxi_w);

                    if let Some(z) = self.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                    debug::gui_debug(
                        "GuiApp: window created on resumed (hidden); initial redraw requested",
                    );
                }
                Err(e) => {
                    eprintln!("GuiApp: resumed failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else {
            debug::gui_debug("GuiApp: resumed called but window already created");
        }
    }

    fn about_to_wait(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        self.process_picker_result();

        if self.requested_initial_frame {
            if let Some(z) = self.maybe_window.as_ref() {
                debug::gui_debug(
                    "GuiApp: about_to_wait -> requesting initial redraw (engine window)",
                );
                let _ = z.window().request_redraw();
            }
            self.requested_initial_frame = false;
            active_loop.set_control_flow(ControlFlow::Wait);
            debug::gui_debug("GuiApp: about_to_wait -> switched control flow back to Wait");
        }
    }

    fn window_event(
        &mut self,
        active_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        // ── Permanently-ungated focus & pointer-enter diagnostics ──
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
        debug::click_trace_fmt!("ZAROXI_EVT: {}", debug::event_label(&event));

        match event {
            WindowEvent::CloseRequested => {
                active_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.needs_render = true;
                if let Some(z) = self.maybe_window.as_mut() {
                    z.update_size(size.width, size.height);
                    debug::gui_debug_fmt!(
                        "GuiApp: Resized -> {size:?}, requesting redraw (engine window)"
                    );
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.needs_render = true;
                if let Some(z) = self.maybe_window.as_ref() {
                    debug::gui_debug(
                        "GuiApp: ScaleFactorChanged -> requesting redraw (engine window)",
                    );
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.interaction.set_cursor_pos(position.x as f32, position.y as f32);
                debug::click_trace_fmt!(
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
                        editor_interaction::update_drag_selection(self, position);
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
                input::process_mouse_wheel(self, &delta);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    let (x, y) = match self.interaction.cursor_pos_f32() {
                        Some(pos) => pos,
                        None => {
                            debug::click_trace(
                                "ZAROXI_CLICK: MouseInput — cursor_pos is None, skipping",
                            );
                            return;
                        }
                    };
                    debug::click_trace_fmt!(
                        "ZAROXI_CLICK: MouseInput state={:?} x={:.1} y={:.1} btn_rect={:?}",
                        state,
                        x,
                        y,
                        self.explorer_button_rect
                    );
                    let actions = match state {
                        ElementState::Pressed => {
                            if let Some(ref mut tree) = self.widget_tree {
                                self.interaction.on_pointer_down(
                                    tree,
                                    x,
                                    y,
                                    zaroxi_core_engine_ui::PointerButton::Primary,
                                )
                            } else {
                                Vec::new()
                            }
                        }
                        ElementState::Released => {
                            let mut explorer_activated = false;
                            if let Some((bx, by, bw, bh)) = self.explorer_button_rect {
                                if x >= bx && x <= bx + bw && y >= by && y <= by + bh {
                                    explorer_activated = true;
                                    debug::click_trace_fmt!(
                                        "ZAROXI_CLICK: RELEASE hit CTA rect=({:.1},{:.1},{:.1},{:.1}) click=({:.1},{:.1})",
                                        bx,
                                        by,
                                        bw,
                                        bh,
                                        x,
                                        y
                                    );
                                } else {
                                    debug::click_trace_fmt!(
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
                                debug::click_trace_fmt!(
                                    "ZAROXI_CLICK: RELEASE btn_rect is None click=({:.1},{:.1})",
                                    x,
                                    y
                                );
                            }
                            if explorer_activated {
                                let id = zaroxi_core_engine_ui::WidgetId::button(
                                    lc::BTN_ID_EXPLORER_CTA,
                                );
                                debug::click_trace(
                                    "ZAROXI_CLICK: dispatching Activated(Explorer CTA)",
                                );
                                self.handle_actions(vec![
                                    zaroxi_core_engine_ui::WidgetAction::Activated(id),
                                ]);
                                Vec::new()
                            } else if let Some(ref mut tree) = self.widget_tree {
                                self.interaction.on_pointer_up(
                                    tree,
                                    x,
                                    y,
                                    zaroxi_core_engine_ui::PointerButton::Primary,
                                )
                            } else {
                                Vec::new()
                            }
                        }
                    };
                    self.handle_actions(actions);

                    if let ElementState::Pressed = state {
                        editor_interaction::init_selection_from_click(self);
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

                    let _ = self.layout_controller.get_or_compute(sw, sh, resolved);
                    self.editor_viewport = Some(self.layout_controller.viewport().clone());
                    self.shell.work_content = self.work_content.clone();

                    let mut sem = variant.colors(false);

                    let debug_theme_active =
                        std::env::var("ZAROXI_DEBUG_THEME").as_deref() == Ok("1");
                    if debug_theme_active {
                        sem = zaroxi_interface_theme::theme::SemanticColors::debug();
                        debug::gui_debug("ZAROXI_DEBUG_THEME: debug theme override ACTIVE");
                    }

                    if !self.first_render_shown && debug_theme_active {
                        debug::gui_debug_fmt!(
                            "ZAROXI_THEME_TRACE: mode={:?} system_is_dark={} resolved={:?}",
                            self.theme_mode,
                            system_is_dark,
                            variant
                        );
                        debug::gui_debug_fmt!(
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
                        debug::gui_debug_fmt!(
                            "ZAROXI_STYLE_TOKENS: app_bg={:?} titlebar_bg={:?} editor_bg={:?} sidebar_bg={:?}",
                            tokens.app_background.to_array(),
                            tokens.titlebar_background.to_array(),
                            tokens.editor_content_background.to_array(),
                            tokens.sidebar_background.to_array(),
                        );
                    }

                    if let Some(ref mut comp) = self.composition {
                        comp.apply_pending_scrolls();
                    }

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
                    debug::click_trace_fmt!(
                        "ZAROXI_REDRAW: widget_tree built widgets={} cta_rect_present={}",
                        widget_tree.widgets.len(),
                        self.explorer_button_rect.is_some()
                    );

                    let shell_regions = self.layout_controller.shell_regions();
                    let render_layout =
                        super::renderbridge::build_render_layout(shell_regions, &tokens);

                    self.shell.regions = shell_regions.to_vec();
                    self.shell.size = *self.layout_controller.size();

                    let editor_data = render_state::prepare_editor_data(
                        &self.shell.work_content,
                        &mut self.cached_editor_data,
                        &mut self.cached_editor_lines_hash,
                        &self.parser_pool,
                        &sem,
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
                    debug::click_trace_fmt!(
                        "ZAROXI_REDRAW: cta_rect={:?}",
                        explorer_cta_rect
                            .map(|(x, y, w, h)| format!("({:.0},{:.0},{:.0}x{:.0})", x, y, w, h))
                    );

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

                    if let Some(ref mut comp) = self.composition {
                        comp.set_editor_viewport_lines(editor_visible_lines);
                    }

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

                    let editor_scroll_offset = self
                        .interaction
                        .get_scroll_offset(&WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR });

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
                    if let Some(ref tree) = self.widget_tree {
                        for w in &tree.widgets {
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
                        }
                    }

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
                                if let Some(ref comp) = self.composition {
                                    if let Some(meta) = &comp.metadata {
                                        block.content_offset_x =
                                            meta.editor_horizontal_offset_px.unwrap_or(0.0);
                                        let off_y = meta.editor_scroll_px;
                                        block.content_offset_y = off_y;
                                        if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref()
                                            == Ok("1")
                                        {
                                            eprintln!(
                                                "ZAROXI_SCROLL: block content_offset x={:.1} y={:.1} px={:.1}",
                                                block.content_offset_x,
                                                off_y,
                                                meta.editor_scroll_px
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Apply vertical scroll offset to the gutter lane block
                        if let Some(ref comp) = self.composition {
                            if let Some(meta) = &comp.metadata {
                                let off_y = meta.editor_scroll_top_line as f32 * lc::LINE_HEIGHT;
                                for block in &mut render_blocks {
                                    if block.id == "gutter_lane" {
                                        block.clip_rect = Some(zaroxi_core_engine_render::Rect {
                                            x: block.rect.x,
                                            y: block.rect.y,
                                            w: block.rect.w,
                                            h: block.rect.h,
                                        });
                                        block.content_offset_y = off_y;
                                        block.content_offset_x =
                                            meta.editor_horizontal_offset_px.unwrap_or(0.0);
                                        break;
                                    }
                                }
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
                    self.last_render_size = (sw as u32, sh as u32);

                    let clear_color = [
                        tokens.app_background.r as f64,
                        tokens.app_background.g as f64,
                        tokens.app_background.b as f64,
                        1.0,
                    ];

                    if self.render_core.is_none() {
                        match pollster::block_on(RenderCore::new(clear_color)) {
                            Ok(rc) => {
                                self.render_core = Some(rc);
                            }
                            Err(e) => {
                                eprintln!("GuiApp: failed to create RenderCore: {:?}", e);
                                return;
                            }
                        }
                    }

                    if let Some(ref mut rc) = self.render_core {
                        match rc.render_to_window(z.window(), &render_layout, &render_blocks) {
                            Ok(()) => {
                                if !self.first_render_shown {
                                    let _ = z.window().set_visible(true);
                                    let _ = z.window().pre_present_notify();
                                    self.first_render_shown = true;
                                    eprintln!("GuiApp: first full-renderer frame; window visible");
                                }
                            }
                            Err(e) => {
                                eprintln!("GuiApp: render_to_window failed: {:?}", e);
                            }
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
                let actions = input::handle_keyboard_press(self, &event.logical_key);
                self.handle_actions(actions);
            }
            _ => {}
        }
    }
}
