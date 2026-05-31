use std::error::Error;

// Note: binaries in the same package can reference the library crate by its package name.
// The package crate name is expected to be `zaroxi_interface_desktop` (hyphens -> underscores).
use zaroxi_interface_desktop::gui::{ShellFrame, Size};

fn main() -> Result<(), Box<dyn Error>> {
    let size = Size { width: 1280, height: 800 };
    let shell = ShellFrame::new(size);

    // If compiled with the "gui_window" feature, attempt to open a native window.
    // Otherwise, fall back to deterministic transcript output.
    #[cfg(feature = "gui_window")]
    {
        match zaroxi_interface_desktop::gui::run_shell_window(shell.clone()) {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("window init failed; falling back to transcript output: {}", e);
            }
        }
    }

    // Default / fallback path: print deterministic transcript to stdout.
    // Create a DesktopComposition instance (empty) and pass it into render_lines so
    // the widgets can consume authoritative composition projections when available.
    let comp = zaroxi_interface_desktop::DesktopComposition::new();
    for line in shell.render_lines(Some(&comp)) {
        println!("{}", line);
    }

    Ok(())
}
