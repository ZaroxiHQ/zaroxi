//! Zaroxi Desktop - Tauri backend
//!
//! This module contains app-specific orchestration for the Zaroxi IDE.
//! Domain logic resides in the workspace crates (`crates/*`).

mod adapters;
mod app_state;
mod bootstrap;
mod commands;
mod events;
mod menu;
mod services;
mod windows;
mod zaroxi_infra_permissions;

use crate::services::workspace_service::WorkspaceService;
use std::sync::Arc;
use tauri::{Manager, RunEvent};

/// Main entry point for Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    #[cfg(debug_assertions)]
    {
        use tracing_subscriber::{EnvFilter, fmt, prelude::*};

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,workspace_service=debug,tauri=warn"));

        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(std::io::stdout))
            .with(filter)
            .init();

        tracing::info!("Tracing initialized for Zaroxi Desktop");
    }

    tauri::Builder::default()
        .setup(|app| {
            // No menu created - using custom UI menu only

            // Initialize and manage the workspace service
            let workspace_service = Arc::new(WorkspaceService::new());
            app.manage(workspace_service);

            // Initialize theme service
            let theme_service =
                crate::services::theme_service::ThemeService::new(app.handle().clone());

            // Apply initial theme before moving
            let theme_service_clone = theme_service.clone();
            tauri::async_runtime::spawn(async move {
                theme_service_clone.apply_theme().await;
            });

            // Manage the theme service
            app.manage(theme_service);

            // Initialize app state
            let app_state = crate::app_state::AppState::new();
            app.manage(app_state);

            // Get the main window and set it up
            let main_window = app.get_webview_window("main").expect("Failed to get main window");

            // Call our window setup function to ensure decorations are removed
            if let Err(e) = windows::setup_window(&main_window) {
                tracing::error!("Failed to setup window: {}", e);
            }

            // Build native menu (works on all platforms via Tauri)
            if let Err(e) = menu::build_menu(app.handle()) {
                tracing::error!("Failed to build native menu: {}", e);
            }

            tracing::info!("Zaroxi Desktop app setup complete");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::workspace::open_workspace,
            commands::workspace::list_directory,
            commands::workspace::get_workspace_tree,
            commands::workspace::open_file,
            commands::workspace::save_file_workspace,
            commands::workspace::open_file_dialog,
            commands::editor::open_document,
            commands::editor::get_visible_lines,
            commands::editor::apply_edit,
            commands::editor::save_document,
            commands::editor::get_line_count,
            commands::editor::get_styled_spans,
            commands::editor::highlight_document,
            commands::assistant::start_ai_task,
            commands::assistant::cancel_ai_task,
            commands::search::search_workspace,
            commands::preview::generate_preview,
            commands::zaroxi_infra_settings::load_settings,
            commands::zaroxi_infra_settings::save_settings,
            // Theme commands
            commands::zaroxi_infra_settings::load_theme_settings,
            commands::zaroxi_infra_settings::save_theme_settings,
            commands::zaroxi_infra_settings::get_current_theme,
            commands::zaroxi_infra_settings::set_theme,
        ])
        .on_window_event(|window, event| {
            windows::handle_window_event(window, event);
        })
        .build(tauri::generate_context!())?
        .run(|app_handle, event| match event {
            RunEvent::Ready => {
                tracing::info!("App is ready");

                // Initialize services after app is ready
                if let Err(e) = bootstrap::setup::on_app_ready(app_handle) {
                    tracing::error!("Failed to initialize app: {}", e);
                }

                // Start the workspace service
                // We can get the workspace service from app state
                // For now, we'll start it when needed
            }
            RunEvent::Exit => {
                tracing::info!("App is exiting");
                // Cleanup resources
                events::emitter::broadcast_shutdown(app_handle);
            }
            _ => {}
        });

    Ok(())
}
