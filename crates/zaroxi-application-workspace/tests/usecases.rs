use std::sync::Arc;
use std::path::PathBuf;

use zaroxi_application_workspace::usecases::WorkspaceOrchestrator;
use zaroxi_application_workspace::ports::{
    WorkspaceBootRequest, OpenBufferRequest, DispatchCommandRequest, AppCommand,
};
use zaroxi_application_workspace::ports as ports;
use zaroxi_application_workspace::ports::{WorkspaceService, WorkspaceView};
use zaroxi_domain_workspace::ports as domain_ports;
use zaroxi_core_editor_buffer::ports as buffer_ports;
use zaroxi_application_ai::ports as ai_ports;
use zaroxi_kernel_types::Id;

/// Simple integration-style unit test using in-crate test doubles.
#[tokio::test]
async fn orchestrator_flow_happy_path() {
    // Fake repo
    struct FakeRepo;
    impl domain_ports::WorkspaceRepository for FakeRepo {
        fn open_workspace(&self, cmd: domain_ports::WorkspaceOpenCommand) -> ports::BoxFuture<'static, Result<domain_ports::WorkspaceDTO, domain_ports::DomainError>> {
            Box::pin(async move {
                Ok(domain_ports::WorkspaceDTO { id: Id::new(), root_path: cmd.path.clone(), name: "Test".to_string() })
            })
        }
    }

    // Fake buffer store
    struct FakeStore;
    impl buffer_ports::BufferStore for FakeStore {
        fn open_buffer(&self, path: PathBuf) -> ports::BoxFuture<'static, Result<buffer_ports::BufferId, buffer_ports::BufferError>> {
            // Prefer the canonical core helper to construct BufferId from a path.
            let id = buffer_ports::BufferId::from(path);
            Box::pin(async move {
                Ok(id)
            })
        }

        fn get_text(&self, _id: &buffer_ports::BufferId) -> Option<String> {
            Some("fn main() {}".to_string())
        }

        fn set_text(&self, id: &buffer_ports::BufferId, _content: String) -> ports::BoxFuture<'static, Result<(), buffer_ports::BufferError>> {
            let key = id.0.clone();
            Box::pin(async move {
                // Lightweight fake behavior: accept writes for any buffer id that looks like a BufferId produced by open_buffer.
                if key.starts_with("buf:") {
                    Ok(())
                } else {
                    Err(buffer_ports::BufferError("buffer not found".to_string()))
                }
            })
        }
    }

    // Fake AI
    struct FakeAi;
    impl ai_ports::AiClient for FakeAi {
        fn request(&self, req: ai_ports::AiRequest) -> ai_ports::BoxFuture<'static, Result<ai_ports::AiResponseDTO, ai_ports::AiError>> {
            Box::pin(async move {
                Ok(ai_ports::AiResponseDTO { text: format!("fake: {}", req.buffer_id) })
            })
        }
    }

    let repo = Arc::new(FakeRepo) as Arc<dyn domain_ports::WorkspaceRepository>;
    let store = Arc::new(FakeStore) as Arc<dyn buffer_ports::BufferStore>;
    let ai = Arc::new(FakeAi) as Arc<dyn ai_ports::AiClient>;

    let orchestrator = WorkspaceOrchestrator::new(repo, store, ai);

    let boot = WorkspaceBootRequest { path: PathBuf::from("./sample") };
    let boot_res = orchestrator.boot_workspace(boot).await.expect("boot ok");
    // session id is now typed; ensure it's present.
    assert!(boot_res.session.session_id.0.to_string().len() > 0);

    let open = OpenBufferRequest { session_id: boot_res.session.session_id.clone(), path: PathBuf::from("main.rs") };
    let open_res = orchestrator.open_buffer(open).await.expect("open ok");
    // Prefer typed assertion: the BufferId is expected to map to a filesystem path.
    assert!(open_res.buffer_id.path().is_some());

    // Phase 2: verify view seam can read buffer content
    let content = orchestrator.get_buffer_content(open_res.buffer_id.clone()).await.expect("read ok");
    assert!(content.is_some());
    assert!(content.unwrap().contains("fn main"));

    let dispatch = DispatchCommandRequest { session_id: boot_res.session.session_id.clone(), command: AppCommand::AiExplain { buffer_id: open_res.buffer_id.clone() } };
    let dispatch_res = orchestrator.dispatch_command(dispatch).await.expect("dispatch ok");
    assert!(dispatch_res.result.message.contains("fake:"));
}
