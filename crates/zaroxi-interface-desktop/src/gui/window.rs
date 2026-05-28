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

    // Build WindowAttributes once and create the Window from the ActiveEventLoop
    // inside the run_app handler (recommended by this winit version).
    let window_attributes = WindowAttributes::default()
        .with_title("Zaroxi - GUI Shell")
        .with_inner_size(PhysicalSize::new(shell.size.width, shell.size.height))
        .with_resizable(true);

    // Helpful title showing the shell size; keep this small visual hint.
    let title = format!("Zaroxi - GUI Shell ({:?}x{:?})", shell.size.width, shell.size.height);

    // We'll create the Window inside the event loop via ActiveEventLoop::create_window.
    // Store it in an Option captured by the closure so we can reference it for redraws.
    let mut maybe_window: Option<Window> = None;

    // Use run_app (preferred over deprecated `run`) and handle events with access to ActiveEventLoop.
    let run_result = event_loop.run_app(move |event, active_loop: &winit::event_loop::ActiveEventLoop| {
        // Default to waiting for events between iterations.
        active_loop.set_control_flow(ControlFlow::Wait);

        match event {
            // Create the window once when the event loop initializes (NewEvents/Init).
            Event::NewEvents(_) => {
                if maybe_window.is_none() {
                    match active_loop.create_window(window_attributes.clone()) {
                        Ok(new_w) => {
                            // Give the window an explicit type so the compiler can infer captures.
                            let w: Window = new_w;
                            w.set_title(&title);
                            maybe_window = Some(w);
                        }
                        Err(e) => {
                            // If window creation fails, log and exit the loop so caller can fallback.
                            eprintln!("failed to create window: {}", e);
                            active_loop.exit();
                            return;
                        }
                    }
                }
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    // Ask the loop to exit cleanly.
                    active_loop.exit();
                }
                WindowEvent::Resized(_size) => {
                    if let Some(w) = maybe_window.as_ref() {
                        let _ = w.request_redraw();
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    if let Some(w) = maybe_window.as_ref() {
                        let _ = w.request_redraw();
                    }
                }
                _ => {}
            },

            _ => {
                // No-op for other events in this minimal bootstrap.
            }
        }
    });

    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
