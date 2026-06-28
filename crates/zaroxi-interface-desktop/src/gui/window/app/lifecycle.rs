/*!
Winit `ApplicationHandler` lifecycle for [`GuiApp`]: window creation,
background-work polling / frame pacing (`about_to_wait`), and top-level
`window_event` dispatch. Heavy arms delegate immediately to
`on_mouse_left` (navigation) and `on_redraw_requested` (redraw).
*/

use super::*;
use std::time::Instant;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, StartCause, WindowEvent},
    event_loop::ControlFlow,
};

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
                    zaroxi_w.window().set_outer_position(PhysicalPosition::new(100, 100));
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

                    if zaroxi_core_telemetry::startup_trace_enabled() {
                        eprintln!(
                            "MEM_STARTUP: after_first_window rss={:.1}MB",
                            zaroxi_core_telemetry::rss_mb()
                        );
                    }

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

        // Apply any completed background parse result; this may invalidate the
        // UI so freshly parsed highlight spans become visible.
        self.poll_parse_results();
        // Commit a completed background-open rope (winning token only); this
        // invalidates the UI so the freshly materialized buffer paints.
        self.poll_open_results();
        // Commit a completed off-thread read (winning token only); this issues a
        // `request_open` which invalidates and schedules the next frame.
        self.poll_read_results();

        if self.requested_initial_frame {
            self.invalidate(InvalidationFlags::content());
            self.requested_initial_frame = false;
        }

        let now = Instant::now();

        if self.needs_render || self.interaction.scrollbar_drag_active() {
            // A frame is pending. Honour the pacing budget: issue the redraw now
            // if the budget has elapsed, otherwise park until the deadline. No
            // busy spinning — the loop sleeps until there is real work.
            if self.frame_scheduler.budget_elapsed(now) {
                self.schedule_redraw();
                active_loop.set_control_flow(ControlFlow::Wait);
            } else {
                active_loop.set_control_flow(ControlFlow::WaitUntil(
                    self.frame_scheduler.next_deadline(now),
                ));
            }
        } else if self.picker_in_flight
            || self.parse_result_pending()
            || self.background_open_pending
            || self.read_pending
        {
            // Background work is in flight; poll on a relaxed cadence so the
            // result is applied promptly without pinning a CPU core.
            active_loop.set_control_flow(ControlFlow::WaitUntil(now + BACKGROUND_POLL_INTERVAL));
        } else if self.explorer_search_active {
            // Search box focused and otherwise idle: blink the caret by waking
            // at each toggle and arming a repaint. Bounded to the focused state,
            // so it never affects frame pacing or background polling elsewhere.
            self.invalidate(InvalidationFlags::content());
            active_loop.set_control_flow(ControlFlow::WaitUntil(now + CARET_BLINK_INTERVAL));
        } else {
            active_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn window_event(
        &mut self,
        active_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
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
                if let Some(z) = self.maybe_window.as_mut() {
                    z.update_size(size.width, size.height);
                    debug::gui_debug_fmt!("GuiApp: Resized -> {size:?}, invalidating");
                }
                if self.startup_geometry_initial.is_none() {
                    self.startup_geometry_initial = Some((size.width, size.height));
                    self.startup_geometry_changed_reason =
                        Some("compositor_resize_before_first_paint".to_string());
                } else {
                    self.startup_geometry_final = Some((size.width, size.height));
                }
                self.resize_pending = true;
                self.invalidate(InvalidationFlags::resize());
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                debug::gui_debug("GuiApp: ScaleFactorChanged -> invalidating");
                self.resize_pending = true;
                self.invalidate(InvalidationFlags::resize());
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.interaction.set_cursor_pos(position.x as f32, position.y as f32);
                debug::click_trace_fmt!(
                    "ZAROXI_CLICK: CursorMoved x={:.1} y={:.1} widget_tree={}",
                    position.x,
                    position.y,
                    self.widget_tree.is_some()
                );

                // Rail hover detection (cockpit-owned surface, not in shell tree).
                {
                    let px = position.x as f32;
                    let py = position.y as f32;
                    let mut hit_idx = None;
                    for (i, &(rx, ry, rw, rh)) in self.rail_item_hit_rects.iter().enumerate() {
                        if px >= rx && px < rx + rw && py >= ry && py < ry + rh {
                            hit_idx = Some(i);
                            break;
                        }
                    }
                    if hit_idx != self.rail_hovered_index {
                        self.rail_hovered_index = hit_idx;
                        if self.settings_dropdown.open_row.is_none() {
                            self.cockpit_status_fingerprint = 0;
                        }
                        self.needs_render = true;
                    }
                }

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
                let ev_start = std::time::Instant::now();
                input::process_mouse_wheel(self, &delta);
                perf_event("scroll", ev_start, "");
            }
            WindowEvent::MouseInput { state, button, .. } if button == MouseButton::Left => {
                self.on_mouse_left(state);
            }
            WindowEvent::RedrawRequested => {
                self.on_redraw_requested();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
                self.ctrl_held = modifiers.state().control_key();
                self.cmd_held = modifiers.state().super_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let ev_kind = input::classify_editor_key(self, &event.logical_key);
                let ev_start = std::time::Instant::now();
                let actions = input::handle_keyboard_press(self, &event.logical_key);
                if let Some(kind) = ev_kind {
                    perf_event(
                        kind,
                        ev_start,
                        &format!(
                            "ln={} col={}",
                            self.editor_cursor_line(),
                            self.editor_cursor_col()
                        ),
                    );
                }
                self.handle_actions(actions);
            }
            _ => {}
        }
    }
}
