use std::error::Error;

// Note: binaries in the same package can reference the library crate by its package name.
// The package crate name is expected to be `zaroxi_interface_desktop` (hyphens -> underscores).
use zaroxi_interface_desktop::gui::{ShellFrame, Size};

fn main() -> Result<(), Box<dyn Error>> {
    let size = Size { width: 1280, height: 800 };
    let shell = ShellFrame::new(size);

    // Try to open a native window and render the shell. If that fails (headless CI,
    // missing GPU, etc.) fall back to printing the deterministic transcript.
    match zaroxi_interface_desktop::gui::window::run_shell_window(shell.clone()) {
        Ok(_) => {
            // Window runner handled lifecycle and exited normally.
            Ok(())
        }
        Err(e) => {
            eprintln!("window init failed; falling back to transcript output: {}", e);
            for line in shell.render_lines() {
                println!("{}", line);
            }
            Ok(())
        }
    }
}
