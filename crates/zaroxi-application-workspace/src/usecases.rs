use std::sync::Arc;

use crate::ports::{
    WorkspaceBootRequest, WorkspaceBootResponse, OpenBufferRequest, OpenBufferResponse,
    DispatchCommandRequest, DispatchCommandResponse, AppCommand, CommandResult, WorkspaceSessionDTO,
};

use zaroxi_domain_workspace::ports as domain_ports;
use zaroxi_core_editor_buffer::ports as buffer_ports;
use zaroxi_application_ai::ports as ai_ports;

/// Concrete orchestrator implementing application use-cases.
///
/// This struct belongs to the application layer. It composes domain and core ports,
/// delegating side-effects to adapters provided by the composition root.
pub struct WorkspaceOrchestrator {
    repo: Arc<dyn domain_ports::WorkspaceRepository>,
    buffer_store: Arc<dyn buffer_ports::BufferStore>,
    ai_client: Arc<dyn ai_ports::AiClient>,
}

impl WorkspaceOrchestrator {
    /// Create a new orchestrator with concrete port implementations (adapters).
    pub fn new(
        repo: Arc<dyn domain_ports::WorkspaceRepository>,
        buffer_store: Arc<dyn buffer_ports::BufferStore>,
        ai_client: Arc<dyn ai_ports::AiClient>,
    ) -> Self {
        Self { repo, buffer_store, ai_client }
    }
}

use crate::ports::BoxFuture;

impl crate::ports::WorkspaceService for WorkspaceOrchestrator {
    fn boot_workspace(&self, req: WorkspaceBootRequest) -> BoxFuture<'static, Result<WorkspaceBootResponse, String>> {
        let repo = self.repo.clone();
        Box::pin(async move {
            let domain_cmd = domain_ports::WorkspaceOpenCommand { path: req.path.clone() };
            let dto = repo.open_workspace(domain_cmd).await.map_err(|e| e.0)?;
            let session = WorkspaceSessionDTO { session_id: dto.id.clone(), workspace_id: dto.id.clone() };
            Ok(WorkspaceBootResponse { session })
        })
    }

    fn open_buffer(&self, req: OpenBufferRequest) -> BoxFuture<'static, Result<OpenBufferResponse, String>> {
        let store = self.buffer_store.clone();
        Box::pin(async move {
            let id = store.open_buffer(req.path.clone()).await.map_err(|e| e.0)?;
            Ok(OpenBufferResponse { buffer_id: id.0 })
        })
    }

    fn dispatch_command(&self, req: DispatchCommandRequest) -> BoxFuture<'static, Result<DispatchCommandResponse, String>> {
        let ai = self.ai_client.clone();
        Box::pin(async move {
            match req.command {
                AppCommand::AiExplain { prompt } => {
                    let res = ai.request(prompt).await.map_err(|e| e.0)?;
                    Ok(DispatchCommandResponse { result: CommandResult { message: res.text } })
                }
                AppCommand::InsertText { .. } => {
                    // Not implemented in Phase 1; return a successful no-op.
                    Ok(DispatchCommandResponse { result: CommandResult { message: "inserted (noop)".to_string() } })
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::WorkspaceService;
    use std::sync::Arc;
    use std::path::PathBuf;

    // Lightweight test doubles implementing the required ports.
    struct FakeRepo;
    impl domain_ports::WorkspaceRepository for FakeRepo {
        fn open_workspace(&self, cmd: domain_ports::WorkspaceOpenCommand) -> crate::ports::BoxFuture<'static, Result<domain_ports::WorkspaceDTO, domain_ports::DomainError>> {
            Box::pin(async move {
                Ok(domain_ports::WorkspaceDTO { id: "ws-test".to_string(), root_path: cmd.path.clone(), name: "TestWS".to_string() })
            })
        }
    }

    struct FakeBufferStore;
    impl buffer_ports::BufferStore for FakeBufferStore {
        fn open_buffer(&self, path: PathBuf) -> crate::ports::BoxFuture<'static, Result<buffer_ports::BufferId, buffer_ports::BufferError>> {
            Box::pin(async move {
                Ok(buffer_ports::BufferId(format!("buf:{}", path.to_string_lossy())))
            })
        }

        fn get_text(&self, _id: &buffer_ports::BufferId) -> Option<String> {
            Some("fn main() {}".to_string())
        }
    }

    struct FakeAi;
    impl ai_ports::AiClient for FakeAi {
        fn request(&self, prompt: String) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
            Box::pin(async move {
                Ok(ai_ports::AiResponseDTO { text: format!("fake-explain: {}", prompt) })
            })
        }
    }

    #[tokio::test]
    async fn end_to_end_usecase_flow() {
        let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
        let buffer = Arc::new(FakeBufferStore) as Arc<dyn buffer_ports::BufferStore>;
        let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

        let orch = WorkspaceOrchestrator::new(repo, buffer, ai);

        // Boot workspace
        let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
        let boot_res = orch.boot_workspace(boot).await.expect("boot ok");
        assert_eq!(boot_res.session.session_id, "ws-test");

        // Open buffer
        let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
        let open_res = orch.open_buffer(open).await.expect("open ok");
        assert!(open_res.buffer_id.starts_with("buf:"));

        // Dispatch AI explain
        let cmd = DispatchCommandRequest { session_id: boot_res.session.session_id.clone(), command: AppCommand::AiExplain { prompt: format!("Explain {}", open_res.buffer_id) } };
        let cmd_res = orch.dispatch_command(cmd).await.expect("dispatch ok");
        assert!(cmd_res.result.message.contains("fake-explain"));
    }
}
