use std::error::Error;

use zaroxi_interface_desktop::gui::{ShellFrame, Size};

fn main() -> Result<(), Box<dyn Error>> {
    let size = Size { width: 1354, height: 720 };
    let mut shell = ShellFrame::new(size);

    // Build live workspace content from DesktopComposition.
    // In a production harness, DesktopComposition would be refreshed with a real
    // workspace view and service; here we use an empty composition which produces
    // minimal content (terminals tabs only). The architecture wire is correct —
    // any caller can inject a fully populated ShellWorkContent.
    let comp = zaroxi_interface_desktop::DesktopComposition::new();
    let work = comp.build_work_content();

    // If compiled with the "gui_window" feature, attempt to open a native window.
    // The work_content is threaded through so the GPU path renders live data.
    #[cfg(feature = "gui_window")]
    {
        match zaroxi_interface_desktop::gui::run_shell_window(shell.clone(), Some(work.clone())) {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("window init failed; falling back to transcript output: {}", e);
            }
        }
    }

    // Default / fallback path: print deterministic transcript to stdout.
    shell.work_content = Some(work);
    for line in shell.render_lines(Some(&comp)) {
        println!("{}", line);
    }

    Ok(())
}
