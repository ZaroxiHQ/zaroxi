/*!
Bootstrap and public runner for the GUI window.
This file contains run_shell_window which creates the EventLoop, attributes,
instantiates the GuiApp and hands it to run_app.

Phase 4: accepts optional ShellWorkContent so live DesktopComposition
data flows through the native GPU window path.
*/

use crate::gui::ShellFrame;
use crate::gui::ShellWorkContent;
use std::error::Error;
use winit::{dpi::PhysicalSize, event_loop::EventLoop, window::WindowAttributes};

/// Public runner: open a native window and run a basic winit event loop.
///
/// `work_content` carries live editor/explorer/terminal content built from
/// DesktopComposition. When `Some`, the GPU window renders real session data;
/// when `None`, panels render with placeholder content.
///
/// This function will start the event loop and (on supported platforms) will
/// not return. It returns Err only if the window cannot be created so callers
/// may fall back to the transcript output in that case.
pub fn run_shell_window(
    shell: ShellFrame,
    work_content: Option<ShellWorkContent>,
) -> Result<(), Box<dyn Error>> {
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(err) => {
            eprintln!("EventLoop::new() failed: {}. Falling back to transcript mode.", err);
            return Err(Box::new(err));
        }
    };

    let window_attributes = WindowAttributes::default()
        .with_title("Zaroxi - GUI Shell")
        .with_inner_size(PhysicalSize::new(shell.size.width, shell.size.height))
        .with_resizable(true);

    let title = format!("Zaroxi - GUI Shell ({:?}x{:?})", shell.size.width, shell.size.height);

    let mut app = super::app::GuiApp {
        window_attributes: window_attributes.clone(),
        title,
        maybe_window: None,
        shell: shell.clone(),
        work_content: work_content.clone(),
        requested_initial_frame: false,
        already_logged_existing: false,
        first_render_shown: false,
        widget_tree: None,
        hovered_widget_idx: None,
        cursor_pos: None,
        scrollbar_drag: None,
        pressed_widget_idx: None,
        editor_scroll_offset: 0.0,
        terminal_scroll_offset: 0.0,
        editor_cursor_line: 7,
        editor_cursor_col: 4,
        selection_anchor: None,
        use_light_theme: false,
    };

    let run_result = event_loop.run_app(&mut app);

    match run_result {
        Ok(()) => Ok(()),
        Err(e) => Err(Box::new(e)),
    }
}
