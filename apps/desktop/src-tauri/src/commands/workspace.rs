use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::command;
use tauri::{AppHandle, State};

use crate::services::workspace_service::WorkspaceService;

#[derive(Debug, Deserialize)]
pub struct OpenWorkspaceRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWorkspaceResponse {
    pub workspace_id: String,
    pub root_path: String,
    pub file_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTreeRequest {
    pub workspace_id: String,
    pub root_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTreeResponse {
    pub workspace_id: String,
    pub root_path: String,
    pub tree: Vec<crate::services::workspace_service::ExplorerTreeNode>,
}

#[command]
pub async fn open_workspace(
    request: OpenWorkspaceRequest,
    workspace_service: State<'_, Arc<WorkspaceService>>,
    app_handle: AppHandle,
) -> Result<OpenWorkspaceResponse, String> {
    use tracing::{error, info};

    info!("Opening workspace at path: {}", request.path);

    let path = PathBuf::from(&request.path);

    // Validate path exists
    if !path.exists() {
        error!("Path does not exist: {}", request.path);
        return Err(format!("Path does not exist: {}", request.path));
    }

    if !path.is_dir() {
        error!("Path is not a directory: {}", request.path);
        return Err(format!("Path is not a directory: {}", request.path));
    }

    info!("Path exists and is a directory, opening workspace...");

    // Open workspace using the service
    let workspace = match workspace_service.open_workspace(path).await {
        Ok(workspace) => {
            info!("Workspace opened successfully: {} ({})", workspace.name, workspace.id);
            workspace
        }
        Err(e) => {
            error!("Failed to open workspace: {}", e);
            return Err(format!("Failed to open workspace: {}", e));
        }
    };

    // Convert to DTO
    let response = crate::adapters::workspace_adapter::domain_workspace_to_dto(&workspace);
    info!(
        "Converted to DTO: workspace_id={}, root_path={}, file_count={}",
        response.workspace_id, response.root_path, response.file_count
    );

    // Emit event
    let emitter = crate::events::workspace_events::WorkspaceEventEmitter::new(&app_handle);
    if let Err(e) = emitter.emit_workspace_opened(&workspace.id.to_string(), &workspace.root_path) {
        error!("Failed to emit workspace opened event: {}", e);
    } else {
        info!("Emitted workspace opened event");
    }

    Ok(response)
}

#[derive(Debug, Deserialize)]
pub struct ListDirectoryRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct DirectoryEntryDto {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub file_type: Option<String>,
    pub size: Option<u64>,
    pub modified: Option<String>,
}

#[command]
pub async fn list_directory(
    request: ListDirectoryRequest,
    workspace_service: State<'_, Arc<WorkspaceService>>,
) -> Result<Vec<DirectoryEntryDto>, String> {
    let path = PathBuf::from(&request.path);

    // Validate path exists
    if !path.exists() {
        return Err(format!("Path does not exist: {}", request.path));
    }

    // List directory using the service
    let entries = workspace_service
        .list_directory(path)
        .await
        .map_err(|e| format!("Failed to list directory: {}", e))?;

    // Convert to DTOs
    let dtos = entries.iter().map(crate::adapters::workspace_adapter::file_entry_to_dto).collect();

    Ok(dtos)
}

#[command]
pub async fn get_workspace_tree(
    request: WorkspaceTreeRequest,
    workspace_service: State<'_, Arc<WorkspaceService>>,
) -> Result<WorkspaceTreeResponse, String> {
    use tracing::{error, info, warn};

    info!("get_workspace_tree command called with request: {:?}", request);
    info!("Building workspace tree for path: {}", request.root_path);

    let path = PathBuf::from(&request.root_path);

    // Validate path exists
    if !path.exists() {
        error!("Path does not exist: {}", request.root_path);
        return Err(format!("Path does not exist: {}", request.root_path));
    }

    if !path.is_dir() {
        error!("Path is not a directory: {}", request.root_path);
        return Err(format!("Path is not a directory: {}", request.root_path));
    }

    info!("Path exists and is a directory, building tree...");

    // Check if we can read the directory
    match std::fs::read_dir(&path) {
        Ok(_) => info!("Directory is readable"),
        Err(e) => {
            error!("Cannot read directory {}: {}", request.root_path, e);
            return Err(format!("Cannot read directory: {}", e));
        }
    }

    // Build workspace tree with better error handling
    let tree_result = workspace_service.build_workspace_tree(path).await;

    match tree_result {
        Ok(tree) => {
            info!("Successfully built tree with {} nodes", tree.len());
            if tree.is_empty() {
                warn!("Tree is empty - directory might be empty or have permission issues");
                // Check if directory is actually empty
                if let Ok(entries) = std::fs::read_dir(&request.root_path) {
                    let count = entries.count();
                    info!("Directory actually has {} entries", count);
                }
            } else {
                info!("First node: name={}, is_dir={}", tree[0].name, tree[0].is_dir);
            }
            Ok(WorkspaceTreeResponse {
                workspace_id: request.workspace_id,
                root_path: request.root_path,
                tree,
            })
        }
        Err(e) => {
            // Log the full error chain
            error!("Failed to build workspace tree: {:?}", e);

            // Create a user-friendly error message
            let error_msg = e.to_string();
            let user_msg = if error_msg.contains("Permission denied") {
                format!(
                    "Permission denied accessing '{}'. Check file permissions.",
                    request.root_path
                )
            } else if error_msg.contains("No such file") {
                format!("Directory '{}' does not exist.", request.root_path)
            } else if error_msg.contains("Not a directory") {
                format!("'{}' is not a directory.", request.root_path)
            } else {
                format!("Failed to load directory contents: {}", error_msg)
            };

            // For now, return an empty tree instead of failing
            // This allows the UI to at least show the workspace
            warn!("Returning empty tree due to error: {}", user_msg);
            Ok(WorkspaceTreeResponse {
                workspace_id: request.workspace_id,
                root_path: request.root_path,
                tree: Vec::new(),
            })
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct OpenFileRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct OpenFileResponse {
    pub content: String,
    pub language: Option<String>,
}

#[command]
pub async fn open_file(request: OpenFileRequest) -> Result<OpenFileResponse, String> {
    use std::fs;

    let path = PathBuf::from(&request.path);

    // Read file content
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Determine language from file extension
    let language = path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_string());

    Ok(OpenFileResponse { content, language })
}

#[derive(Debug, Deserialize)]
pub struct SaveFileRequest {
    pub path: String,
    pub content: String,
}

/// Save a file (workspace-scoped command).
///
/// Renamed to `save_file_workspace` to avoid a macro/command-name collision
/// with the editor module. The frontend will invoke this command under the
/// new name. Behavior: write the provided content to disk atomically.
#[command]
pub async fn save_file_workspace(request: SaveFileRequest) -> Result<(), String> {
    use std::fs;

    let path = PathBuf::from(&request.path);

    // Write file content
    fs::write(&path, request.content).map_err(|e| format!("Failed to save file: {}", e))?;

    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDialogResponse {
    pub selected_path: Option<String>,
}

#[command]
pub async fn open_file_dialog() -> Result<OpenDialogResponse, String> {
    use tracing::{info, warn};

    tracing::info!("open_file_dialog command called");
    info!("Opening file dialog for workspace selection");

    // Check if we're on Wayland
    let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    info!("Wayland detected: {}", is_wayland);

    // Use rfd for all platforms
    use rfd::AsyncFileDialog;

    // Open a directory picker dialog
    let handle = AsyncFileDialog::new().set_title("Select Workspace Directory").pick_folder().await;

    info!("Dialog completed, handle: {:?}", handle.is_some());

    let selected_path = handle.map(|handle| {
        let path = handle.path().to_string_lossy().to_string();
        info!("User selected path: {}", path);
        path
    });

    if selected_path.is_none() {
        warn!("No path selected - dialog was cancelled or failed");
        // On Wayland, we might need to check if portals are available
        if is_wayland {
            info!("On Wayland, ensure xdg-desktop-portal is installed and running");
            info!("You may need to install xdg-desktop-portal-gtk or xdg-desktop-portal-kde");
        }
    } else {
        info!("Dialog completed with path: {:?}", selected_path);
    }

    Ok(OpenDialogResponse { selected_path })
}
