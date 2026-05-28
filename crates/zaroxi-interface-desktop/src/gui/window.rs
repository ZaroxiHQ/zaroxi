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
    event_loop::ControlFlow,
};

use crate::gui::ShellFrame;

/// Public runner: open a native window and run a basic winit event loop.
///
/// This function will start the event loop and (on supported platforms) will
/// not return. It returns Err only if the window cannot be created so callers
/// may fall back to the transcript output in that case.
pub fn run_shell_window(shell: ShellFrame) -> Result<(), Box<dyn Error>> {
    // Create the event loop explicitly from the winit path to avoid any local
    // name resolution ambiguity. EventLoop::new() returns an EventLoop directly,
    // so construct it here and keep the error path for window building below.
    let event_loop = winit::event_loop::EventLoop::new();

    // Build the window using the fully-qualified path to ensure the import
    // resolution does not conflict with local modules named `window`.
    let window = winit::window::WindowBuilder::new()
        .with_title("Zaroxi - GUI Shell")
        .with_inner_size(PhysicalSize::new(shell.size.width, shell.size.height))
        .with_resizable(true)
        .build(&event_loop)?;

    // Put a helpful title showing the shell size; this is a tiny visual hint.
    let title = format!("Zaroxi - GUI Shell ({:?}x{:?})", shell.size.width, shell.size.height);
    window.set_title(&title);

    // Run the event loop and convert winit's run result into a Box<dyn Error> when needed.
    let run_result = event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_size) => {
                    // In a future patch we'll rebuild GPU surfaces / vertex data here.
                    window.request_redraw();
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    window.request_redraw();
                }
                _ => {}
            },

            Event::RedrawRequested(_) => {
                // No GPU rendering yet — placeholder for future draw code.
            }

            Event::MainEventsCleared => {
                // Drive a modest refresh rate for simple animations / updates later.
                window.request_redraw();
            }

            _ => {}
        }
    });

    // `event_loop.run` returns `Result<(), EventLoopError>` in the pinned winit version;
    // convert that into the function's Result type.
    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
