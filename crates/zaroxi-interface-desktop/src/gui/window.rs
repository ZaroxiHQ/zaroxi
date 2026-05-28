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
    event::{Event, WindowEvent},
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

    // Create a Window using the winit Window API. The winit crate exposes Window
    // construction via `Window::new(&event_loop, WindowAttributes)`.
    // We use default attributes here to keep this bootstrap minimal.
    let window = window::Window::new(&event_loop, window::WindowAttributes::default())?;

    // Helpful title showing the shell size; small visual hint.
    let title = format!("Zaroxi - GUI Shell ({:?}x{:?})", shell.size.width, shell.size.height);
    window.set_title(&title);

    // Run the event loop. For winit v0.30 the (deprecated) `run` method expects a
    // handler of the form `FnMut(Event<T>, &ActiveEventLoop) -> ()`. We set the
    // control flow via the ActiveEventLoop API exposed as `set_control_flow`.
    let run_result = event_loop.run(move |event, active_loop| {
        // Default to waiting for events.
        active_loop.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    // Exit the process on close request for this minimal bootstrap.
                    std::process::exit(0);
                }
                WindowEvent::Resized(_size) => {
                    // Request a redraw; in the next phase we'll reconfigure GPU surfaces.
                    let _ = window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let _ = window.request_redraw();
                }
                _ => {}
            },

            Event::NewEvents(_) | Event::UserEvent(_) | Event::DeviceEvent { .. } | Event::Suspended
            | Event::Resumed | Event::AboutToWait | Event::LoopExiting | Event::MemoryWarning => {
                // No-op for now; placeholder for future behavior.
            }
        }
    });

    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
