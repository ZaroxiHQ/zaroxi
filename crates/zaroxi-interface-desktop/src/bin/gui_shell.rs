use std::error::Error;

// Note: binaries in the same package can reference the library crate by its package name.
// The package crate name is expected to be `zaroxi_interface_desktop` (hyphens -> underscores).
use zaroxi_interface_desktop::gui::{ShellFrame, Size};

fn main() -> Result<(), Box<dyn Error>> {
    let size = Size { width: 1280, height: 800 };
    let shell = ShellFrame::new(size);

    for line in shell.render_lines() {
        println!("{}", line);
    }

    Ok(())
}
