/*!
Bootstrap and public runner for the GUI window.
This file contains run_shell_window which creates the EventLoop, attributes,
instantiates the GuiApp and hands it to run_app.
*/

use std::error::Error;
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoop,
    window::WindowAttributes,
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

    // Instantiate the app and hand it to run_app.
    let mut app = super::app::GuiApp {
        window_attributes: window_attributes.clone(),
        title,
        maybe_window: None,
        bg_color: super::theme_adapter::parse_hex_color(shell.theme.surface),
        shell: shell.clone(),
        requested_initial_frame: false,
        already_logged_existing: false,
    };

    let run_result = event_loop.run_app(&mut app);

    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
