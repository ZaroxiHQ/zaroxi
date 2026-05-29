/*!
GuiApp implementation and winit ApplicationHandler lifecycle methods.
This file contains the GuiApp struct and its ApplicationHandler impl
(moved out of the large `window.rs` to make the module tree clearer).
*/

use winit::{
    dpi::PhysicalPosition,
    event::{StartCause, WindowEvent},
    event_loop::{ControlFlow},
    window::WindowAttributes,
};
use pollster;

use crate::gui::ShellFrame;

/// Small application handler that owns the engine window handle and the ShellFrame
/// snapshot. Lifecycle methods handle window creation, the first-frame bootstrap
/// (clear+present) and redraw requests.
pub struct GuiApp {
    pub window_attributes: WindowAttributes,
    pub title: String,
    pub maybe_window: Option<zaroxi_core_engine_window::ZaroxiWindow>,
    /// Background clear color derived from shell theme (wgpu::Color).
    pub bg_color: wgpu::Color,
    /// Keep a clone of the ShellFrame so we can resolve stable region rects and theme tokens
    /// at window creation time and pass low-level draw inputs into the backend.
    pub shell: ShellFrame,
    /// Request the initial frame once after window creation to avoid a busy loop.
    pub requested_initial_frame: bool,
    /// Prevent repeated "already created" logs from flooding the terminal.
    pub already_logged_existing: bool,
}

