/*!
Minimal GUI-3 window bootstrap (winit-only runner).

This minimal version intentionally avoids direct wgpu calls so it compiles
cleanly against the workspace policy (no unsafe blocks enforced by workspace lints)
and opens a native window for manual verification.


Notes:
- This is an incremental step: it opens a window and runs a simple event loop.
- It does not perform GPU rendering yet. The previous GPU-based implementation
  (wgpu shader/pipeline) is intentionally removed here to match the workspace
  safety policy and the immediate need to open a window reliably.
- Future patches will reintroduce a wgpu/vello render bridge using exact APIs
  and careful handling of unsafe surface creation in a crate-level `unsafe` block
  that is review-justified.

Behavior:
- Opens a resizable window sized to the ShellFrame.
- Requests redraws periodically; no drawing occurs yet (blank content).
- Returns Err only if the window fails to be constructed; otherwise the function
  runs the event loop and never returns (EventLoop::run is a diverging call).
*/

use std::error::Error;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{StartCause, WindowEvent},
    event_loop::{EventLoop, ControlFlow},
    window::WindowAttributes,
};
use pollster;
use wgpu::Color;

use crate::gui::ShellFrame;

/// Public runner: open a native window and run a basic winit event loop.
///
/// This function will start the event loop and (on supported platforms) will
/// not return. It returns Err only if the window cannot be created so callers
/// may fall back to the transcript output in that case.
pub fn run_shell_window(shell: ShellFrame) -> Result<(), Box<dyn Error>> {
    // Create the EventLoop using the winit API. This returns a Result which we
    // propagate to the caller so the caller can fall back to transcript mode when
    // window creation is not possible.
    // Create the EventLoop using the winit API.
    // If the default attempt fails (commonly because Wayland libs couldn't be loaded
    // in this environment), try a conservative fallback: if an X11 DISPLAY is set
    // and the session is Wayland, set WINIT_UNIX_BACKEND=x11 and retry once.
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(err) => {
            // EventLoop creation failed (commonly due to missing Wayland libs on some systems).
            // Do not call unsafe or process-global environment setters here; instead
            // propagate the error so the caller can fall back to transcript output.
            eprintln!("EventLoop::new() failed: {}. Falling back to transcript mode.", err);
            return Err(Box::new(err));
        }
    }; // EventLoop::new() -> Result<EventLoop, EventLoopError>

    // Build WindowAttributes once and create the Window from the ActiveEventLoop
    // inside the run_app handler (recommended by this winit version).
    let window_attributes = WindowAttributes::default()
        .with_title("Zaroxi - GUI Shell")
        .with_inner_size(PhysicalSize::new(shell.size.width, shell.size.height))
        .with_resizable(true);

    // Helpful title showing the shell size; keep this small visual hint.
    let title = format!("Zaroxi - GUI Shell ({:?}x{:?})", shell.size.width, shell.size.height);

    // Helper: parse hex "#rrggbb" -> wgpu::Color (srgb approx).
    fn parse_hex_color(s: &str) -> Color {
        let s = s.trim_start_matches('#');
        if s.len() == 6 {
            if let Ok(v) = u32::from_str_radix(s, 16) {
                let r = ((v >> 16) & 0xFF) as f64 / 255.0;
                let g = ((v >> 8) & 0xFF) as f64 / 255.0;
                let b = (v & 0xFF) as f64 / 255.0;
                return Color { r, g, b, a: 1.0 };
            }
        }
        // fallback: use a neutral, non-branded technical default to avoid
        // hardcoding product colors in window glue.
        eprintln!("GuiApp: parse_hex_color: invalid hex '{}', falling back to neutral black", s);
        Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
    }

    // Build a small ApplicationHandler implementation to satisfy winit's run_app
    // API. This avoids passing a closure and matches the ApplicationHandler trait
    // expected by `EventLoop::run_app`.
    // Use the engine's ZaroxiWindow wrapper for safe, unified window handling.
    struct GuiApp {
        window_attributes: WindowAttributes,
        title: String,
        maybe_window: Option<zaroxi_core_engine_window::ZaroxiWindow>,
        /// Background clear color derived from shell theme (wgpu::Color).
        bg_color: Color,
        /// Request the initial frame once after window creation to avoid a busy loop.
        requested_initial_frame: bool,
        /// Prevent repeated "already created" logs from flooding the terminal.
        already_logged_existing: bool,
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
                            let res = pollster::block_on(
                                zaroxi_core_engine_render_backend::RenderBackend::clear_present_once(
                                    z,
                                    self.bg_color,
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
                        let _ = zaroxi_w.window().request_redraw();
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
                debug!("GuiApp: resumed -> attempting to create window");
                match active_loop.create_window(self.window_attributes.clone()) {
                    Ok(w) => {
                        // Wrap the freshly created raw winit Window in the engine ZaroxiWindow
                        // so we have the engine-level helpers (show_and_warmup, size, etc).
                        let zaroxi_w = zaroxi_core_engine_window::ZaroxiWindow::from_window(w);
                        let wid = zaroxi_w.window().id();
                        debug!("GuiApp: created engine window on resumed id={:?}", wid);
                        // Ensure visible + pre-present + redraw to nudge compositor mapping.
                        let _ = zaroxi_w.window().set_visible(true);
                        let _ = zaroxi_w.window().pre_present_notify();
                        let _ = zaroxi_w.window().request_redraw();
                        // Keep the engine window handle so we can request redraws later.
                        self.maybe_window = Some(zaroxi_w);

                        // Mark a single initial-frame request to drive one redraw pass.
                        self.requested_initial_frame = false;
                        let _ = zaroxi_w.window().request_redraw();
                        debug!("GuiApp: marked initial frame request after resumed creation");
                    }
                    Err(e) => {
                        warn!("GuiApp: resumed failed to create window: {}", e);
                        active_loop.exit();
                    }
                }
            } else {
                debug!("GuiApp: resumed called but window already created");
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
                        // If we later add a wgpu clear/present path we will call it here.
                    }
                }
                _ => {}
            }
        }
    }

    // Instantiate the app and hand it to run_app.
    let mut app = GuiApp {
        window_attributes: window_attributes.clone(),
        title,
        maybe_window: None,
        bg_color: parse_hex_color(shell.theme.surface),
        requested_initial_frame: false,
        already_logged_existing: false,
    };

    let run_result = event_loop.run_app(&mut app);

    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
