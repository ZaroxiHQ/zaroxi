// Composition root skeleton for Phase 0 / Phase 1.
//
// This module wires the minimal services together using the ports defined in application/domain/core.
// It runs a tiny scenario:
//  - open workspace
//  - open a buffer
//  - dispatch an AI explain command through application-ai (mocked infra)
//  - print the AI response

use std::path::PathBuf;
use std::sync::Arc;

use crate as _; // avoid unused warning in skeleton

// Import the port traits. In the real repo these paths should be the crate names.
use crates::zaroxi_kernel_core::boot::{BootConfig, Boot};
use crates::zaroxi_application_workspace::ports::{WorkspaceService, WorkspaceOpenCommand};
use crates::zaroxi_application_ai::ports::{AiClient};

/// Composition entrypoint used by a binary (or tests) to validate the first slice.
/// Keep this function small; the real application will register actual implementations.
pub async fn compose_and_run() -> Result<(), String> {
    // Build boot config (for the slice, a path may be provided)
    let boot = BootConfig { workspace_path: Some(PathBuf::from("./sample-workspace")) };

    // Create mock AI adapter. In real code, this is resolved from infra crate.
    // The infra mock crate exported into_dyn helper.
    let ai_client = {
        // Safe placeholder: the actual module paths will differ in the workspace.
        // This skeleton assumes the mock crate is available and provides into_dyn.
        let mock = crates::zaroxi_infrastructure_ai_mock::MockAiClient::new();
        std::sync::Arc::new(mock) as Arc<dyn AiClient>
    };

    // TODO: Create implementations for WorkspaceRepository, BufferStore, and WorkspaceService.
    // For the skeleton we log intended actions and demonstrate how to call the AiClient.

    // Demonstrate an AI request flow:
    let prompt = "Explain why composition roots matter.".to_string();
    let response = ai_client.request(prompt).await.map_err(|e| e.0)?;
    println!("AI response: {}", response.text);

    // On success complete.
    Ok(())
}
