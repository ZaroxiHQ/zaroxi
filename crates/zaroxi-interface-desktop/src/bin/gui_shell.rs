use std::path::PathBuf;

use zaroxi_application_workspace::ports::{OpenBufferRequest, WorkspaceBootRequest};
use zaroxi_interface_desktop::DesktopComposition;
use zaroxi_interface_desktop::gui::{ShellFrame, Size, run_shell_window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (service_handle, view_handle) =
        zaroxi_application_bootstrap::create_in_memory_orchestrator();

    let mut composition = DesktopComposition::new();

    let boot_req = WorkspaceBootRequest { path: PathBuf::from("./sample-workspace") };
    let boot_res = pollster::block_on(service_handle.boot_workspace(boot_req))?;
    log::info!("gui_shell: booted session {}", boot_res.session.session_id);

    let _ = pollster::block_on(service_handle.open_buffer(OpenBufferRequest {
        session_id: boot_res.session.session_id.clone(),
        path: PathBuf::from("main.rs"),
    }))?;

    let _ = pollster::block_on(zaroxi_interface_desktop::actions::refresh_desktop(
        &mut composition,
        view_handle.clone(),
        boot_res.session.session_id.clone(),
        Some(boot_res.session.workspace_id),
        Some(service_handle.clone()),
    ));

    let work = composition.build_work_content();
    let size = Size { width: 1354, height: 720 };
    let shell = ShellFrame::new(size, zaroxi_interface_theme::theme::ZaroxiTheme::Dark);

    run_shell_window(
        shell,
        Some(work),
        Some(composition),
        Some(view_handle),
        Some(service_handle),
        Some(boot_res.session.session_id),
        Some(boot_res.session.workspace_id),
    )?;

    Ok(())
}
