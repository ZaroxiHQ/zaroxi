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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::ControlFlow,
    window::WindowAttributes,
};

static GUI_FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);

fn render_trace_enabled() -> bool {
    std::env::var("ZAROXI_RENDER_TRACE").as_deref() == Ok("1")
}

fn scroll_trace_enabled() -> bool {
    std::env::var("ZAROXI_SCROLL_TRACE").as_deref() == Ok("1")
}

fn record_frame_presented() {
    if std::env::var("ZAROXI_FPS_TRACE").as_deref() != Ok("1") {
        return;
    }
    let now = std::time::Instant::now();
    use std::sync::Mutex;
    static TRACKER: Mutex<Option<(Option<std::time::Instant>, u64, u64, f64, std::time::Instant)>> =
        Mutex::new(None);
    let mut guard = TRACKER.lock().unwrap();
    if guard.is_none() {
        *guard = Some((None, 0, 0, 0.0, now));
    }
    let (last_frame, count, win_frames, win_sum_ms, win_start) = guard.as_mut().unwrap();
    *count += 1;
    let dt_ms: f64 = last_frame.map_or(0.0, |lf| (now - lf).as_secs_f64() * 1000.0);
    *last_frame = Some(now);

    *win_frames += 1;
    *win_sum_ms += dt_ms;
    let win_elapsed = (now - *win_start).as_secs_f64();
    if win_elapsed >= 1.0 {
        let avg_fps = *win_frames as f64 / win_elapsed;
        let avg_ms = *win_sum_ms / (*win_frames).max(1) as f64;
        eprintln!(
            "ZAROXI_FPS_TRACE: rolling frames={} avg_fps={:.1} avg_frame_ms={:.1}",
            win_frames, avg_fps, avg_ms
        );
        *win_start = now;
        *win_frames = 0;
        *win_sum_ms = 0.0;
    }
    eprintln!(
        "ZAROXI_FPS_TRACE: frame={} dt_ms={:.1} instant_fps={:.0}",
        count,
        dt_ms,
        if dt_ms > 0.0 { 1000.0 / dt_ms } else { 0.0 }
    );
}

use crate::DesktopComposition;
use crate::folder_picker::{DynFolderPicker, PickerOutcome};
use crate::gui::window::editor_buf::EditorBufferState;
use crate::gui::window::editor_shell::{EditorViewport, ShellLayoutController};
use crate::gui::window::explorer_panel::ExplorerPanelActions;
use crate::gui::{ShellFrame, ShellWorkContent};
use zaroxi_application_workspace::ports::{SessionId, WorkspaceService, WorkspaceView};
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
    pub editor_buffer: EditorBufferState,
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
    pub picker_in_flight: bool,
    pub pending_picker_rx: Option<mpsc::Receiver<PickerOutcome>>,
    pub last_widget_tree_size: (u32, u32),
    pub last_widget_tree_content: Option<ShellWorkContent>,
    pub render_core: Option<zaroxi_core_engine_render::renderer::core::RenderCore>,
    /// Per-line syntax-colored span cache keyed by (line_index, content_fnv_hash).
    /// Avoids recomputing spans for lines whose content didn't change.
    pub line_syntax_cache: HashMap<(usize, u64), Vec<(String, [f32; 4])>>,
    /// Per-line raw-content fnv hash from the last cache build.
    pub cached_line_hashes: Vec<u64>,
}

impl GuiApp {
    pub fn editor_cursor_line(&self) -> usize {
        self.editor_buffer.caret_line()
    }

    pub fn editor_cursor_col(&self) -> usize {
        self.editor_buffer.caret_col()
    }

    pub fn editor_selection_range(&self) -> Option<(usize, usize, usize, usize)> {
        self.editor_buffer.selection_range()
    }

    pub fn editor_selection_active(&self) -> bool {
        self.editor_buffer.selection_active
    }

    /// Return the monospace character advance from the font system,
    /// falling back to the layout-constant stub when the renderer isn't available.
    pub fn monospace_advance_x(&self) -> Option<f32> {
        self.render_core
            .as_ref()
            .and_then(|core| core.text_renderer().and_then(|tr| tr.monospace_advance_x()))
    }

