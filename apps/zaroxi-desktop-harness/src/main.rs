use std::sync::Arc;
use std::path::PathBuf;

use tokio;

use zaroxi_application_workspace::ports::{
    WorkspaceService, WorkspaceOpenCommand, AppCommand, CommandResult,
    WorkspaceSessionDTO,
};
use zaroxi_domain_workspace::ports::{WorkspaceRepository, WorkspaceOpenCommand as DomainOpenCmd};
use zaroxi_core_editor_buffer::ports::{BufferStore, BufferId};
use zaroxi_application_ai::ports::{AiClient};

// Infra adapters
use zaroxi_infrastructure_ai_mock;
use zaroxi_infrastructure_memory;

type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Small service implementation that lives in the outer composition crate for Phase 0.
/// It implements the application-owned WorkspaceService trait by delegating to infrastructure ports.
/// NOTE: For Phase 0 this lightweight implementation is acceptable; for Phase 1 this belongs
/// in the application-workspace crate proper.
struct SimpleWorkspaceService {
    repo: Arc<dyn WorkspaceRepository>,
    buffer_store: Arc<dyn BufferStore>,
    ai_client: Arc<dyn AiClient>,
}

impl SimpleWorkspaceService {
    fn new(
        repo: Arc<dyn WorkspaceRepository>,
        buffer_store: Arc<dyn BufferStore>,
        ai_client: Arc<dyn AiClient>,
    ) -> Self {
        Self { repo, buffer_store, ai_client }
    }
}

impl WorkspaceService for SimpleWorkspaceService {
    fn open_workspace(&self, cmd: WorkspaceOpenCommand) -> BoxFuture<'static, Result<WorkspaceSessionDTO, String>> {
        let repo = self.repo.clone();
        Box::pin(async move {
            // Translate application command into domain repo call and create a session DTO.
            let domain_cmd = DomainOpenCmd { path: cmd.path.clone() };
            let dto = repo.open_workspace(domain_cmd).await.map_err(|e| e.0)?;
            Ok(WorkspaceSessionDTO {
                session_id: dto.id.clone(),
                workspace_id: dto.id.clone(),
            })
        })
    }

    fn open_buffer(&self, _session_id: String, path: PathBuf) -> BoxFuture<'static, Result<String, String>> {
        let store = self.buffer_store.clone();
        Box::pin(async move {
            let id = store.open_buffer(path).await.map_err(|e| e.0)?;
            Ok(id.0)
        })
    }

    fn dispatch_command(&self, _session_id: String, cmd: AppCommand) -> BoxFuture<'static, Result<CommandResult, String>> {
        let ai = self.ai_client.clone();
        Box::pin(async move {
            match cmd {
                AppCommand::AiExplain { prompt } => {
                    let res = ai.request(prompt).await.map_err(|e| e.0)?;
                    Ok(CommandResult { message: res.text })
                }
                AppCommand::InsertText { buffer_id: _, offset: _, text: _ } => {
                    // Not implemented for Phase 0
                    Ok(CommandResult { message: "inserted (noop-sim)".to_string() })
                }
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    // Build concrete infra adapters
    let repo = zaroxi_infrastructure_memory::InMemoryWorkspaceRepo::new();
    let repo_dyn = zaroxi_infrastructure_memory::into_workspace_repo(repo);

    let buffer_store = zaroxi_infrastructure_memory::InMemoryBufferStore::new();
    let buffer_dyn = zaroxi_infrastructure_memory::into_buffer_store(buffer_store);

    // AI mock
    let ai = zaroxi_infrastructure_ai_mock::MockAiClient::new();
    let ai_dyn = zaroxi_infrastructure_ai_mock::into_dyn(ai);

    // Compose the application service (for Phase 0 we construct it here).
    let service = SimpleWorkspaceService::new(repo_dyn, buffer_dyn, ai_dyn);

    // Run the harness flow directly (composition root) — uses application service directly.
    let open_cmd = WorkspaceOpenCommand { path: PathBuf::from("./sample-workspace") };
    let session = service.open_workspace(open_cmd).await?;
    println!("Harness: opened workspace session: {}", session.session_id);

    let buffer_id = service.open_buffer(session.session_id.clone(), PathBuf::from("main.rs")).await?;
    println!("Harness: opened buffer id: {}", buffer_id);

    let cmd = AppCommand::AiExplain { prompt: format!("Explain contents of buffer {}", buffer_id) };
    let result = service.dispatch_command(session.session_id.clone(), cmd).await?;
    println!("Harness: command result: {}", result.message);

    Ok(())
}
