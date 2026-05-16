 // Desktop UI surface: pure presenter/compositor for Phase 0.
 //
 // IMPORTANT: this module must not construct concrete adapters or import infrastructure.
 // It accepts application-level traits and drives the UI-level scenario only.

 use std::path::PathBuf;
 use std::sync::Arc;

 use crate as _; // avoid unused warning in skeleton

 // Import the port traits from the application layer only.
 use zaroxi_kernel_core::boot::BootConfig;
 use zaroxi_application_workspace::ports::{
     WorkspaceService, WorkspaceOpenCommand, AppCommand, CommandResult,
 };

 /// Pure interface entrypoint used by an outer composition binary to exercise the first slice.
 /// This function performs the UI-level scenario by calling only application contracts.
 pub async fn run_desktop_flow(workspace_service: Arc<dyn WorkspaceService>) -> Result<(), String> {
     // Build boot config (for the slice, a path may be provided)
     let boot = BootConfig { workspace_path: Some(PathBuf::from("./sample-workspace")) };

     // Ask the application service to open a workspace and create a session.
     let open_cmd = WorkspaceOpenCommand {
         path: boot.workspace_path.clone().unwrap_or_else(|| PathBuf::from(".")),
     };

     let session = workspace_service.open_workspace(open_cmd).await?;
     println!("Interface: opened workspace session: {}", session.session_id);

     // Open a single buffer (UI asks application to open a file)
     let buffer_id = workspace_service
         .open_buffer(session.session_id.clone(), PathBuf::from("main.rs"))
         .await?;
     println!("Interface: opened buffer id: {}", buffer_id);

     // Dispatch a simple AI "explain" command via the application service.
     let cmd = AppCommand::AiExplain { prompt: format!("Explain contents of buffer {}", buffer_id) };
     let result = workspace_service.dispatch_command(session.session_id.clone(), cmd).await?;
     println!("Interface: command result: {}", result.message);

     Ok(())
 }