    /// Set the work_content and sync the editor buffer from its content.
    fn set_work_content(&mut self, wc: ShellWorkContent) {
        if let Some(ref body) = wc.editor_body {
            self.editor_buffer.populate_from_lines(&body.lines, body.cursor_line, body.cursor_col);
        }
        self.work_content = Some(wc);
    }
}

impl GuiApp {
    pub fn dispatch_activation(&mut self, id: &WidgetId) -> Option<ShellWorkContent> {
        activation::dispatch_activation(self, id)
    }

    fn request_render(&mut self) {
        if render_trace_enabled() {
            let pending = GUI_FRAME_COUNTER.load(Ordering::Relaxed) + 1;
            eprintln!(
                "ZAROXI_RENDER_TRACE: request_render frame_pending={} already_dirty={}",
                pending, self.needs_render
            );
        }
        if !self.needs_render {
            if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                if let Some(z) = self.maybe_window.as_ref() {
                    eprintln!("ZAROXI_FRAMEFLOW: request_render dirty id={:?}", z.window().id());
                }
            }
        }
        self.needs_render = true;
        if let Some(z) = self.maybe_window.as_ref() {
            let _ = z.window().request_redraw();
        }
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
                        debug::click_trace_fmt!(
                            "ZAROXI_DIAG: picker Selected({}) — composition exists={} explorer_actions exists={}",
                            path.display(),
                            self.composition.is_some(),
                            self.explorer_actions.is_some()
                        );
                        if let Some(ref mut actions) = self.explorer_actions {
                            let comp = match self.composition.as_mut() {
                                Some(c) => c,
                                None => {
                                    debug::click_trace(
                                        "ZAROXI_DIAG: composition is None — cannot open workspace",
                                    );
                                    return;
                                }
                            };
                            let service = match self.workspace_service.clone() {
                                Some(s) => s,
                                None => {
                                    debug::click_trace("ZAROXI_DIAG: workspace_service is None");
                                    return;
                                }
                            };
                            let view = match self.workspace_view.clone() {
                                Some(v) => v,
                                None => {
                                    debug::click_trace("ZAROXI_DIAG: workspace_view is None");
                                    return;
                                }
                            };
                            debug::click_trace_fmt!(
                                "ZAROXI_DIAG: calling open_workspace with path={}",
                                path.display()
                            );
                            let pre_root = comp.workspace_root_path.clone();
                            let pre_items = comp.cached_explorer_items.len();
                            debug::click_trace_fmt!(
                                "ZAROXI_DIAG: BEFORE open_workspace — root={:?} cached_items={}",
                                pre_root,
                                pre_items
                            );
                            let content = actions.open_workspace(
                                comp,
                                service,
                                view,
                                &mut self.session_id,
                                &mut self.workspace_id,
                                path,
                            );
                            let post_root = comp.workspace_root_path.clone();
                            let post_items = comp.cached_explorer_items.len();
                            debug::click_trace_fmt!(
                                "ZAROXI_DIAG: AFTER open_workspace — root={:?} cached_items={} content_is_some={}",
                                post_root,
                                post_items,
                                content.is_some()
                            );
                            if let Some(ref wc) = content {
                                debug::click_trace_fmt!(
                                    "ZAROXI_DIAG: work_content — empty_button={:?} panel_items_count={}",
                                    wc.explorer_empty_button,
                                    wc.explorer_panel_items.as_ref().map_or(0, |v| v.len())
                                );
                            }
                            if let Some(wc) = content {
                                self.set_work_content(wc);
                                self.last_widget_tree_content = None;
                                self.pending_scroll_frac = 0.0;
                                if let Some(ref mut comp) = self.composition {
                                    comp.reset_scroll_state();
                                }
                                let editor_id =
                                    WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                                self.interaction.set_scroll_offset(&editor_id, 0.0);
                                self.request_render();
                            } else {
                                debug::click_trace(
                                    "ZAROXI_DIAG: open_workspace returned None — explorer stays empty",
                                );
                            }
                        }
                    }
                    PickerOutcome::Cancelled => {
                        debug::click_trace("ZAROXI_PICKER: thread result=Cancelled");
                        let wc = if let Some(ref mut comp) = self.composition {
                            comp.set_status_message("No folder selected".to_string());
                            comp.build_work_content()
                        } else {
                            return;
                        };
                        self.set_work_content(wc);
                        self.last_widget_tree_content = None;
                        self.request_render();
                    }
                    PickerOutcome::Unavailable { reason, .. } => {
                        debug::click_trace_fmt!(
                            "ZAROXI_PICKER: thread result=Unavailable({})",
                            reason
                        );
                        let wc = if let Some(ref mut comp) = self.composition {
                            let msg = if reason.len() > 90 {
                                "Workspace picker unavailable — see log for details".to_string()
                            } else {
                                format!("Workspace picker unavailable: {}", reason)
                            };
                            comp.set_status_message(msg);
                            comp.build_work_content()
                        } else {
                            return;
                        };
                        self.set_work_content(wc);
                        self.last_widget_tree_content = None;
                        self.request_render();
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
                                || old.active_file != wc.active_file
                                || old.editor_tabs != wc.editor_tabs
                                || old.editor_body.as_ref().map(|b| &b.lines)
                                    != wc.editor_body.as_ref().map(|b| &b.lines)
                        });
                        if changed {
                            self.set_work_content(wc.clone());
                            content_changed = true;
                            self.pending_scroll_frac = 0.0;
                            if let Some(ref mut comp) = self.composition {
                                comp.reset_scroll_state();
                            }
                            let editor_id = WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                            self.interaction.set_scroll_offset(&editor_id, 0.0);
                        }
                    }
                    needs_redraw = true;
                }
                zaroxi_core_engine_ui::WidgetAction::HoverChanged(_)
                | zaroxi_core_engine_ui::WidgetAction::Nothing => {}
            }
        }
        if needs_redraw || content_changed {
            self.request_render();
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

                    self.request_render();
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

                    self.request_render();
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
            self.request_render();
            self.requested_initial_frame = false;
            active_loop.set_control_flow(ControlFlow::Wait);
            debug::gui_debug("GuiApp: about_to_wait -> switched control flow back to Wait");
        } else if self.picker_in_flight {
            active_loop.set_control_flow(ControlFlow::Poll);
        } else if self.needs_render || self.interaction.scrollbar_drag_active() {
            active_loop.set_control_flow(ControlFlow::Poll);
        } else {
            active_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn window_event(
        &mut self,
        active_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.process_picker_result();

        // ── Gated focus / pointer-enter diagnostics (ZAROXI_LIVE_DIAG=1) ──
        if std::env::var("ZAROXI_LIVE_DIAG").as_deref() == Ok("1") {
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
                    if self.editor_selection_active() {
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
                        self.editor_buffer.end_selection();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let frame_id = GUI_FRAME_COUNTER.fetch_add(1, Ordering::Relaxed);
                if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                    eprintln!(
                        "ZAROXI_FRAMEFLOW: RedrawRequested id={:?} dirty={}",
                        window_id, self.needs_render
                    );
                }
                if render_trace_enabled() {
                    eprintln!(
                        "ZAROXI_RENDER_TRACE: RedrawRequested frame={} dirty={}",
                        frame_id, self.needs_render
                    );
                }
                if !self.needs_render {
                    if render_trace_enabled() {
                        eprintln!(
                            "ZAROXI_RENDER_TRACE: RedrawRequested frame={} SKIPPED (not dirty)",
                            frame_id
                        );
                    }
                    return;
                }

                let cursor_line = self.editor_cursor_line();
                let cursor_col = self.editor_cursor_col();
                let selection_range = self.editor_selection_range();

                if let Some(z) = self.maybe_window.as_mut() {
                    let (sw, sh) = z.size();
                    if sw == 0 || sh == 0 {
                        if render_trace_enabled() {
                            eprintln!(
                                "ZAROXI_RENDER_TRACE: RedrawRequested frame={} SKIPPED (zero size)",
                                frame_id
                            );
                        }
                        return;
                    }

                    // Notify compositor before rendering this frame.
                    // Required on Wayland to register for the next frame callback.
                    let _ = z.window().pre_present_notify();

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

                    // Sync normalized scroll offset from canonical top_line to interaction model.
                    // Must run unconditionally — small files (total <= visible) need offset 0.0
                    // to avoid a stale value from a previous file.
                    if let Some(ref comp) = self.composition {
                        if let Some(ref meta) = comp.metadata {
                            let total_lines = self
                                .work_content
                                .as_ref()
                                .and_then(|w| w.editor_body.as_ref())
                                .map(|cv| cv.lines.len())
                                .unwrap_or(0);
                            let visible = meta.editor_viewport_line_count.unwrap_or(10).max(1);
                            let max_scroll = total_lines.saturating_sub(visible).max(1) as f32;
                            let norm_offset = (meta.editor_scroll_top_line as f32
                                / max_scroll.max(1.0))
                            .clamp(0.0, 1.0);
                            let editor_id = WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                            self.interaction.set_scroll_offset(&editor_id, norm_offset);
                        }
                    }

                    let engine_layout = self.layout_controller.engine_shell_layout();

                    let content_changed = self
                        .last_widget_tree_content
                        .as_ref()
                        .and_then(|old| {
                            self.work_content.as_ref().map(|new| {
                                old.explorer_empty_button != new.explorer_empty_button
                                    || old.explorer_panel_items.as_ref().map(|v| v.len())
                                        != new.explorer_panel_items.as_ref().map(|v| v.len())
                                    || old.editor_body.as_ref().map(|b| b.lines.len())
                                        != new.editor_body.as_ref().map(|b| b.lines.len())
                                    || old.active_file != new.active_file
                                    || old.editor_tabs != new.editor_tabs
                            })
                        })
                        .unwrap_or(true);
                    let rebuild_tree = self.last_widget_tree_size != (sw, sh) || content_changed;

                    self.last_widget_tree_size = (sw, sh);
                    if let Some(ref wc) = self.work_content {
                        self.last_widget_tree_content = Some(wc.clone());
                    }

                    let mut widget_tree = if rebuild_tree {
                        let new_tree = zaroxi_core_engine_ui::build_shell_widget_tree(
                            engine_layout,
                            &tokens,
                            self.work_content.as_ref(),
                        );
                        new_tree
                    } else {
                        self.widget_tree.clone().unwrap_or_else(|| {
                            zaroxi_core_engine_ui::build_shell_widget_tree(
                                engine_layout,
                                &tokens,
                                self.work_content.as_ref(),
                            )
                        })
                    };

                    self.interaction.apply_to_tree(&mut widget_tree);

                    // Fix editor scrollbar thumb height to match actual content ratio.
                    let editor_id = WidgetId::Scrollbar { index: lc::SCROLLBAR_ID_EDITOR };
                    let total_lines = self
                        .work_content
                        .as_ref()
                        .and_then(|w| w.editor_body.as_ref())
                        .map(|cv| cv.lines.len().max(1))
                        .unwrap_or(1);
                    let visible = self
                        .editor_viewport
                        .as_ref()
                        .map(|vp| lc::visible_lines_from_region(vp.content_rect.3) as usize)
                        .unwrap_or(10)
                        .max(1);
                    let thumb_ratio = (visible as f32 / total_lines as f32).clamp(0.05, 1.0);
                    for w in &mut widget_tree.widgets {
                        if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
                            id,
                            track_rect,
                            thumb_rect,
                            ..
                        } = w
                        {
                            if id == &editor_id {
                                let min_h = 20.0f32;
                                let new_h = (track_rect.height * thumb_ratio)
                                    .max(min_h)
                                    .min(track_rect.height);
                                thumb_rect.height = new_h;
                            }
                        }
                    }

                    self.interaction.apply_scroll_offsets(&mut widget_tree);
                    self.widget_tree = Some(widget_tree.clone());

                    if scroll_trace_enabled() {
                        let engine_layout = self.layout_controller.engine_shell_layout();
                        let content_right =
                            engine_layout.content_area.x + engine_layout.content_area.width;
                        let ai_left = engine_layout.right_panel.x;
                        let mut found = false;
                        for w in &widget_tree.widgets {
                            if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
                                id,
                                track_rect,
                                thumb_rect,
                                ..
                            } = w
                            {
                                if id == &editor_id {
                                    eprintln!(
                                        "ZAROXI_SCROLL_TRACE: widget_tree scrollbar rect=(ix={:.1},iy={:.1},iw={:.1},ih={:.1}) thumb_h={:.1} hit_right={:.1} content_right={:.1} ai_left={:.1}",
                                        track_rect.x,
                                        track_rect.y,
                                        track_rect.width,
                                        track_rect.height,
                                        thumb_rect.height,
                                        track_rect.x + track_rect.width,
                                        content_right,
                                        ai_left
                                    );
                                    found = true;
                                }
                            }
                        }
                        if !found {
                            eprintln!(
                                "ZAROXI_SCROLL_TRACE: widget_tree scrollbar MISSING total_widgets={} content_right={:.1} ai_left={:.1}",
                                widget_tree.widgets.len(),
                                content_right,
                                ai_left
                            );
                        }
                    }
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
                    debug::click_trace_fmt!(
                        "ZAROXI_DIAG: window={}x{} layout_last={}x{} nregions={}",
                        sw,
                        sh,
                        self.layout_controller.size().width,
                        self.layout_controller.size().height,
                        shell_regions.len(),
                    );
                    for r in shell_regions {
                        if r.rect.width > 0 || r.rect.height > 0 {
                            debug::click_trace_fmt!(
                                "ZAROXI_DIAG:   region id={} x={} y={} w={} h={}",
                                r.id,
                                r.rect.x,
                                r.rect.y,
                                r.rect.width,
                                r.rect.height,
                            );
                        }
                    }
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
                        &mut self.line_syntax_cache,
                        &mut self.cached_line_hashes,
                    );
                    let explorer_data =
                        super::presenters::shape_explorer_content(&self.shell.work_content);
                    let ai_data = super::presenters::shape_ai_content(&self.shell.work_content);
                    let status_data = super::presenters::shape_status_content(
                        &self.shell.work_content,
                        cursor_line,
                        cursor_col,
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

                    // ── Scrollbar hover/active state bridging ──
                    if let Some(ref tree) = self.widget_tree {
                        for w in &tree.widgets {
                            if let zaroxi_core_engine_ui::ShellWidget::ScrollBar {
                                id: zaroxi_core_engine_ui::WidgetId::Scrollbar { index },
                                state,
                                ..
                            } = w
                            {
                                if *index == lc::SCROLLBAR_ID_EDITOR {
                                    let highlight_color = match *state {
                                        zaroxi_core_engine_ui::InteractionState::Hover
                                        | zaroxi_core_engine_ui::InteractionState::Active => {
                                            let mut c = tokens.editor_scrollbar_thumb.to_array();
                                            c[3] = (c[3] * 2.0).min(1.0);
                                            Some(c)
                                        }
                                        _ => None,
                                    };
                                    if let Some(color) = highlight_color {
                                        for block in &mut render_blocks {
                                            if block.id == "scrollbar_thumb_editor" {
                                                block.header_color = Some(color);
                                                break;
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }

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
                                block.cursor_line = Some(cursor_line);
                                block.cursor_col = Some(cursor_col);
                                block.selection_range = selection_range;
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
                                        let off_y =
                                            meta.editor_scroll_top_line as f32 * lc::LINE_HEIGHT;
                                        block.content_offset_y = off_y;
                                        if std::env::var("ZAROXI_DEBUG_SCROLL").as_deref()
                                            == Ok("1")
                                        {
                                            eprintln!(
                                                "ZAROXI_SCROLL: block content_offset x={:.1} y={:.1} top_line={}",
                                                block.content_offset_x,
                                                off_y,
                                                meta.editor_scroll_top_line
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
                                block.cursor_line = Some(cursor_line);
                                block.cursor_col = Some(cursor_col);
                                block.selection_range = selection_range;
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

                    // ── Per-frame content trace (ZAROXI_RENDER_TRACE=1) ──
                    if render_trace_enabled() {
                        let editor_body_hash = self
                            .work_content
                            .as_ref()
                            .and_then(|wc| wc.editor_body.as_ref())
                            .map(|cv| {
                                let mut h: u64 = 0;
                                for line in cv.lines.iter() {
                                    h = h.wrapping_mul(31).wrapping_add(line.len() as u64);
                                }
                                h
                            })
                            .unwrap_or(0);
                        let explorer_count = self
                            .work_content
                            .as_ref()
                            .map(|wc| wc.explorer_items.as_ref().map(|v| v.len()).unwrap_or(0))
                            .unwrap_or(0);
                        let mut rblock_hash: u64 = 0;
                        for blk in &render_blocks {
                            rblock_hash =
                                rblock_hash.wrapping_mul(31).wrapping_add(blk.id.len() as u64);
                            rblock_hash =
                                rblock_hash.wrapping_mul(31).wrapping_add(blk.content.len() as u64);
                            rblock_hash = rblock_hash
                                .wrapping_mul(31)
                                .wrapping_add((blk.rect.x * 100.0) as u64);
                            rblock_hash = rblock_hash
                                .wrapping_mul(31)
                                .wrapping_add((blk.rect.y * 100.0) as u64);
                        }
                        eprintln!(
                            "ZAROXI_RENDER_TRACE: app_frame frame={} work_hash={:016x} explorer_count={} rblocks={} rblock_hash={:016x}",
                            frame_id,
                            editor_body_hash,
                            explorer_count,
                            render_blocks.len(),
                            rblock_hash
                        );
                    }

                    // Create persistent RenderCore on first frame.
                    let core_exists = self.render_core.is_some();
                    if !core_exists {
                        let window_arc = z.window_arc();
                        let surface_size = winit::dpi::PhysicalSize::new(sw, sh);
                        match pollster::block_on(
                            zaroxi_core_engine_render::renderer::core::RenderCore::new(
                                window_arc,
                                clear_color,
                                surface_size,
                            ),
                        ) {
                            Ok(core) => {
                                if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                                    eprintln!(
                                        "ZAROXI_FRAMEFLOW: RenderCore created (size={}x{})",
                                        sw, sh
                                    );
                                }
                                self.render_core = Some(core);
                            }
                            Err(e) => {
                                eprintln!("GuiApp: failed to create RenderCore: {:?}", e);
                                return;
                            }
                        }
                    }

                    if let Some(ref mut core) = self.render_core {
                        let surface_size = winit::dpi::PhysicalSize::new(sw, sh);
                        match core.render_to_window(surface_size, &render_layout, &render_blocks) {
                            Ok(()) => {
                                self.needs_render = false;
                                if render_trace_enabled() {
                                    eprintln!(
                                        "ZAROXI_RENDER_TRACE: render_result frame={} ok",
                                        frame_id
                                    );
                                }
                                record_frame_presented();
                                if !self.first_render_shown {
                                    let _ = z.window().set_visible(true);
                                    self.first_render_shown = true;
                                    eprintln!("GuiApp: first full-renderer frame; window visible");
                                }
                            }
                            Err(e) => {
                                if render_trace_enabled() {
                                    eprintln!(
                                        "ZAROXI_RENDER_TRACE: render_result frame={} err={:?}",
                                        frame_id, e
                                    );
                                }
                                if std::env::var("ZAROXI_FRAMEFLOW").as_deref() == Ok("1") {
                                    eprintln!("ZAROXI_FRAMEFLOW: render_to_window error: {:?}", e);
                                }
                                // Keep needs_render=true and request another redraw
                                // so the frame is retried on the next opportunity.
                                self.needs_render = true;
                                let _ = z.window().request_redraw();
                            }
                        }
                    }

                    if std::env::var("ZAROXI_DEBUG_RENDER").as_deref() == Ok("1") {
                        eprintln!("...");
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
