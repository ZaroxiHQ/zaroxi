use std::sync::Arc;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, DispatchCommandRequest, AppCommand,
};
use zaroxi_domain_workspace::ports as domain_ports;
use zaroxi_core_editor_buffer::ports as buffer_ports;
use zaroxi_application_ai::ports as ai_ports;

/// Simple integration-style unit test using in-crate test doubles.
#[tokio::test]
async fn orchestrator_flow_happy_path() {
    // Fake repo
    struct FakeRepo;
    impl domain_ports::WorkspaceRepository for FakeRepo {
        fn open_workspace(&self, cmd: domain_ports::WorkspaceOpenCommand) -> crate::ports::BoxFuture<'static, Result<domain_ports::WorkspaceDTO, domain_ports::DomainError>> {
            Box::pin(async move {
                Ok(domain_ports::WorkspaceDTO { id: "ws-test".to_string(), root_path: cmd.path.clone(), name: "Test" .to_string() })
            })
        }
    }

    // Fake buffer store
    struct FakeStore;
    impl buffer_ports::BufferStore for FakeStore {
        fn open_buffer(&self, path: PathBuf) -> crate::ports::BoxFuture<'static, Result<buffer_ports::BufferId, buffer_ports::BufferError>> {
            Box::pin(async move {
                Ok(buffer_ports::BufferId(format!("buf:{}", path.to_string_lossy())))
            })
        }

        fn get_text(&self, _id: &buffer_ports::BufferId) -> Option<String> {
            Some("fn main() {}".to_string())
        }
    }

    // Fake AI
    struct FakeAi;
    impl ai_ports::AiClient for FakeAi {
        fn request(&self, prompt: String) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
            Box::pin(async move {
                Ok(ai_ports::AiResponseDTO { text: format!("fake: {}", prompt) })
            })
        }
    }

    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store, ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");
    assert_eq!(boot_res.session.session_id, "ws-test");

    let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");
    assert!(open_res.buffer_id.starts_with("buf:"));

    let dispatch = DispatchCommandRequest { session_id: boot_res.session.session_id.clone(), command: AppCommand::AiExplain { prompt: format!("Explain {}", open_res.buffer_id) } };
    let dispatch_res = orchestrator.dispatch_command(dispatch).await.expect("dispatch ok");
    assert!(dispatch_res.result.message.contains("fake:"));
}
