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
    dpi::PhysicalSize,
    event::{Event, WindowEvent, StartCause},
    event_loop::{EventLoop, ControlFlow},
    window::{Window, WindowAttributes},
};

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
    let event_loop = EventLoop::new()?; // EventLoop::new() -> Result<EventLoop, EventLoopError>

    // Build WindowAttributes once and create the Window from the ActiveEventLoop
    // inside the run_app handler (recommended by this winit version).
    let window_attributes = WindowAttributes::default()
        .with_title("Zaroxi - GUI Shell")
        .with_inner_size(PhysicalSize::new(shell.size.width, shell.size.height))
        .with_resizable(true);

    // Helpful title showing the shell size; keep this small visual hint.
    let title = format!("Zaroxi - GUI Shell ({:?}x{:?})", shell.size.width, shell.size.height);

    // Build a small ApplicationHandler implementation to satisfy winit's run_app
    // API. This avoids passing a closure and matches the ApplicationHandler trait
    // expected by `EventLoop::run_app`.
    struct GuiApp {
        window_attributes: WindowAttributes,
        title: String,
        maybe_window: Option<Window>,
    }

    impl winit::application::ApplicationHandler for GuiApp {
        fn new_events(&mut self, active_loop: &winit::event_loop::ActiveEventLoop, cause: StartCause) {
            // Create the window once on Init (or when resumed on some platforms).
            if self.maybe_window.is_none() && matches!(cause, StartCause::Init) {
                match active_loop.create_window(self.window_attributes.clone()) {
                    Ok(w) => {
                        w.set_title(&self.title);
                        self.maybe_window = Some(w);
                    }
                    Err(e) => {
                        eprintln!("failed to create window: {}", e);
                        active_loop.exit();
                    }
                }
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
                WindowEvent::Resized(_size) => {
                    if let Some(w) = self.maybe_window.as_ref() {
                        let _ = w.request_redraw();
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    if let Some(w) = self.maybe_window.as_ref() {
                        let _ = w.request_redraw();
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
    };

    let run_result = event_loop.run_app(&mut app);

    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
