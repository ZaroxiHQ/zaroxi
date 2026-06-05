use std::error::Error;

use zaroxi_interface_desktop::gui::{ShellFrame, Size};

fn main() -> Result<(), Box<dyn Error>> {
    let size = Size { width: 1354, height: 720 };
    let mut shell = ShellFrame::new(size, zaroxi_interface_theme::theme::ZaroxiTheme::Dark);

    // Populate workspace content from DesktopComposition.
    let comp = zaroxi_interface_desktop::DesktopComposition::new();
    let work = comp.build_work_content();

    // If compiled with the "gui_window" feature, attempt to open a native window.
    #[cfg(feature = "gui_window")]
    {
        match zaroxi_interface_desktop::gui::run_shell_window(
            shell.clone(),
            Some(work.clone()),
            Some(comp),
            None,
            None,
            None,
            None,
        ) {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("window init failed; falling back to transcript output: {}", e);
            }
        }
    }

    shell.work_content = Some(work);
    for line in shell.render_lines(None::<&zaroxi_interface_desktop::DesktopComposition>) {
        println!("{}", line);
    }

    Ok(())
}
