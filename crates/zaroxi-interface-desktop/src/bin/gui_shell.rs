use std::sync::Arc;

use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_interface_desktop::folder_picker::{DynFolderPicker, NativeFolderPicker};
use zaroxi_interface_desktop::gui::{ShellFrame, Size, run_shell_window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (service_handle, view_handle) =
        zaroxi_application_bootstrap::create_in_memory_orchestrator();

    let composition = DesktopComposition::new();

    let work = composition.build_work_content();
    let size = Size { width: 1354, height: 720 };
    let shell = ShellFrame::new(size, zaroxi_interface_theme::theme::ZaroxiTheme::Dark);

    let folder_picker: DynFolderPicker = Arc::new(NativeFolderPicker);

    run_shell_window(
        shell,
        Some(work),
        Some(composition),
        Some(view_handle),
        Some(service_handle),
        None,
        None,
        Some(folder_picker),
    )?;

    Ok(())
}