impl winit::application::ApplicationHandler for GuiApp {
    fn new_events(
        &mut self,
        active_loop: &winit::event_loop::ActiveEventLoop,
        cause: StartCause,
    ) {
        // Create the window once on Init (or when resumed on some platforms).
        if self.maybe_window.is_none() && matches!(cause, StartCause::Init) {
            eprintln!("GuiApp: attempting to create window (StartCause::Init)");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    // Convert the raw winit Window into the engine wrapper and warm it up.
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    eprintln!("GuiApp: created engine window id={:?}", wid);
                    // Ensure a visible title is set (small visual hint).
                    zaroxi_w.window().set_title(&self.title);
                    // Try to place the window at a sane on-screen position.
                    let _ = zaroxi_w.window().set_outer_position(PhysicalPosition::new(100, 100));
                    // Warmup: visible + pre-present + redraw requests to nudge compositor mapping.
                    zaroxi_w.show_and_warmup();
                    // Keep the engine window handle so we can request redraws later.
                    self.maybe_window = Some(zaroxi_w);

                    // Perform a one-shot clear+present using the engine render backend
                    // to ensure the compositor receives a GPU-backed frame and maps the window.
                    if let Some(z) = self.maybe_window.as_ref() {
                        eprintln!("GuiApp: invoking clear_present_once to produce first GPU frame");

                        // Build a small set of resolved low-level rect draws from the shell regions.
                        // Delegate rect construction to the frame module so the code is easier to follow.
                        let rects = super::frame::build_overlay_rects(&self.shell);

                        // Path marker for diagnostics: clear_present_once (first-frame bootstrap).
                        eprintln!("GUI_TEXT_PATH=clear_present_once");
                        // Gather adapter/backend markers for a compact per-path summary (non-fatal).
                        let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
                        let tmp_cosmic = std::env::temp_dir().join("zaroxi_gui_trace_cosmic_prepare");
                        let layout_present = tmp_layout.exists();
                        let cosmic_present = tmp_cosmic.exists();
                        let mut adapter_text_ops: usize = 0;
                        if layout_present {
                            if let Ok(s) = std::fs::read_to_string(&tmp_layout) {
                                if let Some(rest) = s.strip_prefix("lines=") {
                                    adapter_text_ops = rest.split(" | ").filter(|p| !p.is_empty()).count();
                                }
                            }
                        }
                        let backend_text_ops = rects.len();
                        let fallback_used = layout_present && !cosmic_present;

                        eprintln!(
                            "GUI_TEXT_FRAME_SUMMARY: path=clear_present_once adapter_text_ops={} backend_text_ops={} core_text_ops=0 cosmic_prepare_called={} glyphs=0 atlas_entries=0 pipeline_render_called=false overlay_rects={} fallback_used={}",
                            adapter_text_ops,
                            backend_text_ops,
                            if cosmic_present { "true" } else { "false" },
                            backend_text_ops,
                            if fallback_used { "true" } else { "false" }
                        );

                        // Hard-checks for broken links (diagnostic only).
                        if adapter_text_ops > 0 && backend_text_ops == 0 {
                            eprintln!("GUI_TEXT_BROKEN_LINK: adapter->backend");
                        }
                        if backend_text_ops > 0 {
                            // At this bootstrap path the full renderer/core is not invoked,
                            // therefore core_text_ops will be zero here: this indicates the
                            // backend->core link was not exercised for the clear_present_once path.
                            eprintln!("GUI_TEXT_BROKEN_LINK: backend->core");
                        }

                        let res = pollster::block_on(
                            zaroxi_core_engine_render_backend::RenderBackend::clear_present_once(
                                z,
                                self.bg_color,
                                Some(&rects),
                            ),
                        );
                        if let Err(e) = res {
                            eprintln!("GuiApp: clear_present_once failed: {}", e);
                        } else {
                            eprintln!("GuiApp: clear_present_once succeeded");
                        }
                    }

                    // Ask for a single initial frame: request a redraw once and use Wait for steady-state.
                    self.requested_initial_frame = false;
                    if let Some(z) = self.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                    active_loop.set_control_flow(ControlFlow::Wait);
                    eprintln!("GuiApp: marked initial frame request (engine window) and set Wait");
                }
                Err(e) => {
                    eprintln!("GuiApp: failed to create window: {}", e);
                    // Ask the event loop to exit; caller will fall back to transcript.
                    active_loop.exit();
                }
            }
        } else if self.maybe_window.is_some() {
            // Already created; noop but only log once for diagnostics to avoid terminal bloat.
            if !self.already_logged_existing {
                eprintln!("GuiApp: new_events called but window already created");
                self.already_logged_existing = true;
            }
        } else {
            eprintln!("GuiApp: new_events called with cause={:?} (no creation)", cause);
        }
    }

    fn resumed(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        // Some Wayland compositors deliver readiness after `resumed`, not Init.
        // Attempt window creation here as a fallback when new_events didn't create it.
        if self.maybe_window.is_none() {
            eprintln!("GuiApp: resumed -> attempting to create window");
            match active_loop.create_window(self.window_attributes.clone()) {
                Ok(w) => {
                    // Wrap the freshly created raw winit Window in the engine ZaroxiWindow
                    // so we have the engine-level helpers (show_and_warmup, size, etc).
                    let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                    let wid = zaroxi_w.window().id();
                    eprintln!("GuiApp: created engine window on resumed id={:?}", wid);
                    // Ensure visible + pre-present + redraw to nudge compositor mapping.
                    let _ = zaroxi_w.window().set_visible(true);
                    let _ = zaroxi_w.window().pre_present_notify();
                    let _ = zaroxi_w.window().request_redraw();
                    // Keep the engine window handle so we can request redraws later.
                    self.maybe_window = Some(zaroxi_w);

                    // Perform a one-shot clear+present using the engine render backend
                    // to ensure the compositor receives a GPU-backed frame and maps the window.
                    if let Some(z) = self.maybe_window.as_ref() {
                        eprintln!("GuiApp: invoking clear_present_once to produce first GPU frame (resumed)");

                        // Delegate rect construction to the frame module (same policy as init path).
                        let rects = super::frame::build_overlay_rects(&self.shell);

                        // Path marker for diagnostics: clear_present_once invoked from resumed.
                        eprintln!("GUI_TEXT_PATH=clear_present_once");

                        // Gather adapter/backend markers for a compact per-path summary (non-fatal).
                        let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
                        let tmp_cosmic = std::env::temp_dir().join("zaroxi_gui_trace_cosmic_prepare");
                        let layout_present = tmp_layout.exists();
                        let cosmic_present = tmp_cosmic.exists();
                        let mut adapter_text_ops: usize = 0;
                        if layout_present {
                            if let Ok(s) = std::fs::read_to_string(&tmp_layout) {
                                if let Some(rest) = s.strip_prefix("lines=") {
                                    adapter_text_ops = rest.split(" | ").filter(|p| !p.is_empty()).count();
                                }
                            }
                        }
                        let backend_text_ops = rects.len();
                        let fallback_used = layout_present && !cosmic_present;

                        eprintln!(
                            "GUI_TEXT_FRAME_SUMMARY: path=clear_present_once adapter_text_ops={} backend_text_ops={} core_text_ops=0 cosmic_prepare_called={} glyphs=0 atlas_entries=0 pipeline_render_called=false overlay_rects={} fallback_used={}",
                            adapter_text_ops,
                            backend_text_ops,
                            if cosmic_present { "true" } else { "false" },
                            backend_text_ops,
                            if fallback_used { "true" } else { "false" }
                        );

                        // Hard-checks for broken links (diagnostic only).
                        if adapter_text_ops > 0 && backend_text_ops == 0 {
                            eprintln!("GUI_TEXT_BROKEN_LINK: adapter->backend");
                        }
                        if backend_text_ops > 0 {
                            // At this bootstrap path the full renderer/core is not invoked,
                            // therefore core_text_ops will be zero here: this indicates the
                            // backend->core link was not exercised for the clear_present_once path.
                            eprintln!("GUI_TEXT_BROKEN_LINK: backend->core");
                        }

                        let res = pollster::block_on(
                            zaroxi_core_engine_render_backend::RenderBackend::clear_present_once(
                                z,
                                self.bg_color,
                                Some(&rects),
                            ),
                        );
                        if let Err(e) = res {
                            eprintln!("GuiApp: clear_present_once failed (resumed): {}", e);
                        } else {
                            eprintln!("GuiApp: clear_present_once succeeded (resumed)");
                        }
                    }

                    // Mark a single initial-frame request to drive one redraw pass.
                    self.requested_initial_frame = false;
                    if let Some(z) = self.maybe_window.as_ref() {
                        let _ = z.window().request_redraw();
                    }
                    eprintln!("GuiApp: marked initial frame request after resumed creation");
                }
                Err(e) => {
                    eprintln!("GuiApp: resumed failed to create window: {}", e);
                    active_loop.exit();
                }
            }
        } else {
            eprintln!("GuiApp: resumed called but window already created");
        }
    }

    fn about_to_wait(&mut self, active_loop: &winit::event_loop::ActiveEventLoop) {
        // Request the initial frame once to avoid a continuous busy redraw loop.
        if self.requested_initial_frame {
            if let Some(z) = self.maybe_window.as_ref() {
                eprintln!("GuiApp: about_to_wait -> requesting initial redraw (engine window)");
                let _ = z.window().request_redraw();
            }
            self.requested_initial_frame = false;
            // After requesting the single initial frame, stop polling to avoid busy-looping.
            active_loop.set_control_flow(ControlFlow::Wait);
            eprintln!("GuiApp: about_to_wait -> switched control flow back to Wait");
        }
        // Otherwise remain idle (Wait) and let the platform wake us for real events.
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
            WindowEvent::Resized(_size) => {
                if let Some(z) = self.maybe_window.as_ref() {
                    eprintln!("GuiApp: Resized -> requesting redraw (engine window)");
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(z) = self.maybe_window.as_ref() {
                    eprintln!("GuiApp: ScaleFactorChanged -> requesting redraw (engine window)");
                    let _ = z.window().request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                eprintln!("GuiApp: RedrawRequested received");
                if let Some(z) = self.maybe_window.as_ref() {
                    eprintln!("GuiApp: performing present-related nudges (engine window)");
                    let _ = z.window().pre_present_notify();

                    // Diagnostic: produce a compact per-redraw summary so we can compare
                    // the clear_present_once bootstrap path with the normal redraw path.
                    eprintln!("GUI_TEXT_PATH=redraw_requested");
                    let rects = super::frame::build_overlay_rects(&self.shell);
                    let tmp_layout = std::env::temp_dir().join("zaroxi_gui_trace_layout");
                    let tmp_cosmic = std::env::temp_dir().join("zaroxi_gui_trace_cosmic_prepare");
                    let layout_present = tmp_layout.exists();
                    let cosmic_present = tmp_cosmic.exists();
                    let mut adapter_text_ops: usize = 0;
                    if layout_present {
                        if let Ok(s) = std::fs::read_to_string(&tmp_layout) {
                            if let Some(rest) = s.strip_prefix("lines=") {
                                adapter_text_ops = rest.split(" | ").filter(|p| !p.is_empty()).count();
                            }
                        }
                    }
                    let backend_text_ops = rects.len();
                    let core_text_ops: usize = 0; // full renderer not invoked here in this simplified path
                    let fallback_used = layout_present && !cosmic_present;

                    eprintln!(
                        "GUI_TEXT_FRAME_SUMMARY: path=redraw_requested adapter_text_ops={} backend_text_ops={} core_text_ops={} cosmic_prepare_called={} glyphs=0 atlas_entries=0 pipeline_render_called=false overlay_rects={} fallback_used={}",
                        adapter_text_ops,
                        backend_text_ops,
                        core_text_ops,
                        if cosmic_present { "true" } else { "false" },
                        backend_text_ops,
                        if fallback_used { "true" } else { "false" }
                    );

                    // Hard-checks for broken links (diagnostic only).
                    if adapter_text_ops > 0 && backend_text_ops == 0 {
                        eprintln!("GUI_TEXT_BROKEN_LINK: adapter->backend");
                    }
                    if backend_text_ops > 0 && core_text_ops == 0 {
                        eprintln!("GUI_TEXT_BROKEN_LINK: backend->core");
                    }

                    // If we later add a wgpu clear/present path we will call it here.
                }
            }
            _ => {}
        }
    }
}
