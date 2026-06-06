//! Application-layer workspace boot factory.
//!
//! Provides `create_in_memory_orchestrator()` which composes all the
//! in-memory adapters and returns `(WorkspaceService, WorkspaceView)` handles.
//! Callers (harness, gui_shell) never see infrastructure types — only
//! application-layer trait objects.

use std::sync::Arc;
use zaroxi_application_workspace::ports::{WorkspaceService, WorkspaceView};

pub fn create_in_memory_orchestrator() -> (Arc<dyn WorkspaceService>, Arc<dyn WorkspaceView>) {
    let repo = zaroxi_application_workspace::in_memory_adapters::InMemoryWorkspaceRepo::new();
    let repo_dyn = zaroxi_application_workspace::in_memory_adapters::into_workspace_repo(repo);
    let buffer_store = zaroxi_application_workspace::in_memory_adapters::InMemoryBufferStore::new();
    let buffer_dyn =
        zaroxi_application_workspace::in_memory_adapters::into_buffer_store(buffer_store);
    let history = zaroxi_infrastructure_memory::InMemoryHistoryStore::new();
    let history_dyn = zaroxi_infrastructure_memory::into_history_store(history);
    let ai = zaroxi_infrastructure_ai_mock::MockAiClient::new();
    let ai_dyn = zaroxi_infrastructure_ai_mock::into_dyn(ai);
    let checkpoint_store = zaroxi_infrastructure_memory::InMemoryCheckpointStore::new();
    let checkpoint_dyn = zaroxi_infrastructure_memory::into_checkpoint_store(checkpoint_store);

    let orchestrator =
        zaroxi_application_workspace::usecases::WorkspaceOrchestrator::new_with_history_and_durability(
            repo_dyn,
            buffer_dyn,
            ai_dyn,
            history_dyn,
            checkpoint_dyn,
        );
    let shared: Arc<zaroxi_application_workspace::usecases::WorkspaceOrchestrator> =
        Arc::new(orchestrator);
    let service: Arc<dyn WorkspaceService> = shared.clone();
    let view: Arc<dyn WorkspaceView> = shared;
    (service, view)
}
